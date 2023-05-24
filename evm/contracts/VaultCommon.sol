//SPDX-License-Identifier: Unlicensed

pragma solidity ^0.8.16;

import {ERC20} from 'solmate/src/tokens/ERC20.sol';
import {SafeTransferLib} from 'solmate/src/utils/SafeTransferLib.sol';
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import "@openzeppelin/contracts/utils/Multicall.sol";
import "./CatalystVaultFactory.sol";
import "./FixedPointMathLib.sol";
import "./CatalystIBCInterface.sol";
import "./interfaces/IOnCatalyst.sol";
import "./ICatalystV1Vault.sol";
import "./interfaces/ICatalystV1VaultErrors.sol";

/**
 * @title Catalyst: Common Swap Vault Logic
 * @author Catalyst Labs
 * @notice This abstract contract defines general logic of a Catalyst vault like:
 * - Vault Token through Solmate's ERC20 implementation.
 * - Connection management
 * - Security limit
 * - Swap Escrow
 *
 * By inheriting this abstract contract, a Vault automatically implements common swap vault logic.
 * @dev This contract uses the following special notation:
 * CAPITAL_LETTER_VARIABLES are constants or immutable.
 * _ prefixed varaibles are storage.
 * _ prefixed functions are internal.
 * Unless otherwise required, variables are exposed directly. Such that storage functions are
 * prefixed with _.
 */
