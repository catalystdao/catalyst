//SPDX-License-Identifier: Unlicensed

pragma solidity ^0.8.16;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import "@openzeppelin/contracts/utils/Multicall.sol";
import "./SwapPoolFactory.sol";
import "./FixedPointMathLib.sol";
import "./CatalystIBCInterface.sol";
import "./interfaces/IOnCatalyst.sol";
import "./ICatalystV1Pool.sol";
import "./interfaces/ICatalystV1PoolErrors.sol";

/**
 * @title Catalyst: Common Swap Pool Logic
 * @author Catalyst Labs
 * @notice This abstract contract defines general logic of a
 * Catalyst swap pool like:
 * - Pool Token
 * - Connection management
 * - Security limit
 * - Escrow
 *
 * By inheriting this contract, a Swap Pool automatically implements
 * common swap pool logic.
 */
abstract contract CatalystSwapPoolCommon is
    Initializable,
    Multicall,
    ERC20,
    ICatalystV1PoolErrors,
    ICatalystV1Pool
{
    using SafeERC20 for IERC20;

    //--- Config ---//
    // The following section contains the configurable variables.

    /// @notice Determines how fast the security limit decreases.
    /// @dev Needs to be long enough for pool token providers to be notified of a breach but short enough for volatility to not soft-freeze the pool.
    uint256 constant DECAY_RATE = 60 * 60 * 24;

    /// @notice Number of decimals used by the pool's pool tokens
    uint8 constant DECIMALS = 18;

    /// @notice The pool tokens initially minted to the user who setup the pools.
    /// @dev The initial deposit along with this value determines the base value of a pool token.
    uint256 constant INITIAL_MINT_AMOUNT = 10**DECIMALS;

    /// @notice Maximum number of assets supported
    /// @dev Impacts the cost of some for loops. Can be changed without breaking compatiblity.
    uint8 constant MAX_ASSETS = 3;

    //-- Variables --//

    address public immutable FACTORY;
    address public _chainInterface;

    /// @notice To indicate which token is desired on the target pool,
    /// the desired tokens is provided as an integer which maps to the
    /// asset address. This variable is the map.
    mapping(uint256 => address) public _tokenIndexing;

    /// @notice The token weights. Used for maintaining a non symmetric pool balance.
    mapping(address => uint256) public _weight;

    //-- Weight change variables --//
    uint256 public _adjustmentTarget;
    uint256 public _lastModificationTime;

    /// @notice The total pool fee. Multiplied by 10**18. 
    /// @dev Implementation of fee: FixedPointMathLib.mulWadDown(amount, _poolFee);
    uint256 public _poolFee;
    /// @notice The governance's cut of _poolFee. 
    /// @dev FixedPointMathLib.mulWadDown(FixedPointMathLib.mulWadDown(amount, _poolFee), _governanceFeeShare);
    uint256 public _governanceFeeShare;
    /// @notice The fee pool fee can be changed. _feeAdministrator is the address allowed to change it
    address public _feeAdministrator; 

    /// @notice The setupMaster is the short-term owner of the pool.
    /// They can connect the pool to pools on other chains.
    /// @dev !Can extract all of the pool value!
    address public _setupMaster;

    //--- Messaging router limit ---//
    // The router is not completely trusted. Some limits are
    // imposed on the DECAY_RATE-ly unidirectional liquidity flow. That is:
    // if the pool observes more than _maxUnitCapacity of incoming
    // units, then it will not accept further incoming units. This means the router
    // can only drain a prefigured percentage of the pool every DECAY_RATE

    // Outgoing flow is subtracted incoming flow until 0.

    /// @notice The max incoming liquidity flow from the router.
    uint256 public _maxUnitCapacity;
    // -- State related to unit flow calculation -- //
    // Use getUnitCapacity to indircetly access these variables.
    uint256 _usedUnitCapacity;
    uint256 _usedUnitCapacityTimestamp;

    // Escrow reference
    /// @notice Total current escrowed tokens
    mapping(address => uint256) public _escrowedTokens;
    /// @notice Specific escrow information
    mapping(bytes32 => address) public _escrowedFor;

    /// @notice Total current escrowed pool tokens
    uint256 public _escrowedPoolTokens;
    /// @notice Specific escrow information (Liquidity)
    mapping(bytes32 => address) public _escrowedLiquidityFor;

    /// @notice Variable to check if the pool has already been setup.
    bool _INITIALIZED;

    constructor(address factory_) ERC20("Catalyst Pool Template", "") {
        FACTORY = factory_;

        _disableInitializers();
        __name = "Catalyst Pool Template";
        __symbol = "";
    }

    // Overriding the name and symbol storage variables
    string private __name;
    string private __symbol;

    function name() public view override returns (string memory) {
        return __name;
    }

    function symbol() public view override returns (string memory) {
        return __symbol;
    }

    function decimals() public pure override returns (uint8) {
        return DECIMALS;
    }

    function factoryOwner() public view override returns (address) {
        return CatalystSwapPoolFactory(FACTORY).owner();
    }

    /**
     * @notice Only allow Governance to change pool parameters
     * @dev Because of dangours permissions (setConnection, weight changes, amplification changes):
     * !CatalystSwapPoolFactory(_factory).owner() must be set to a timelock! 
     */ 
    modifier onlyFactoryOwner() {
        require(msg.sender == CatalystSwapPoolFactory(FACTORY).owner());   // dev: Only factory owner
        _;
    }

    function onlyLocal() public view returns (bool) {
        return _chainInterface == address(0);
    }

    /** @notice Setup a pool. */
    function setup(
        string calldata name_,
        string calldata symbol_,
        address chainInterface,
        uint256 poolFee,
        uint256 governanceFee,
        address feeAdministrator,
        address setupMaster
    ) initializer external {
        // The pool is designed to be used by a proxy and not as a standalone pool.
        // On pool deployment check is set to TRUE, to stop anyone from using the pool without a proxy.
        // Likewise, it shouldn't be possible to setup the pool twice.
        require(!_INITIALIZED); // dev: Pool Already setup.

        _chainInterface = chainInterface;
        _setupMaster = setupMaster;

        setPoolFee(poolFee);
        setGovernanceFee(governanceFee);
        setFeeAdministrator(feeAdministrator);

        // Names the ERC20 pool token //
        __name = name_;
        __symbol = symbol_;
        // END ERC20 //

        _INITIALIZED = true;
    }

    /** @notice  Returns the current cross-chain swap capacity. */
    function getUnitCapacity() external view virtual override returns (uint256) {
        uint256 MUC = _maxUnitCapacity;

        // The delta change to the limit is: timePassed · slope = timePassed · Max/decayrate
        uint256 unitCapacityReleased = ((block.timestamp - _usedUnitCapacityTimestamp) * MUC) / DECAY_RATE;

        uint256 UC = _usedUnitCapacity;
        // If the change is greater than the units which has passed through
        // return maximum. We do not want (MUC - (UC - unitCapacityReleased) > MUC)
        if (UC <= unitCapacityReleased) return MUC;

        // Amplified pools can have MUC <= UC since MUC is modified when swapping
        if (MUC <= UC - unitCapacityReleased) return 0; 

        return MUC + unitCapacityReleased - UC;     // MUC - (UC - unitCapacityReleased)
    }

    /**
     * @notice Checks if the pool supports an inflow of units and decreases
     * unit capacity by the inflow.
     * @dev Implement a lot of similar logic to getUnitCapacity. 
     * @param units The number of units to check and set.
     */
    function updateUnitCapacity(uint256 units) internal {
        uint256 MUC = _maxUnitCapacity;

        uint256 unitCapacityReleased = ((block.timestamp - _usedUnitCapacityTimestamp) * MUC) / DECAY_RATE;

        // Set last change to block.timestamp.
        // Otherwise it would have to be repeated twice. (small deployment savings)
        _usedUnitCapacityTimestamp = block.timestamp; 

        uint256 UC = _usedUnitCapacity; 
        // If the change is greater than the units which has passed through the limit is max
        if (UC <= unitCapacityReleased) {
            require(units <= MUC, EXCEEDS_SECURITY_LIMIT);
            _usedUnitCapacity = units;
            return;
        }

        uint256 newUnitFlow = (UC + units) - unitCapacityReleased;
        require(newUnitFlow <= MUC, EXCEEDS_SECURITY_LIMIT);
        _usedUnitCapacity = newUnitFlow;
    }

    
    function setFeeAdministrator(address administrator) public override {
        require(msg.sender == factoryOwner() || _isInitializing());   // dev: Only factory owner
        _feeAdministrator = administrator;

        emit SetFeeAdministrator(administrator);
    }

    function setPoolFee(uint256 fee) public override {
        require(msg.sender == _feeAdministrator || _isInitializing()); // dev: Only feeAdministrator can set new fee
        require(fee <= 10**18);  // dev: PoolFee is maximum 100%.
        _poolFee = fee;

        emit SetPoolFee(fee);
    }

    function setGovernanceFee(uint256 fee) public override {
        require(msg.sender == _feeAdministrator || _isInitializing()); // dev: Only feeAdministrator can set new fee
        require(fee <= MAX_GOVERNANCE_FEE_SHARE); // dev: Maximum GovernanceFeeSare exceeded.
        _governanceFeeShare = fee;

        emit SetGovernanceFee(fee);
    }

    /**
     * @dev Collect the governance fee share of the specified pool fee
     */
    function collectGovernanceFee(uint256 poolFeeAmount, address asset) internal {

        uint256 governanceFeeShare = _governanceFeeShare;

        if (governanceFeeShare != 0) {
            uint256 governanceFeeAmount = FixedPointMathLib.mulWadDown(
                poolFeeAmount,
                governanceFeeShare
            );
            IERC20(asset).safeTransfer(factoryOwner(), governanceFeeAmount);
        }
    }

    /**
     * @notice Creates a connection to targetPool on the channel_channelId.
     * @dev Encoding addresses in bytes32 for EVM can be done be computed with:
     * Vyper: convert(<poolAddress>, bytes32)
     * Solidity: abi.encode(<poolAddress>)
     * Brownie: brownie.convert.to_bytes(<poolAddress>, type_str="bytes32")
     * @param channelId Target chain identifier.
     * @param targetPool Bytes32 representation of the target pool.
     * @param state Boolean indicating if the connection should be open or closed.
     */
    function setConnection(
        bytes32 channelId,
        bytes32 targetPool,
        bool state
    ) external override {
        require((msg.sender == _setupMaster) || (msg.sender == factoryOwner())); // dev: No auth

        CatalystIBCInterface(_chainInterface).setConnection(
            channelId,
            targetPool,
            state
        );

        emit SetConnection(channelId, targetPool, state);
    }

    /**
     * @notice Gives up short term ownership of the pool making the pool unstoppable.
     */
    function finishSetup() external override {
        require(msg.sender == _setupMaster); // dev: No auth

        _setupMaster = address(0);

        emit FinishSetup();
    }

    /**
     * @notice View function to signal if a pool is safe to use.
     * @dev Checks if the setup master has been set to ZERO_ADDRESS.
     * In other words, has finishSetup been called?
     */
    function ready() external view override returns (bool) {
        return _setupMaster == address(0) && _tokenIndexing[0] != address(0);
    }


    //-- Escrow Functions --//

    function releaseTokenEscrow(
        bytes32 messageHash,
        uint256 escrowAmount,
        address escrowToken
    ) internal returns(address) {
        require(msg.sender == _chainInterface);  // dev: Only _chainInterface

        address fallbackUser = _escrowedFor[messageHash];  // Passing in an invalid messageHash returns address(0)
        require(fallbackUser != address(0));  // dev: Invalid messageHash. Alt: Escrow doesn't exist.
        delete _escrowedFor[messageHash];  // Stops timeout and further acks from being called

        _escrowedTokens[escrowToken] -= escrowAmount; // This does not revert, since escrowAmount \subseteq _escrowedTokens => escrowAmount <= _escrowedTokens. Cannot be called twice since the 3 lines before ensure this can only be reached once.
        
        return (fallbackUser);
    }

    function releaseLiquidityEscrow(
        bytes32 messageHash,
        uint256 escrowAmount
    ) internal returns(address) {
        require(msg.sender == _chainInterface);  // dev: Only _chainInterface

        address fallbackUser = _escrowedLiquidityFor[messageHash];  // Passing in an invalid messageHash returns address(0)
        require(fallbackUser != address(0));  // dev: Invalid messageHash. Alt: Escrow doesn't exist.
        delete _escrowedLiquidityFor[messageHash];  // Stops timeout and further acks from being called

        _escrowedPoolTokens -= escrowAmount;  // This does not revert, since escrowAmount \subseteq _escrowedPoolTokens => escrowAmount <= _escrowedPoolTokens. Cannot be called twice since the 3 lines before ensure this can only be reached once.
        
        return fallbackUser;
    }


    /** 
     * @notice Implements basic ack logic: Deletes and release tokens to the pool
     * @dev Should never revert! For security limit adjustments, the implementation should be overwritten.
     * @param messageHash A hash of the cross-chain message ensure the message arrives indentical to the sent message.
     * @param U The number of units initially purchased.
     * @param escrowAmount The number of tokens escrowed.
     * @param escrowToken The token escrowed.
     */
    function sendSwapAck(
        bytes32 messageHash,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken
    ) public virtual override {

        releaseTokenEscrow(messageHash, escrowAmount, escrowToken); // Only reverts for missing escrow

        emit EscrowAck(messageHash, false);  // Never reverts.
    }

    /** 
     * @notice Implements basic timeout logic: Deletes and sends tokens to the user.
     * @dev Should never revert!
     * @param messageHash A hash of the cross-chain message ensure the message arrives indentical to the sent message.
     * @param U The number of units initially purchased.
     * @param escrowAmount The number of tokens escrowed.
     * @param escrowToken The token escrowed.
     */
    function sendSwapTimeout(
        bytes32 messageHash,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken
    ) public virtual override {

        address fallbackAddress = releaseTokenEscrow(messageHash, escrowAmount, escrowToken); // Only reverts for missing escrow,

        IERC20(escrowToken).safeTransfer(fallbackAddress, escrowAmount);  // Would fail if there is no balance. To protect against this, the escrow amount is removed from what can be claimed by users.

        emit EscrowTimeout(messageHash, false);  // Never reverts.
    }

    /** 
     * @notice Implements basic liquidity ack logic: Deletes and releases pool tokens to the pool.
     * @dev Should never revert! For security limit adjustments, the implementation should be overwritten.
     * @param messageHash A hash of the cross-chain message ensure the message arrives indentical to the sent message.
     * @param U The number of units initially acquired.
     * @param escrowAmount The number of pool tokens escrowed.
     */
    function sendLiquidityAck(
        bytes32 messageHash,
        uint256 U,
        uint256 escrowAmount
    ) public virtual override {

        releaseLiquidityEscrow(messageHash, escrowAmount); // Only reverts for missing escrow

        emit EscrowAck(messageHash, true);  // Never reverts.
    }

    /** 
     * @notice Implements basic liquidity timeout logic: Deletes and sends pool tokens to the user.
     * @dev Should never revert!
     * @param messageHash A hash of the cross-chain message ensure the message arrives indentical to the sent message.
     * @param U The number of units initially acquired.
     * @param escrowAmount The number of pool tokens escrowed.
     */
    function sendLiquidityTimeout(
        bytes32 messageHash,
        uint256 U,
        uint256 escrowAmount
    ) public virtual override {

        address fallbackAddress = releaseLiquidityEscrow(messageHash, escrowAmount); // Only reverts for missing escrow

        _mint(fallbackAddress, escrowAmount);  

        emit EscrowTimeout(messageHash, true);  // Never reverts.
    }
}
