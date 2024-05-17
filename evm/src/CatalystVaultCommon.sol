//SPDX-License-Identifier: BUSL-1.1
pragma solidity ^0.8.19;

import { Ownable } from "solady/auth/Ownable.sol";
import {ERC20} from 'solady/tokens/ERC20.sol';
import {SafeTransferLib} from 'solady/utils/SafeTransferLib.sol';
import { ReentrancyGuard} from "solady/utils/ReentrancyGuard.sol";
import { Initializable } from "solady/utils/Initializable.sol";
import { Multicallable } from "solady/utils/Multicallable.sol";
import { FixedPointMathLib} from "solady/utils/FixedPointMathLib.sol";

import { IMessageEscrowStructs } from "GeneralisedIncentives/src/interfaces/IMessageEscrowStructs.sol";

import { ICatalystReceiver} from "./interfaces/IOnCatalyst.sol";
import { ICatalystV1Factory } from "./interfaces/ICatalystV1Factory.sol";
import "./interfaces/ICatalystV1VaultErrors.sol";
import { MAX_GOVERNANCE_FEE_SHARE } from"./CatalystFactory.sol";
import { ICatalystV1Vault } from "./ICatalystV1Vault.sol";

/**
 * @title Catalyst: Common Vault Logic
 * @author Cata Labs Inc.
 * @notice This abstract contract defines general logic of a Catalyst vault like:
 * - Vault Token through Solmate's ERC20 implementation.
 * - Connection management
 * - Security limit
 * - Swap Escrow
 *
 * By inheriting this abstract contract, a Vault automatically implements common vault logic.
 * @dev This contract uses the following special notation:
 * CAPITAL_LETTER_VARIABLES are constants or immutable.
 * _ prefixed varaibles are storage.
 * _ prefixed functions are internal.
 * Unless otherwise required, variables are exposed directly. Such that storage functions are
 * prefixed with _.
 *
 * Upon deleting the escrow, this contract special logic in case of refunds. We want to ensure that
 * any acks does not revert to clear up the escrows. However, some tokens can revert on demand (blacklist tokens)
 * For these tokens, we make an optimistic call to the token to send the assets to the user. We don't
 * care if it actually goes through. Cases where this call would fail the user does not get anything.
 * A caveat of the implementation is that tokens that revert by PANIC'ing or spending all gas, are not supported
 * since there is a catch of OOO gas that reverts. It is then dependent on replaying the ack to release the escrow.
 */