abstract contract CatalystVaultCommon is
    Initializable,
    Multicall,
    ReentrancyGuard,
    ERC20,
    ICatalystV1Vault
{
    using SafeTransferLib for ERC20;

    //--- Config ---//
    // The following section contains the configurable variables.

    /// @notice Determines how fast the security limit decreases.
    /// @dev Needs to be long enough for vault token providers to be notified of a breach but short enough for volatility to not soft-freeze the vault.
    uint256 constant DECAY_RATE = 1 days;

    /// @notice Number of decimals used by the vault's vault tokens
    uint8 constant DECIMALS = 18;

    /// @notice The vault tokens initially minted to the user who set up the vault.
    /// @dev The initial deposit along with this value determines the base value of a vault token.
    uint256 constant INITIAL_MINT_AMOUNT = 1e18;  // 10**decimals

    /// @notice Maximum number of assets supported
    /// @dev Impacts the cost of some for loops. Can be changed without breaking compatibility.
    uint8 constant MAX_ASSETS = 3;

    //-- Variables --//

    // immutable variables can be read by proxies, thus it is safe to set this on the constructor.
    address public immutable FACTORY;
    address public _chainInterface;

    /// @notice The approved connections for this vault, stored as _vaultConnection[connectionId][toVault]
    /// @dev to vault is encoded as 64 + 1 bytes.
    mapping(bytes32 => mapping(bytes => bool)) public _vaultConnection;

    /// @notice To indicate which token is desired on the target vault,
    /// the desired tokens are provided as an integer which maps to the
    /// asset address. This variable is the map.
    mapping(uint256 => address) public _tokenIndexing;

    /// @notice The token weights. Used for maintaining a non-symmetric vault balance.
    mapping(address => uint256) public _weight;

    //-- Parameter change variables --//
    uint256 public _adjustmentTarget;
    uint256 public _lastModificationTime;

    //-- Vault fee variables --//
    /// @notice The total vault fee. Multiplied by 10**18. 
    /// @dev Implementation of fee: FixedPointMathLib.mulWadDown(amount, _vaultFee);
    uint256 public _vaultFee;
    /// @notice The governance's cut of _vaultFee. 
    /// @dev FixedPointMathLib.mulWadDown(FixedPointMathLib.mulWadDown(amount, _vaultFee), _governanceFeeShare);
    uint256 public _governanceFeeShare;
    /// @notice The vault fee can be changed. _feeAdministrator is the address allowed to change it
    address public _feeAdministrator; 

    /// @notice The setupMaster is the short-term owner of the vault.
    /// They can connect the vault to vaults on other chains.
    /// @dev !Can extract all of the vault value! Should be set to address(0) once setup is complete.
    address public _setupMaster;

    //--- Messaging router limit ---//
    // The router is not completely trusted. Some limits are
    // imposed on the DECAY_RATE-ly unidirectional liquidity flow. That is:
    // if the vault observes more than _maxUnitCapacity of incoming
    // units, then it will not accept further incoming units. This means the router
    // can only drain a prefigured percentage of the vault every DECAY_RATE
    // For amplified vaults, the security limit is denominated in assets rather than Units.

    // Outgoing flow is subtracted from incoming flow until 0.

    /// @notice The max incoming liquidity flow from the router.
    uint256 public _maxUnitCapacity;
    // -- State related to unit flow calculation -- //
    // Use getUnitCapacity to indirectly access these variables.
    uint256 _usedUnitCapacity;
    uint256 _usedUnitCapacityTimestamp;

    // Escrow reference
    /// @notice Total current escrowed tokens
    mapping(address => uint256) public _escrowedTokens;
    /// @notice Specific escrow information
    mapping(bytes32 => address) public _escrowedTokensFor;

    /// @notice Total current escrowed vault tokens
    uint256 public _escrowedVaultTokens;
    /// @notice Specific escrow information (Liquidity)
    mapping(bytes32 => address) public _escrowedVaultTokensFor;

    constructor(address factory_) ERC20("Catalyst Vault Template", "", DECIMALS) {
        FACTORY = factory_;

        // Disable the contract from being initialized. This ensures the factory is
        // used to deploy vaults.
        _disableInitializers();
    }

    function factoryOwner() public view override returns (address) {
        return CatalystVaultFactory(FACTORY).owner();
    }

    /**
     * @notice Only allow Governance to change vault parameters
     * @dev Because of dangerous permissions (setConnection, weight changes, amplification changes):
     * !CatalystVaultFactory(_factory).owner() must be set to a timelock! 
     */ 
    modifier onlyFactoryOwner() {
        require(msg.sender == factoryOwner());   // dev: Only factory owner
        _;
    }

    /**
     * @notice Checks that an incoming message is coming from the cross-chain interface and that the context of the message is valid. (connection)
     */ 
    modifier verifyIncomingMessage(bytes32 channelId, bytes calldata fromVault) {
        // The chainInterface is the only valid caller of this function.
        require(msg.sender == _chainInterface);
        // Only allow connected vaults
        if (!_vaultConnection[channelId][fromVault]) revert VaultNotConnected(channelId, fromVault);

        _;
    }

    /// @notice Does this vault define a pool without other vaults?
    /// @dev Checked by comparing  _chainInterface to address(0). It is possible that 
    /// no connections have been reacted and this returns false.
    function onlyLocal() public view override returns (bool) {
        return _chainInterface == address(0);
    }

    /** @notice Setup a vault. */
    function setup(
        string calldata name_,
        string calldata symbol_,
        address chainInterface,
        uint256 vaultFee,
        uint256 governanceFee,
        address feeAdministrator,
        address setupMaster
    ) initializer external override {
        // The vault is designed to be used by a proxy and not as a standalone vault.
        // initializer ensures this function is only called once.

        _chainInterface = chainInterface;
        _setupMaster = setupMaster;

        _setVaultFee(vaultFee);
        _setGovernanceFee(governanceFee);
        _setFeeAdministrator(feeAdministrator);

        // Name the ERC20 vault token //
        name = name_;
        symbol = symbol_;
        // END ERC20 //
    }

    /** @notice  Returns the current cross-chain swap capacity. */
    function getUnitCapacity() public view virtual override returns (uint256) {
        uint256 MUC = _maxUnitCapacity;

        // The delta change to the limit is: timePassed · slope = timePassed · Max/decayrate
        uint256 unitCapacityReleased;
        unchecked {
            // block.timestamp >= _usedUnitCapacityTimestamp, always.
            // MUC is generally low.
            unitCapacityReleased = (block.timestamp - _usedUnitCapacityTimestamp);
        }
        unitCapacityReleased *= MUC;
        unchecked {
            // DECAY_RATE != 0.
            unitCapacityReleased /= DECAY_RATE;
        }

        uint256 UC = _usedUnitCapacity;
        // If the change is greater than the units which have passed through
        // return maximum. We do not want (MUC - (UC - unitCapacityReleased) > MUC)
        if (UC <= unitCapacityReleased) return MUC;

        // Amplified vaults can have MUC <= UC since MUC is modified when swapping.
        unchecked {
            // we know that UC > unitCapacityReleased
            if (MUC <= UC - unitCapacityReleased) return 0; 

            // we know UC > unitCapacityReleased 
            // and because of the above if statement, we know
            // MUC > (UC - unitCapacityReleased)
            // Thus we can compute the difference unchecked.
            return MUC - (UC - unitCapacityReleased);
        }

    }

    /**
     * @notice Checks if the vault supports an inflow of units and decreases
     * unit capacity by the inflow.
     * @dev Implement a lot of similar logic to getUnitCapacity. 
     * @param units The number of units to check and set.
     */
    function _updateUnitCapacity(uint256 Units) internal {
        uint256 MUC = _maxUnitCapacity;

        // The delta change to the limit is: timePassed · slope = timePassed · Max/decayrate
        uint256 unitCapacityReleased;
        unchecked {
            // block.timestamp > _usedUnitCapacityTimestamp, always.
            // MUC is generally low.
            unitCapacityReleased = (block.timestamp - _usedUnitCapacityTimestamp);
        }
        unitCapacityReleased *= MUC;
        unchecked {
            // DECAY_RATE != 0.
            unitCapacityReleased /= DECAY_RATE;
        }

        // Set last change to block.timestamp.
        // Otherwise it would have to be repeated twice. (small deployment savings)
        _usedUnitCapacityTimestamp = block.timestamp; 

        uint256 UC = _usedUnitCapacity; 
        // If the change is greater than the units which have passed through the limit is max
        if (UC <= unitCapacityReleased) {
            if (units > MUC) revert ExceedsSecurityLimit(units - MUC);
            _usedUnitCapacity = units;
            return;
        }
        
        uint256 newUnitFlow = UC + units;  // (UC + units) - unitCapacityReleased
        unchecked {
            // We know that UC > unitCapacityReleased
            newUnitFlow -= unitCapacityReleased;
        }
        if (newUnitFlow > MUC) revert ExceedsSecurityLimit(newUnitFlow - MUC);
        _usedUnitCapacity = newUnitFlow;
    }


    /// @notice Sets a new fee administrator that can configure vault fees.
    /// @dev The fee administrator is responsible for modifying vault fees.
    function _setFeeAdministrator(address administrator) internal {
        _feeAdministrator = administrator;
        emit SetFeeAdministrator(administrator);
    }

    /// @notice Sets a new vault fee, taken from input amount.
    function _setVaultFee(uint256 fee) internal {
        require(fee <= 1e18);  // dev: VaultFee is maximum 100%.
        _vaultFee = fee;
        emit SetVaultFee(fee);
    }

    /// @notice Sets a new governance fee. Taken out of the vault fee.
    function _setGovernanceFee(uint256 fee) internal {
        require(fee <= MAX_GOVERNANCE_FEE_SHARE);  // dev: Maximum GovernanceFeeSare exceeded.
        _governanceFeeShare = fee;
        emit SetGovernanceFee(fee);
    }

    /// @notice Allows the factory owner to set a new fee administrator
    function setFeeAdministrator(address administrator) public override onlyFactoryOwner {
        _setFeeAdministrator(administrator);
    }

    /// @notice Allows the factory owner to set a new the governance fee
    function setGovernanceFee(uint256 fee) public override onlyFactoryOwner {
        _setGovernanceFee(fee);
    }

    /// @notice Allows the factory owner to modify the vault fee
    function setVaultFee(uint256 fee) public override {
        require(msg.sender == _feeAdministrator); // dev: Only feeAdministrator can set new fee
        _setVaultFee(fee);
    }

    /// @notice Collect the governance fee share of the specified vault fee
    /// @dev The governance fee share is transfered to the factory owner.
    function _collectGovernanceFee(address asset, uint256 vaultFeeAmount) internal {
        uint256 governanceFeeShare = _governanceFeeShare;

        // If governanceFeeShare == 0, then skip the rest of the logic.
        if (governanceFeeShare != 0) {
            uint256 governanceFeeAmount = FixedPointMathLib.mulWadDown(vaultFeeAmount, governanceFeeShare);
            ERC20(asset).safeTransfer(factoryOwner(), governanceFeeAmount);
        }
    }

    /**
     * @notice Creates a connection to toVault on the channel_channelId.
     * @dev Encoding addresses in 64 + 1 bytes for EVM.
     * For Solidity, this can be done as abi.encodePacket(uint8(20), bytes32(0), abi.encode(toAddress))
     * @param channelId Target chain identifier.
     * @param toVault 64 + 1 bytes representation of the target vault.
     * @param state Boolean indicating if the connection should be open or closed.
     */
    function setConnection(
        bytes32 channelId,
        bytes calldata toVault,
        bool state
    ) external override {
        require((msg.sender == _setupMaster) || (msg.sender == factoryOwner())); // dev: No auth
        require(toVault.length == 65);  // dev: Vault addresses are uint8 + 64 bytes.

        _vaultConnection[channelId][toVault] = state;

        emit SetConnection(channelId, toVault, state);
    }

    /**
     * @notice Gives up short-term ownership of the vault. This makes the vault unstoppable.
     * @dev This function should ALWAYS be called before other liquidity providers deposit liquidity.
     * While it is not recommended, the escrow should ensure it is relativly safe trading through it (assuming a minimum output is set).
     */
    function finishSetup() external override {
        require(msg.sender == _setupMaster); // dev: No auth

        _setupMaster = address(0);

        emit FinishSetup();
    }

    /**
     * @notice View function to signal if a vault is safe to use.
     * @dev Checks if the setup master has been set to ZERO_ADDRESS.
     * In other words, has finishSetup been called?
     */
    function ready() external view override returns (bool) {
        // _setupMaster == address(0) ensures the pool is safe. The setup master can drain the pool!
        // _tokenIndexing[0] != address(0) check if the pool has been initialized correctly.
        // The additional check is there to ensure that the initial deployment returns false. 
        return _setupMaster == address(0) && _tokenIndexing[0] != address(0);
    }


    //-- Escrow Functions --//

    /// @notice Returns the fallbackUser for the escrow and cleans up the escrow information.
    /// @dev 'delete _escrowedTokensFor[sendAssetHash]' ensures this function can only be called once.
    function _releaseAssetEscrow(
        bytes32 sendAssetHash,
        uint256 escrowAmount,
        address escrowToken
    ) internal returns(address) {

        address fallbackUser = _escrowedTokensFor[sendAssetHash];  // Passing in an invalid swapHash returns address(0)
        require(fallbackUser != address(0));  // dev: Invalid swapHash. Alt: Escrow doesn't exist.
        delete _escrowedTokensFor[sendAssetHash];  // Stops timeout and further acks from being called

        unchecked {
            // escrowAmount \subseteq _escrowedTokens => escrowAmount <= _escrowedTokens. Cannot be called twice since the 3 lines before ensure this can only be reached once.
            _escrowedTokens[escrowToken] -= escrowAmount;
        }
        
        return fallbackUser;
    }

    /// @notice Returns the fallbackUser for the escrow and cleans up the escrow information.
    /// @dev 'delete _escrowedTokensFor[sendAssetHash]' ensures this function can only be called once.
    function _releaseLiquidityEscrow(
        bytes32 sendLiquidityHash,
        uint256 escrowAmount
    ) internal returns(address) {

        address fallbackUser = _escrowedVaultTokensFor[sendLiquidityHash];  // Passing in an invalid swapHash returns address(0)
        require(fallbackUser != address(0));  // dev: Invalid swapHash. Alt: Escrow doesn't exist.
        delete _escrowedVaultTokensFor[sendLiquidityHash];  // Stops timeout and further acks from being called

        unchecked {
            // escrowAmount \subseteq _escrowedVaultTokens => escrowAmount <= _escrowedVaultTokens. Cannot be called twice since the 3 lines before ensure this can only be reached once.
            _escrowedVaultTokens -= escrowAmount;
        }
        
        return fallbackUser;
    }

    /** 
     * @notice Implements basic ack logic: Deletes and releases tokens to the vault
     * @dev Should never revert! For security limit adjustments, the implementation should be overwritten.
     * @param toAccount The recipient of the transaction on the target chain. Encoded in bytes32.
     * @param U The number of units initially purchased.
     * @param escrowAmount The number of tokens escrowed.
     * @param escrowToken The token escrowed.
     * @param blockNumberMod The block number at which the swap transaction was commited (mod 32)
     */
    function onSendAssetSuccess(
        bytes32 channelId,
        bytes calldata toAccount,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken,
        uint32 blockNumberMod
    ) nonReentrant public override virtual {
        require(msg.sender == _chainInterface);  // dev: Only _chainInterface

        // We need to find the location of the escrow with using the below information.
        // We need to do this twice: 1. Get the address. 2. Delete the escrow.
        // To save a bit of gas, this hash is computed and saved and then used.
        bytes32 sendAssetHash = _computeSendAssetHash( // Computing the hash doesn't revert.
            toAccount,      // Ensures no collisions between different users
            U,              // Used to randomise the hash
            escrowAmount,   // Required! to validate release escrow data
            escrowToken,    // Required! to validate release escrow data
            blockNumberMod  // Used to randomize the hash.
        );

        _releaseAssetEscrow(sendAssetHash, escrowAmount, escrowToken); // Only reverts for missing escrow

        emit SendAssetSuccess( // Never reverts.
            channelId,
            toAccount,
            U,
            escrowAmount,
            escrowToken,
            blockNumberMod
        );
    }

    /** 
     * @notice Implements basic timeout logic: Deletes and sends tokens to the user.
     * @dev Should never revert!
     * @param toAccount The recipient of the transaction on the target chain. Encoded in bytes32.
     * @param U The number of units initially purchased.
     * @param escrowAmount The number of tokens escrowed.
     * @param escrowToken The token escrowed.
     * @param blockNumberMod The block number at which the swap transaction was commited (mod 32)
     */
    function onSendAssetFailure(
        bytes32 channelId,
        bytes calldata toAccount,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken,
        uint32 blockNumberMod
    ) nonReentrant public override virtual {
        require(msg.sender == _chainInterface);  // dev: Only _chainInterface

        // We need to find the location of the escrow with using the below information.
        // We need to do this twice: 1. Get the address. 2. Delete the escrow.
        // To save a bit of gas, this hash is computed and saved and then used.
        bytes32 sendAssetHash = _computeSendAssetHash( // Computing the hash doesn't revert.
            toAccount,      // Ensures no collisions between different users
            U,              // Used to randomise the hash
            escrowAmount,   // Required! to validate release escrow data
            escrowToken,    // Required! to validate release escrow data
            blockNumberMod  // Used to randomize the hash.
        );

        address fallbackAddress = _releaseAssetEscrow(sendAssetHash, escrowAmount, escrowToken); // Only reverts for missing escrow,

        ERC20(escrowToken).safeTransfer(fallbackAddress, escrowAmount);  // Would fail if there is no balance. To protect against this, the escrow amount should be removed from what can be claimed by users.

        emit SendAssetFailure( // Never reverts.
            channelId,
            toAccount,
            U,
            escrowAmount,
            escrowToken,
            blockNumberMod
        );
    }

    /** 
     * @notice Implements basic liquidity ack logic: Deletes and releases vault tokens to the vault.
     * @dev Should never revert! For security limit adjustments, the implementation should be overwritten.
     * @param toAccount The recipient of the transaction on the target chain. Encoded in bytes32.
     * @param U The number of units initially acquired.
     * @param escrowAmount The number of vault tokens escrowed.
     * @param blockNumberMod The block number at which the swap transaction was commited (mod 32)
     */
    function onSendLiquiditySuccess(
        bytes32 channelId,
        bytes calldata toAccount,
        uint256 U,
        uint256 escrowAmount,
        uint32 blockNumberMod
    ) nonReentrant public override virtual {
        require(msg.sender == _chainInterface);  // dev: Only _chainInterface

        // We need to find the location of the escrow with using the below information.
        // We need to do this twice: 1. Get the address. 2. Delete the escrow.
        // To save a bit of gas, this hash is computed and saved and then used.
        bytes32 sendLiquidityHash = _computeSendLiquidityHash( // Computing the hash doesn't revert.
            toAccount,      // Ensures no collisions between different users
            U,              // Used to randomise the hash
            escrowAmount,   // Required! to validate release escrow data
            blockNumberMod  // Used to randomize the hash.
        );

        _releaseLiquidityEscrow(sendLiquidityHash, escrowAmount); // Only reverts for missing escrow

        emit SendLiquiditySuccess( // Never reverts.
            channelId,
            toAccount,
            U,
            escrowAmount,
            blockNumberMod
        );
    }

    /** 
     * @notice Implements basic liquidity timeout logic: Deletes and sends vault tokens to the user.
     * @dev Should never revert!
     * @param toAccount The recipient of the transaction on the target chain. Encoded in bytes32.
     * @param U The number of units initially acquired.
     * @param escrowAmount The number of vault tokens escrowed.
     * @param blockNumberMod The block number at which the swap transaction was commited (mod 32)
     */
    function onSendLiquidityFailure(
        bytes32 channelId,
        bytes calldata toAccount,
        uint256 U,
        uint256 escrowAmount,
        uint32 blockNumberMod
    ) nonReentrant public override virtual {
        require(msg.sender == _chainInterface);  // dev: Only _chainInterface

        bytes32 sendLiquidityHash = _computeSendLiquidityHash( // Computing the hash doesn't revert.
            toAccount,      // Ensures no collisions between different users
            U,              // Used to randomise the hash
            escrowAmount,   // Required! to validate release escrow data
            blockNumberMod  // Used to randomize the hash.
        );

        address fallbackAddress = _releaseLiquidityEscrow(sendLiquidityHash, escrowAmount); // Only reverts for missing escrow

        _mint(fallbackAddress, escrowAmount); // Never reverts.

        emit SendLiquidityFailure( // Never reverts.
            channelId,
            toAccount,
            U,
            escrowAmount,
            blockNumberMod
        );
    }

    /// @notice Computes a unique identifier for a swap. This unique identifier can be used to identify swaps cross-chain.
    /// However, it is never exposed. This is done to let the hashing algorithm be flexible between implementations.
    function _computeSendAssetHash(
        bytes memory toAccount,
        uint256 U,
        uint256 amount,
        address fromAsset,
        uint32 blockNumberMod
    ) internal pure returns(bytes32) {
        return keccak256(
            abi.encodePacked(
                toAccount,      // Ensures no collisions between different users
                U,              // Used to randomise the hash
                amount,         // Required! to validate release escrow data
                fromAsset,      // Required! to validate release escrow data
                blockNumberMod  // Used to randomize the hash.
            )
        );
    }

    /// @notice Computes a unique identifier for a swap. This unique identifier can be used to identify swaps cross-chain.
    /// However, it is never exposed. This is done to let the hashing algorithm be flexible between implementations.
    function _computeSendLiquidityHash(
        bytes calldata toAccount,
        uint256 U,
        uint256 amount,
        uint32 blockNumberMod
    ) internal pure returns(bytes32) {
        return keccak256(
            abi.encodePacked(
                toAccount,      // Ensures no collisions between different users
                U,              // Used to randomise the hash
                amount,         // Required! to validate release escrow data
                blockNumberMod  // Used to randomize the hash.
            )
        );
    }

}
