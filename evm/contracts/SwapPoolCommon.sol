//SPDX-License-Identifier: Unlicsened

pragma solidity ^0.8.16;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "./SwapPoolFactory.sol";
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
    ERC20,
    ICatalystV1PoolErrors,
    ICatalystV1Pool
{
    using SafeERC20 for IERC20;

    //--- Config ---//
    // The following section contains the configurable variables.

    /// @notice Determines how fast the security limit decreases.
    /// @dev Needs to be long enough for pool token providers to be notified of a beach but short enough for volatility to not soft-freeze the pool.
    uint256 constant DECAYRATE = 60 * 60 * 24;

    /// @notice The pool tokens initially minted to the user who setup the pools.
    /// @dev The initial deposit along with this value determines the base value of a pool token.
    uint256 constant MINTAMOUNT = 10**18;

    /// @notice Maximum number of assets suppoed
    /// @dev Impacts the cost of some for loops. Can be changed without breaking compatiblity.
    uint8 constant NUMASSETS = 3;

    //-- Variables --//

    address public _chaininterface;
    address public _factory;

    /// @notice True if the chain interface is set to address(0).
    bool public _onlyLocal;

    /// @notice To indicate which token is desired on the target pool,
    /// the desired tokens is provided as an integer which maps to the
    /// asset address. This variable is the map.
    mapping(uint256 => address) public _tokenIndexing;

    /// @notice The token weights. Used for maintaining a non symmetric pool balance.
    mapping(address => uint256) public _weight;

    //-- Weight change variables --//
    mapping(address => uint256) public _targetWeight;
    uint256 public _adjustmentTarget;
    uint256 public _lastModificationTime;
    uint256 public _target_max_unit_inflow;

    /// @notice The total pool fee. Multiplied by 10**18. 
    /// @dev Implementation of fee: FixedPointMathLib.mulWadDown(amount, _poolFee);
    uint256 public _poolFee;
    /// @notice The governance's cut of _poolFee. 
    /// @dev FixedPointMathLib.mulWadDown(FixedPointMathLib.mulWadDown(amount, _poolFee), _governanceFee);
    uint256 public _governanceFee;
    /// @notice The fee pool fee can be changed. _feeAdministrator is the address allowed to change it
    address public _feeAdministrator; 

    /// @notice The setupMaster is the short-term owner of the pool.
    /// They can connect the pool to pools on other chains.
    /// @dev !Can extract all of the pool value!
    address public _setupMaster;

    //--- Messaging router limit ---//
    // The router is not completely trusted. Some limits are
    // imposed on the DECAYRATE-ly unidirectional liquidity flow. That is:
    // if the pool observes more than self.max_unit_inflow of incoming
    // units, then it will not accept further volume. This means the router
    // can only drain a prefigured percentage of the pool every DECAYRATE

    // Outgoing flow is subtracted incoming flow until 0.

    /// @notice The max incoming liquidity flow from the router.
    uint256 public _max_unit_inflow;
    // -- State related to unit flow calculation -- //
    // Use getUnitCapacity to indircetly access these variables.
    uint256 _unit_flow;
    uint256 _last_change;

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
    bool _CHECK;

    constructor() ERC20("Catalyst Pool Template", "") {
        // Ensure setup cannot be called on the initial deployment.
        _CHECK = true;
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
        return 18;
    }

    function factoryOwner() public view override returns (address) {
        return CatalystSwapPoolFactory(_factory).owner();
    }

    modifier onlyFactoryOwner() {
        require(msg.sender == CatalystSwapPoolFactory(_factory).owner());
        _;
    }

    /** @notice Setup a pool. */
    function setupBase(
        string calldata name_,
        string calldata symbol_,
        address chaininterface,
        address setupMaster
    ) internal {
        // The pool is designed to be used by a proxy and not as a standalone pool.
        // On pool deployment check is set to TRUE, to stop anyone from using the pool without a proxy.
        // Likewise, it shouldn't be possible to setup the pool twice.
        require(!_CHECK); // dev: Pool Already setup.
        _CHECK = true;

        // If the chaininterface is set to address(0), disable cross-chain swaps entirely.
        _onlyLocal = chaininterface == address(0);

        _chaininterface = chaininterface;
        _setupMaster = setupMaster;
        _factory = msg.sender;

        // Names the ERC20 pool token //
        __name = name_;
        __symbol = symbol_;
        // END ERC20 //

        // Mint pool tokens to the short-term pool owner.
        _mint(setupMaster, MINTAMOUNT);
    }

    /** @notice  Returns the current cross-chain swao capacity. */
    function getUnitCapacity() external view override returns (uint256) {
        uint256 MUF = _max_unit_inflow;
        // If the time since last update is more than DECAYRATE return maximum
        if (block.timestamp > DECAYRATE + _last_change) return MUF;

        // The delta change to the limit is: timePassed · slope = timePassed · Max/decayrate
        uint256 delta_flow = ((block.timestamp - _last_change) * MUF) / DECAYRATE;

        uint256 UF = _unit_flow;
        // If the change is greater than the units which has passed through
        // return maximum. We do not want (MUF - (UF - delta_flow) > MUF)
        if (UF <= delta_flow) return MUF;

        // Amplified pools can have MUF <= UF since MUF is modified when swapping
        if (MUF <= UF - delta_flow) return 0; 

        return MUF + delta_flow - UF;
    }

    /**
     * @notice Checks if the pool supports an inflow of units and decreases
     * unit capacity by units.
     * @dev Implement a lot of similar logic to getUnitCapacity. 
     * @param units The number of units to check and set.
     */
    function checkAndSetUnitCapacity(uint256 units) internal {
        uint256 MUF = _max_unit_inflow;
        // If the time since last update is more than DECAYRATE, the security limit is max
        if (block.timestamp > DECAYRATE + _last_change) {
            require(units < MUF, EXCEEDS_SECURITY_LIMIT);
            _unit_flow = units;
            _last_change = block.timestamp;
            return;
        }

        uint256 delta_flow = (MUF * (block.timestamp - _last_change)) / DECAYRATE;

        // Set last change to block.timestamp.
        // Otherwise it would have to be repeated twice. (small deployment savings)
        _last_change = block.timestamp; 

        uint256 UF = _unit_flow; 
        // If the change is greater than the units which has passed through the limit is max
        if (UF <= delta_flow) {
            require(units < MUF, EXCEEDS_SECURITY_LIMIT);
            _unit_flow = units;
            return;
        }

        uint256 newUnitFlow = (UF + units) - delta_flow;
        require(newUnitFlow < MUF, EXCEEDS_SECURITY_LIMIT);
        _unit_flow = newUnitFlow;
    }

    function setFeeAdministrator(address newFeeAdministrator)
        external
        override
        onlyFactoryOwner
    {
        _feeAdministrator = newFeeAdministrator;
    }

    /// @dev There is no maximum pool fee.
    function setPoolFee(uint256 newPoolFee) external override {
        require(msg.sender == _feeAdministrator); // dev: Only feeAdministrator can set new fee
        _poolFee = newPoolFee;
    }

    // TODO add to contract interface
    function setGovernanceFee(uint256 newPoolGovernanceFee) external {
        require(msg.sender == _feeAdministrator); // dev: Only feeAdministrator can set new fee
        require(newPoolGovernanceFee <= 75**17); // dev: GovernanceFee is maximum 75%.
        _governanceFee = newPoolGovernanceFee;
    }

    // TODO Event for connection
    /**
     * @notice Creates a connection to poolReceiving on the channel_channelId.
     * @dev Encoding addresses in bytes32 for EVM can be done be computed with:
     * Vyper: convert(<poolAddress>, bytes32)
     * Solidity: abi.encode(<poolAddress>)
     * Brownie: brownie.convert.to_bytes(<poolAddress>, type_str="bytes32")
     *
     * ! Notice, using tx.origin is not secure.
     * However, it makes it easy to bundle call from an external contract
     * and no assets are at risk because the pool should not be used without
     * setupMaster == ZERO_ADDRESS
     * @param channelId The target chain identifier.
     * @param poolReceiving The bytes32 representation of the target pool.
     * @param state Boolean indicating if the connection should be open or closed.
     */
    function createConnection(
        bytes32 channelId,
        bytes32 poolReceiving,
        bool state
    ) external override {
        // ! tx.origin ! Read @dev.
        require(
            (tx.origin == _setupMaster) ||
            (msg.sender == _setupMaster) ||
            (msg.sender == factoryOwner())
        ); // dev: No auth

        CatalystIBCInterface(_chaininterface).CreateConnection(
            channelId,
            poolReceiving,
            state
        );
    }

    // TODO Event for pool finalisation
    /**
     * @notice Gives up short term ownership of the pool making the pool unstoppable.
     * @dev !Using tx.origin is not secure!
     * However, it makes it easy to bundle call from an external contract
     * and no assets are at risk because the pool should not be used without
     * setupMaster == ZERO_ADDRESS
     */
    function finishSetup() external override {
        // ! tx.origin ! Read @dev.
        require((tx.origin == _setupMaster) || (msg.sender == _setupMaster)); // dev: No auth

        _setupMaster = address(0);
    }

    /**
     * @notice View function to signal if a pool is safe to use.
     * @dev Checks if the setup master has been set to ZERO_ADDRESS.
     * In other words, has finishSetup been called?
     */
    function ready() external view override returns (bool) {
        return _setupMaster == address(0);
    }


    //-- Escrow Functions --//

    /** 
     * @notice Deletes and releases escrowed tokens to the pool.
     * @dev Should never revert!  
     * @param messageHash A hash of the cross-chain message used ensure the message arrives indentical to the sent message.
     * @param U The number of units initially purchased.
     * @param escrowAmount The number of tokens escrowed.
     * @param escrowToken The token escrowed.
     */
    function releaseEscrowACK(
        bytes32 messageHash,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken
    ) external override {
        require(msg.sender == _chaininterface);  // dev: Only _chaininterface

        address fallbackUser = _escrowedFor[messageHash];  // Passing in an invalid messageHash returns address(0)
        require(fallbackUser != address(0));  // dev: Invalid messageHash. Alt: Escrow doesn't exist.
        delete _escrowedFor[messageHash];  // Stops timeout and further acks from being called
        _escrowedTokens[escrowToken] -= escrowAmount; // This does not revert, since escrowAmount \subseteq _escrowedTokens => escrowAmount <= _escrowedTokens. Cannot be called twice since the 3 lines before ensure this can only be reached once.
        emit EscrowAck(messageHash, false);  // Never reverts.

        // Incoming swaps should be subtracted from the unit flow.
        // It is assumed if the router was fraudulent, that no-one would execute a trade.
        // As a result, if people swap into the pool, we should expect that there is exactly
        // the inswapped amount of trust in the pool. If this wasn't implemented, there would be
        // a maximum daily cross chain volume, which is bad for liquidity providers.
        {
            // Calling timeout and then ack should not be possible. 
            // The initial lines deleting the escrow protects against this.
            uint256 UF = _unit_flow;
            // If UF < U and we do UF - U < 0 underflow => bad.
            if (UF > U) {
                _unit_flow = UF - U; // Does not underflow since _unit_flow > U.
            } else if (UF != 0) {
                // If UF == 0, then we shouldn't do anything. Skip that case.
                // when UF <= U => UF - U <= 0 => max(UF - U, 0) = 0
                _unit_flow = 0;
            }
        }
    }

    /** 
     * @notice Deletes and releases escrowed tokens to the user.
     * @dev Should never revert!  
     * @param messageHash A hash of the cross-chain message used ensure the message arrives indentical to the sent message.
     * @param U The number of units initially purchased.
     * @param escrowAmount The number of tokens escrowed.
     * @param escrowToken The token escrowed.
     */
    function releaseEscrowTIMEOUT(
        bytes32 messageHash,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken
    ) external override {
        require(msg.sender == _chaininterface);  // dev: Only _chaininterface

        address fallbackUser = _escrowedFor[messageHash];  // Passing in an invalid messageHash returns address(0)
        require(fallbackUser != address(0));   // dev: Invalid messageHash. Alt: Escrow doesn't exist.
        delete _escrowedFor[messageHash];  // Stops ack and further timeouts from being called.
        _escrowedTokens[escrowToken] -= escrowAmount; // This does not revert, since escrowAmount \subseteq _escrowedTokens => escrowAmount <= _escrowedTokens. Cannot be called twice since the 3 lines before ensure this can only be reached once.
        // releaseEscrowACK cannot be called with the same message hash after delete _escrowedFor[messageHash];

        IERC20(escrowToken).safeTransfer(fallbackUser, escrowAmount);  // Would fail if there is no balance. To protect against this, the escrow amount is removed from what can be claimed by users.

        emit EscrowTimeout(messageHash, false);  // Never reverts.
    }

    /** 
     * @notice Deletes and releases liquidity escrowed tokens to the pool.
     * @dev Should never revert!
     * @param messageHash A hash of the cross-chain message used ensure the message arrives indentical to the sent message.
     * @param U The number of units initially acquired.
     * @param escrowAmount The number of pool tokens escrowed.
     */
    function releaseLiquidityEscrowACK(
        bytes32 messageHash,
        uint256 U,
        uint256 escrowAmount
    ) external override {
        require(msg.sender == _chaininterface);   // dev: Only _chaininterface

        address fallbackUser = _escrowedLiquidityFor[messageHash];   // Passing in an invalid messageHash returns address(0)
        require(fallbackUser != address(0));  // dev: Invalid messageHash. Alt: Escrow doesn't exist.
        delete _escrowedLiquidityFor[messageHash];  // Stops timeout and further acks from being called
        _escrowedPoolTokens -= escrowAmount;  // This does not revert, since escrowAmount \subseteq _escrowedPoolTokens => escrowAmount <= _escrowedPoolTokens. Cannot be called twice since the 3 lines before ensure this can only be reached once.

        emit EscrowAck(messageHash, true);  // Never reverts.

        // Incoming swaps should be subtracted from the unit flow.
        // It is assumed if the router was fraudulent, that no-one would execute a trade.
        // As a result, if people swap into the pool, we should expect that there is exactly
        // the inswapped amount of trust in the pool. If this wasn't implemented, there would be
        // a maximum daily cross chain volume, which is bad for liquidity providers.
        {
            // Calling timeout and then ack should not be possible. 
            // The initial lines deleting the escrow protects against this.
            uint256 UF = _unit_flow;
            // If UF < U and we do UF - U < 0 underflow => bad.
            if (UF > U) {
                _unit_flow = UF - U; // Does not underflow since _unit_flow > U.
            } else if (UF != 0) {
                // If UF == 0, then we shouldn't do anything. Skip that case.
                // when UF <= U => UF - U <= 0 => max(UF - U, 0) = 0
                _unit_flow = 0;
            }
        }
    }

    /** 
     * @notice Deletes and releases escrowed pools tokens to the user.
     * @dev Should never revert!
     * @param messageHash A hash of the cross-chain message used ensure the message arrives indentical to the sent message.
     * @param U The number of units initially acquired.
     * @param escrowAmount The number of pool tokens escrowed.
     */
    function releaseLiquidityEscrowTIMEOUT(
        bytes32 messageHash,
        uint256 U,
        uint256 escrowAmount
    ) external override {
        require(msg.sender == _chaininterface);  // dev: Only _chaininterface

        address fallbackUser = _escrowedLiquidityFor[messageHash];  // Passing in an invalid messageHash returns address(0)
        require(fallbackUser != address(0));  // dev: Invalid messageHash. Alt: Escrow doesn't exist.
        delete _escrowedLiquidityFor[messageHash];  // Stops ack and further timeouts from being called.
        _escrowedPoolTokens -= escrowAmount; // This does not revert, since escrowAmount \subseteq _escrowedTokens => escrowAmount <= _escrowedTokens. Cannot be called twice since the 3 lines before ensure this can only be reached once.
        // releaseEscrowACK cannot be called with the same message hash after delete _escrowedFor[messageHash];

        _mint(fallbackUser, escrowAmount);  // Never reverts

        emit EscrowTimeout(messageHash, true);  // Never reverts.
    }
}