abstract contract CatalystVaultCommon is
    Initializable,
    Multicallable,
    ReentrancyGuard,
    ERC20,
    ICatalystV1Vault
{
    /** @notice The fallback user used as a FALLBACK placeholder when underwrite escrows are set. */
    address constant UNDERWRITE_FALLBACK_USER = address(uint160(1));

    //--- Config ---//
    // The following section contains the configurable variables.

    /**
     * @notice Determines how fast the security limit decreases.
     * @dev Needs to be long enough for vault token providers to be notified of a breach but short enough for volatility to not soft-freeze the vault.
     */
    uint256 constant DECAY_RATE = 1 days;

    /** @notice Number of decimals used by the vault's vault tokens */
    uint8 constant DECIMALS = 18;

    /**
     * @notice The vault tokens initially minted to the user who set up the vault.
     * @dev The initial deposit along with this value determines the base value of a vault token.
     */
    uint256 constant INITIAL_MINT_AMOUNT = 1e18; // 10**decimals

    /**
     * @notice Maximum number of assets supported
     * @dev Impacts the cost of some for loops. Can be changed without breaking compatibility.
     */
    uint8 constant MAX_ASSETS = 3;

    //-- ERC20 --//

    string _name;
    string _symbol;

    function name() public override view returns(string memory) {
        return _name;
    }

    function symbol() public override view returns(string memory) {
        return _symbol;
    }

    //-- Variables --//

    // immutable variables can be read by proxies, thus it is safe to set this on the constructor.
    address public immutable FACTORY;
    address public _chainInterface;

    /**
     * @notice The approved connections for this vault, stored as _vaultConnection[connectionId][toVault]
     * @dev to vault is encoded as 64 + 1 bytes.
     */
    mapping(bytes32 => mapping(bytes => bool)) public _vaultConnection;

    /**
     * @notice To indicate which token is desired on the target vault,
     * the desired tokens are provided as an integer which maps to the
     * asset address. This variable is the map.
     */
    mapping(uint256 => address) public _tokenIndexing;

    /** @notice The token weights. Used for maintaining a non-symmetric vault asset balance. */
    mapping(address => uint256) public _weight;

    //-- Parameter Flow & Change variables --//

    // Use getUnitCapacity to indirectly access these variables
    uint256 _usedUnitCapacity;
    uint48 public _adjustmentTarget;

    uint48 public _lastModificationTime;
    uint48 _usedUnitCapacityTimestamp;

    //-- Vault fee variables --//

    /**
     * @notice The total vault fee. Multiplied by 10**18. 
     * @dev To compute the respective fee, use mulWad: FixedPointMathLib.mulWad(amount, _vaultFee);
     */
    uint64 public _vaultFee;

    /**
     * @notice The governance's cut of _vaultFee. 
     * @dev Usage: FixedPointMathLib.mulWad(FixedPointMathLib.mulWad(amount, _vaultFee), _governanceFeeShare);
     */
    uint64 public _governanceFeeShare;

    /** @notice The vault fee can be changed. _feeAdministrator is the address allowed to change it */
    address public _feeAdministrator; 

    /**
     * @notice The setupMaster is the short-term owner of the vault.
     * They can connect the vault to vaults on other chains.
     * @dev !Can extract all of the vault value! Should be set to address(0) once setup is complete via 'finishSetup()'.
     */
    address public _setupMaster;

    //--- Messaging router limit ---//
    // The router is not completely trusted. Some limits are
    // imposed on the DECAY_RATE-ly unidirectional liquidity flow. That is:
    // if the vault observes more than _maxUnitCapacity of incoming
    // units, then it will not accept further incoming units. This means the router
    // can only drain a prefigured percentage of the vault every DECAY_RATE.
    // For amplified vaults, the security limit is denominated in assets rather than Units.

    // Outgoing flow is subtracted from incoming flow until 0.

    /** @notice The max incoming liquidity flow from the router. */
    uint256 public _maxUnitCapacity;

    // Escrow reference
    /** @notice Total current escrowed tokens. */
    mapping(address => uint256) public _escrowedTokens;
    /** @notice Total current escrowed vault tokens. */
    uint256 public _escrowedVaultTokens;

    /** @notice Find escrow information. Used for both normal swaps and liquidity swaps. */
    mapping(bytes32 => address) public _escrowLookup;

    /**
     * @notice A mathematical lib that describes various properties of this contract.
     * These helper functions are not contained in the swap template, since they notisably inflate
     * the contract side which reduceses the number of optimizer runs => increase the gas cost.
     */
    address immutable public MATHLIB;

    constructor(address factory_, address mathlib) payable {
        FACTORY = factory_;
        MATHLIB = mathlib;

        _name = "Catalyst Vault Template";
        _symbol = "";

        // Disable the contract from being initialized. This ensures the factory is
        // used to deploy vaults.
        _disableInitializers();
    }

    /** @notice Get the factory owner. That is the owner that can configure this vault post finishSetup(). */
    function factoryOwner() public view override returns (address) {
        return Ownable(FACTORY).owner();
    }

    /** @notice Governance fee destination. This is the address the governance fee is sent to. */
    function governanceFeeDestination() public view override returns (address) {
        return ICatalystV1Factory(FACTORY)._governanceFeeDestination();
    }

    /**
     * @notice Only allow Governance to change vault parameters
     * @dev Because of dangerous permissions (setConnection, weight changes):
     * !CatalystFactory(_factory).owner() must be set to a timelock! 
     */ 
    modifier onlyFactoryOwner() {
        require(msg.sender == factoryOwner()); // dev: Only factory owner
        _;
    }

    /**
     * @notice Require the sender of the transaction to be the chain interface. 
     */
     modifier onlyChainInterface() {
        require(msg.sender == _chainInterface); // dev: Only chain interface
        _;
    }

    /**
     * @notice Verify a connected pool.
     */ 
    modifier onlyConnectedPool(bytes32 channelId, bytes memory vault) {
        // Only allow connected vaults
        if (!_vaultConnection[channelId][vault]) revert VaultNotConnected();
        _;
    }

    // -- Receive Abstract Functions -- //

    function _receiveAsset(
        address toAsset,
        uint256 U,
        uint256 minOut
    ) virtual internal returns (uint256);

    // -- Setup Functions -- //

    /**
     * @notice Setup a vault.
     * @dev Is initializer.
     * @param name_ Name of the vault token.
     * @param symbol_ Symbol of the vault token.
     * @param chainInterface The chain interface to use. Set address(0) to disable cross-chain swaps.
     * @param vaultFee Initial vault fee, that is the fee charged on swaps.
     * @param governanceFee Initial governance fee, that is the percentage of the vault fee that is sent to designated address.
     * @param feeAdministrator Special address that can modify the pool fees.
     * @param setupMaster User that is configuring the vault.
     */
    function setup(
        string calldata name_,
        string calldata symbol_,
        address chainInterface,
        uint64 vaultFee,
        uint64 governanceFee,
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

        // Name the ERC20 vault token
        _name = name_;
        _symbol = symbol_;
    }


    /**
     * @notice Creates a connection to toVault on the channelId.
     * @dev Encoding addresses in 64 + 1 bytes for EVM.
     * For Solidity, this can be done as bytes.concat(bytes1(0x14), bytes32(0), abi.encode(toAddress))
     * @param channelId Target chain identifier. Varies from AMB to AMB.
     * @param toVault 64 + 1 bytes representation of the target vault.
     * @param state Boolean indicating if the connection should be open or closed.
     */
    function setConnection(
        bytes32 channelId,
        bytes calldata toVault,
        bool state
    ) external override {
        require(msg.sender == _setupMaster); // dev: No auth
        require(toVault.length == 65); // dev: Vault addresses are uint8 + 64 bytes.

        _vaultConnection[channelId][toVault] = state;

        emit SetConnection(channelId, toVault, state);
    }

    /**
     * @notice Gives up short-term ownership of the vault. This makes the vault unstoppable.
     * @dev This function should ALWAYS be called before other liquidity providers deposit liquidity.
     * While it is not recommended, swapping should be relativly safe since because of the escrow (assuming a minimum output is set).
     */
    function finishSetup() external override {
        require(msg.sender == _setupMaster); // dev: No auth

        _setupMaster = address(0);

        emit FinishSetup();
    }

    /**
     * @notice View function to signal if a vault is safe to use.
     * @dev Checks if setupMaster has been set to ZERO_ADDRESS.  In other words, has finishSetup been called?
     * This function is not "safe()". To properly verify if a pool is "ready", verify:
     *  - The vault template is trusted.
     *  - The cross-chain interfce is trusted.
     *  - All connections are correctly set. (Valid chains, valid vaults)
     *
     * If you are providing liquidity to a vault, furthermore check:
     *  - All assets in the pool are trusted.
     *
     * The above checks have to be done on every vault in the pool.
     */
    function ready() external view override returns (bool) {
        // _setupMaster == address(0) ensures a safe pool cannot be made unsafe. The setup master can drain the pool!
        // _tokenIndexing[0] != address(0) check if the pool has been initialized correctly.
        // The additional check is there to ensure that the initial deployment returns false. 
        return _setupMaster == address(0) && _tokenIndexing[0] != address(0);
    }


    /**
     * @notice Returns the current cross-chain swap capacity.
     * @dev can be overridden to implement other limit curves.
     */
    function getUnitCapacity() public view virtual override returns (uint256) {
        uint256 MUC = _maxUnitCapacity;

        // The delta change to the limit is: timePassed · slope = timePassed · Max/decayrate
        uint256 unitCapacityReleased;
        unchecked {
            // block.timestamp >= _usedUnitCapacityTimestamp, always.
            // MUC is generally low.
            unitCapacityReleased = block.timestamp - _usedUnitCapacityTimestamp;

            // This line can overflow. While unlikely to, if it does it would semi-freeze the pool until
            // some assets are withdrawn. As a fix, let if overflow. If it overflows, then 
            // unitCapacityReleased becomes smaller so there are no security implications.
            unitCapacityReleased *= MUC;

            // DECAY_RATE != 0.
            unitCapacityReleased /= DECAY_RATE;
        }

        uint256 UC = _usedUnitCapacity;
        // If the change is greater than the units than the spent security limit return maximum.
        //If we computed it as (MUC - UC + unitCapacityReleased > MUC) it would obviously be wrong.
        if (UC <= unitCapacityReleased) return MUC;

        // Amplified vaults can have MUC <= UC since MUC is modified when swapping.
        unchecked {
            // We know UC > unitCapacityReleased.
            if (MUC <= UC - unitCapacityReleased) return 0; 

            // We know UC > unitCapacityReleased and with the above statement we know
            // MUC > (UC - unitCapacityReleased). Thus we can compute the difference unchecked.
            return MUC - (UC - unitCapacityReleased);
        }

    }

    // -- Utils -- // 

    /**
     * @notice Check if the vault supports an inflow of units and decrease
     * unit capacity by the inflow.
     * @dev Implement a lot of similar logic to getUnitCapacity. 
     * @param Units The number of units to check and set.
     */
    function _updateUnitCapacity(uint256 Units) internal {
        uint256 MUC = _maxUnitCapacity;

        // The delta change to the limit is: timePassed · slope = timePassed · Max/decayrate
        uint256 unitCapacityReleased;
        unchecked {
            // block.timestamp > _usedUnitCapacityTimestamp, always.
            // MUC is generally low.
            unitCapacityReleased = (block.timestamp - _usedUnitCapacityTimestamp);

            // This line can overflow. While unlikely to, if it does it would semi-freeze the pool until
            // some assets are withdrawn. As a fix, let if overflow. If it overflows, then 
            // unitCapacityReleased becomes smaller so there are no security implications.
            unitCapacityReleased *= MUC;

            // DECAY_RATE != 0.
            unitCapacityReleased /= DECAY_RATE;
        }

        uint256 UC = _usedUnitCapacity; 
        // If the change is greater than the units than the spent security limit it is at max.
        if (UC <= unitCapacityReleased) {
            // The new limit is MUC, check if more units than MUC are getting spent.
            if (Units > MUC) revert ExceedsSecurityLimit();
            // Set last change and the new spent security limit.
            _usedUnitCapacityTimestamp = uint48(block.timestamp);
            _usedUnitCapacity = Units;
            return;
        }
        
        // Compute the new spent security limit. Start by adding used unit capacity
        // and to be spent units
        uint256 newUnitFlow = UC + Units;
        unchecked {
            // Then subtract the units that have been decayed. We know UC + Units >= UC > unitCapacityReleased
            newUnitFlow -= unitCapacityReleased;
        }
        // Then check if the new spent security limit is larger than maximum. 
        if (newUnitFlow > MUC) revert ExceedsSecurityLimit();

        // Set last change and the new spent security limit.
        _usedUnitCapacityTimestamp = uint48(block.timestamp);
        _usedUnitCapacity = newUnitFlow;
    }


    // -- Governance Functions -- //

    /** @notice Sets a new fee administrator that can configure vault fees. */ 
    function _setFeeAdministrator(address administrator) internal {
        _feeAdministrator = administrator;
        emit SetFeeAdministrator(administrator);
    }

    /**
     * @notice Sets a new vault fee, taken from input amount.
     * @param fee Fee in WAD terms. 12e16 is 12%.
     */
    function _setVaultFee(uint64 fee) internal {
        require(fee <= FixedPointMathLib.WAD); // dev: VaultFee is maximum 100%.
        _vaultFee = fee;
        emit SetVaultFee(fee);
    }

    /**
     * @notice Sets a new governance fee. Taken of the vault fee.
     * @param fee Fee in WAD terms. 12e16 is 12%.
     */
    function _setGovernanceFee(uint64 fee) internal {
        require(fee <= MAX_GOVERNANCE_FEE_SHARE);  // dev: Maximum GovernanceFeeSare exceeded.
        _governanceFeeShare = fee;
        emit SetGovernanceFee(fee);
    }

    /** @notice Allows the factory owner to set a new fee administrator. */
    function setFeeAdministrator(address administrator) public override onlyFactoryOwner {
        _setFeeAdministrator(administrator);
    }

    /** 
     * @notice Allows the factory owner to set the governance fee.
     * @dev Can only be called by factory owner.
     * @param fee Fee in WAD terms. 12e16 is 12%.
     */
    function setGovernanceFee(uint64 fee) public override onlyFactoryOwner {
        _setGovernanceFee(fee);
    }

    /**
     * @notice Allows the feeAdministrator to modify the vault fee.
     * @dev Can only be called by feeAdministrator
     * @param fee Fee in WAD terms. 12e16 is 12%.
     */
    function setVaultFee(uint64 fee) public override {
        require(msg.sender == _feeAdministrator); // dev: Only feeAdministrator can set new fee
        _setVaultFee(fee);
    }

    /**
     * @notice Collect the governance fee share of the specified vault fee.
     * @dev The governance fee share is transfered to governanceFeeDestination.
     */
    function _collectGovernanceFee(address asset, uint256 vaultFeeAmount) internal {
        uint256 governanceFeeShare = _governanceFeeShare;

        // If governanceFeeShare == 0, then skip the rest of the logic.
        if (governanceFeeShare != 0) {
            uint256 governanceFeeAmount = FixedPointMathLib.mulWad(vaultFeeAmount, governanceFeeShare);
            SafeTransferLib.safeTransfer(asset, governanceFeeDestination(), governanceFeeAmount);
        }
    }

    //-- Escrow Functions --//

    /**
     * @notice Create a token escrow for a swap.
     * @dev It is not checked if fallbackUser is set to address(0) but if it is, the escrow is lost.
     * @param sendAssetHash The escrow context hash. Will be used to recover the escrow.
     * From a implementation / usage perspective, if this hash contains fromAsset and amount
     * it will improves by allowing on to verify these values independently.
     * @param fallbackUser The user who the escrow belongs to. Do not set to address(0).
     * @param fromAsset Asset to escrow.
     * @param amount Amount to escrow.
     */
    function _setTokenEscrow(
        bytes32 sendAssetHash,
        address fallbackUser,
        address fromAsset,
        uint256 amount
    ) internal {
        if (_escrowLookup[sendAssetHash] != address(0))  revert EscrowAlreadyExists();
        _escrowLookup[sendAssetHash] = fallbackUser;
        unchecked {
            // Must be less than the vault balance.
            _escrowedTokens[fromAsset] += amount;
        }
    }

    /**
     * @notice Create a liquidity escrow for a swap.
     * @dev It is not checked if fallbackUser is set to address(0) but if it is, the escrow is lost.
     * @param sendLiquidityHash The escrow context hash. Will be used to recover the escrow.
     * From a implementation / usage perspective, if this hash contains vaultTokens
     * it will improves by allowing on to verify these values independently.
     * @param fallbackUser The user who the escrow belongs to. Do not set to address(0).
     * @param vaultTokens Number of vault tokens to escrow.
     */
    function _setLiquidityEscrow(
        bytes32 sendLiquidityHash,
        address fallbackUser,
        uint256 vaultTokens
    ) internal {
        if (_escrowLookup[sendLiquidityHash] != address(0)) revert EscrowAlreadyExists();
        _escrowLookup[sendLiquidityHash] = fallbackUser;

        // Escrow vault tokens are first burned and then escrowed. As a result, this may overflow unlike _escrowedTokens.
        _escrowedVaultTokens += vaultTokens;
    }

    /**
     * @notice Returns the fallbackUser for the escrow and cleans up the escrow information.
     * @dev 'delete _escrowLookup[sendAssetHash]' ensures this function can only be called once.
     */
    function _releaseAssetEscrow(
        bytes32 sendAssetHash,
        uint256 escrowAmount,
        address escrowToken
    ) internal returns(address) {
        address fallbackUser = _escrowLookup[sendAssetHash]; // Passing in an invalid swapHash returns address(0).
        require(fallbackUser != address(0)); // dev: Invalid swapHash. Alt: Escrow doesn't exist.
        delete _escrowLookup[sendAssetHash]; // Stops timeout and further acks from being called.

        unchecked {
            // escrowAmount \subseteq _escrowedTokens => escrowAmount <= _escrowedTokens. 
            // Cannot be called twice since the 3 lines before ensure this can only be reached once.
            _escrowedTokens[escrowToken] -= escrowAmount;
        }
        
        return fallbackUser;
    }

    /**
     * @notice Returns the fallbackUser for the escrow and cleans up the escrow information.
     * @dev 'delete _escrowLookup[sendAssetHash]' ensures this function can only be called once.
     */
    function _releaseLiquidityEscrow(
        bytes32 sendLiquidityHash,
        uint256 escrowAmount
    ) internal returns(address) {
        address fallbackUser = _escrowLookup[sendLiquidityHash]; // Passing in an invalid swapHash returns address(0).
        require(fallbackUser != address(0)); // dev: Invalid swapHash. Alt: Escrow doesn't exist.
        delete _escrowLookup[sendLiquidityHash]; // Stops timeout and further acks from being called.

        unchecked {
            // escrowAmount \subseteq _escrowedVaultTokens => escrowAmount <= _escrowedVaultTokens.
            // Cannot be called twice since the 3 lines before ensure this can only be reached once.
            _escrowedVaultTokens -= escrowAmount;
        }

        return fallbackUser;
    }

    /** 
     * @notice Implements basic ack logic: Deletes and releases tokens to the vault
     * @dev Should never revert! For security limit adjustments, the implementation may have to be overwritten.
     * @param toAccount Recipient of the transaction on the target chain.
     * @param U Number of units initially purchased.
     * @param escrowAmount Number of tokens escrowed.
     * @param escrowToken Address of the escrowed token.
     * @param blockNumberMod Block number when the transaction was commited (mod 32)
     */
    function onSendAssetSuccess(
        bytes32 channelId,
        bytes calldata toAccount,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken,
        uint32 blockNumberMod
    ) onlyChainInterface public override virtual {
        // We need to find the location of the escrow using the information below.
        // We need to do this twice: 1. Get the address. 2. Delete the escrow.
        // To save a bit of gas, this hash is computed, cached, and then used.
        bytes32 sendAssetHash = _computeSendAssetHash( // Computing the hash doesn't revert.
            toAccount,      // Ensures no collisions between different users
            U,              // Used to randomise the hash
            escrowAmount,   // Required! to validate release escrow data
            escrowToken,    // Required! to validate release escrow data
            blockNumberMod  // Used to randomize the hash.
        );

        _releaseAssetEscrow(sendAssetHash, escrowAmount, escrowToken); // Only reverts for missing escrow

        emit SendAssetSuccess(
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
     * For blacklist tokens, this function contains custom logic to support failing ERC20 transfer.
     * If an ERC20 transfer fails (say because of a blocklist), we only revert if it was because of OOO.
     * This implies that if an ERC20 transfer fails, then the escrow is lost. This shouldn't be the cast
     * except if a token is paused, blacklisted, or the vault is exploited such that it has less assets
     * than the escrow.
     * @param toAccount Recipient of the transaction on the target chain. Encoded in bytes32.
     * @param U Number of units initially purchased.
     * @param escrowAmount Number of tokens escrowed.
     * @param escrowToken Token escrowed.
     * @param blockNumberMod Block number of the transaction that commited the swap (mod 32)
     */
    function onSendAssetFailure(
        bytes32 channelId,
        bytes calldata toAccount,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken,
        uint32 blockNumberMod
    ) onlyChainInterface public override virtual {
        // We need to find the location of the escrow using the information below.
        // We need to do this twice: 1. Get the address. 2. Delete the escrow.
        // To save a bit of gas, this hash is computed and saved and then used.
        bytes32 sendAssetHash = _computeSendAssetHash( // Computing the hash doesn't revert.
            toAccount,      // Ensures no collisions between different users
            U,              // Used to randomise the hash
            escrowAmount,   // Required! to validate release escrow data
            escrowToken,    // Required! to validate release escrow data
            blockNumberMod  // Used to randomize the hash.
        );

        // This call provides re-entry protection against re-entering this call. Otherwise, this call can always be called.
        address fallbackAddress = _releaseAssetEscrow(sendAssetHash, escrowAmount, escrowToken); // Only reverts for missing escrow,

        // We are going to make a low-level call. It may revert (see comment below) but it should not revert if it runs out of gas (that should be raised).
        // As such, get the current gas in the contract.
        uint256 gasLeftBeforeCall = gasleft();

        bool success;
        // Make a low level call such that the transfer never fails. This is important for tokens using block lists.
        // This also implies that if you get blacklisted between when you initiated the swap and the swap failed, you would lose the tokens.
        bytes memory payload = abi.encodeWithSignature("transfer(address,uint256)", fallbackAddress, escrowAmount);
        assembly ("memory-safe") {
            // The gas limit is set to 0x8000000000000000000000000000000000000000000000000000000000000000.
            // This is essentially all gas since the actual gas forwarded is min(gasForwarded, gasleft * 63/64).
            success := call(0x8000000000000000000000000000000000000000000000000000000000000000, escrowToken, 0,  add(payload, 0x20), mload(payload), 0, 0)
            // SafeTransferLib.safeTransferFrom(escrowToken, fallbackAddress, escrowAmount);
        }

        // If the call failed, check if it failed with OOO (If the call used all of the gas it has available).
        if (!success) if (gasleft() < gasLeftBeforeCall * 1 / 63) revert NotEnoughGas();

        emit SendAssetFailure(
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
     * @param toAccount Recipient of the transaction on the target chain.
     * @param U Number of units initially acquired.
     * @param escrowAmount Number of vault tokens escrowed.
     * @param blockNumberMod Block number at which the swap transaction was commited (mod 32)
     */
    function onSendLiquiditySuccess(
        bytes32 channelId,
        bytes calldata toAccount,
        uint256 U,
        uint256 escrowAmount,
        uint32 blockNumberMod
    ) onlyChainInterface public override virtual {

        // We need to find the location of the escrow using the information below.
        // We need to do this twice: 1. Get the address. 2. Delete the escrow.
        // To save a bit of gas, this hash is computed and saved and then used.
        bytes32 sendLiquidityHash = _computeSendLiquidityHash(
            toAccount,      // Ensures no collisions between different users
            U,              // Used to randomise the hash
            escrowAmount,   // Required! to validate release escrow data
            blockNumberMod  // Used to randomize the hash.
        );

        _releaseLiquidityEscrow(sendLiquidityHash, escrowAmount); // Reverts for missing escrow

        emit SendLiquiditySuccess(
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
     * @param toAccount Recipient of the transaction on the target chain. Encoded in bytes32.
     * @param U Number of units initially acquired.
     * @param escrowAmount Number of vault tokens escrowed.
     * @param blockNumberMod Block number at which the swap transaction was commited (mod 32)
     */
    function onSendLiquidityFailure(
        bytes32 channelId,
        bytes calldata toAccount,
        uint256 U,
        uint256 escrowAmount,
        uint32 blockNumberMod
    ) onlyChainInterface public override virtual {

        bytes32 sendLiquidityHash = _computeSendLiquidityHash(
            toAccount,      // Ensures no collisions between different users
            U,              // Used to randomise the hash
            escrowAmount,   // Required! to validate release escrow data
            blockNumberMod  // Used to randomize the hash.
        );

        // This function only allows entering this function once. Once called, it can never be called without reverting again.
        address fallbackAddress = _releaseLiquidityEscrow(sendLiquidityHash, escrowAmount); // Reverts for missing escrow

        _mint(fallbackAddress, escrowAmount); // Never reverts.

        emit SendLiquidityFailure(
            channelId,
            toAccount,
            U,
            escrowAmount,
            blockNumberMod
        );
    }

    /** 
     * @notice Computes a unique identifier for a swap. This unique identifier can be used to identify swaps cross-chain.
     * However, it is never exposed. This is done to let the hashing algorithm be flexible between implementations.
     */
    function _computeSendAssetHash(
        bytes calldata toAccount,
        uint256 U,
        uint256 amount,
        address fromAsset,
        uint32 blockNumberMod
    ) internal pure returns(bytes32) {
        return keccak256(
            bytes.concat(
                toAccount,               // Ensures no collisions between different users.
                bytes32(U),              // Used to randomise the hash.
                bytes32(amount),         // Required! to validate release escrow data.
                bytes20(fromAsset),      // Required! to validate release escrow data.
                bytes4(blockNumberMod)   // Used to randomize the hash.
            )
        );
    }

    /**
     * @notice Computes a unique identifier for a swap. This unique identifier can be used to identify swaps cross-chain.
     * However, it is never exposed. This is done to let the hashing algorithm be flexible between implementations.
     */
    function _computeSendLiquidityHash(
        bytes calldata toAccount,
        uint256 U,
        uint256 amount,
        uint32 blockNumberMod
    ) internal pure returns(bytes32) {
        return keccak256(
            bytes.concat(
                toAccount,               // Ensures no collisions between different users.
                bytes32(U),              // Used to randomise the hash.
                bytes32(amount),         // Required! to validate release escrow data.
                bytes4(blockNumberMod)   // Used to randomize the hash.
            )
        );
    }

    // -- Underwrite Asset Swaps -- //

    function underwriteAsset(
        bytes32 identifier,
        address toAsset,
        uint256 U,
        uint256 minOut
    ) onlyChainInterface virtual public returns (uint256 purchasedTokens) {
        // Simulate a receiveAsset call. This gets us the purchased tokens.
        purchasedTokens = _receiveAsset(toAsset, U, minOut);

        // Set the escrow.
        _setTokenEscrow(
            identifier,
            UNDERWRITE_FALLBACK_USER,
            toAsset,
            purchasedTokens
        );

        emit SwapUnderwritten(
            identifier,
            toAsset,
            U,
            purchasedTokens
        );
    }

    /**
     * @notice Release assets associated with an underwrite escrow.
     * @param refundTo Released assets are sent to this address.
     * @param identifier Underwriting identifier. Is used to index the storage for valid escrows.
     * @param escrowAmount Number of tokens escrowed.
     * @param escrowToken Escrowed token address.
     * @param sourceIdentifier The source chain identifier.
     * @param fromVault The originating vault.
     */
    function releaseUnderwriteAsset(
        address refundTo,
        bytes32 identifier,
        uint256 escrowAmount,
        address escrowToken,
        bytes32 sourceIdentifier,
        bytes calldata fromVault
    ) onlyChainInterface onlyConnectedPool(sourceIdentifier, fromVault) virtual public {
        _releaseAssetEscrow(identifier, escrowAmount, escrowToken); // Reverts for missing escrow.

        // Send the assets to the user.
        SafeTransferLib.safeTransfer(escrowToken, refundTo, escrowAmount);
    }

    /**
     * @notice Delete an underwrite escrow without releasing any tokens.
     * @dev The unsued parameter U is used for overwrites. (see CataulystVaultAmplified.sol)
     * @param identifier Underwriting identifier. Is used to index the storage for valid escrows.
     * param U Number of underwritten units. Is used for Amplified vaults to modify the unit tracker.
     * @param escrowAmount Number of tokens escrowed.
     * @param escrowToken Escrowed token address.
     */
    function deleteUnderwriteAsset(
        bytes32 identifier,
        uint256 /* U */,
        uint256 escrowAmount,
        address escrowToken
    ) onlyChainInterface virtual public {
        _releaseAssetEscrow(identifier, escrowAmount, escrowToken); // Reverts for missing escrow.
    }

}
