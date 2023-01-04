//SPDX-License-Identifier: Unlicsened

pragma solidity ^0.8.16;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "./SwapPoolFactory.sol";
import "./FixedPointMath.sol";
import "./CatalystIBCInterface.sol";
import "./interfaces/IOnCatalyst.sol";
import "./ICatalystV1Pool.sol";
import "./interfaces/ICatalystV1PoolErrors.sol";

/**
 * @title Catalyst: The Multi-Chain Swap Pool Core
 * @author Catalyst Labs
 */
abstract contract CatalystSwapPoolCommon is
    CatalystFixedPointMath,
    ERC20,
    ICatalystV1PoolErrors,
    ICatalystV1Pool
{
    using SafeERC20 for IERC20;

    //--- Config ---//
    // The following section contains the configurable variables.

    /// @notice Determines how fast the security limit decreases.
    /// @dev
    ///     Needs to be long enough for pool token providers to be notified
    ///     of a beach but short enough for volatility to not soft-freeze the pool.
    uint256 constant DECAYRATE = 60 * 60 * 24;

    /// @notice Maximum number of assets suppoed
    uint8 constant NUMASSETS = 3;

    //-- Variables --//

    address public _chaininterface;
    address public _factory;

    /// @notice
    ///     If the pool has no cross chain connection, this is true.
    ///     Should not be trusted if setupMaster != ZERO_ADDRESS
    bool public _onlyLocal;

    /// @notice
    ///     To indicate which token is desired on the target pool,
    ///     the _toAsset is an integer from 0 to NUMASSETS indicating
    ///     which asset the pool should purchase with units.
    mapping(uint256 => address) public _tokenIndexing;

    /// @notice The token weights. Used for maintaining a non symmetric pool balance.
    mapping(address => uint256) public _weight;
    mapping(address => uint256) public _targetWeight;
    uint256 public _adjustmentTarget;
    uint256 public _lastModificationTime;
    uint256 public _target_max_unit_inflow;

    /// @notice The pool fee in X64. Implementation of fee: mulX64(_amount, self.poolFeeX64)
    uint256 public _poolFeeX64;
    uint256 public _governanceFee;
    address public _feeAdministrator; // The address of the responsible for adjusting the fees.

    /// @notice The setupMaster is the short-term owner of the pool.
    ///     They can connect the pool to pools on other chains.
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
    // max_liquidity_unit_inflow: public(uint256) = totalSupply / 2
    uint256 _unit_flow;
    uint256 _last_change;
    uint256 _liquidity_flow;
    uint256 _last_liquidity_change;

    // Escrow reference
    /// @notice Total current escrowed tokens
    mapping(address => uint256) public _escrowedTokens;
    /// @notice Specific escrow information
    mapping(bytes32 => address) public _escrowedFor;

    /// @notice Total current escrowed pool tokens
    uint256 public _escrowedPoolTokens;
    /// @notice Specific escrow information (Pool Tokens)
    mapping(bytes32 => address) public _escrowedLiquidityFor;

    bool _CHECK;

    constructor() ERC20("", "") {
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
        return 64;
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
        // The pool is only designed to be used by a proxy and not as a standalone.
        // as a result self.check is set to TRUE on init, to stop anyone from using
        // the pool without a proxy.
        require(!_CHECK); // dev: Pool Already setup.
        _onlyLocal = chaininterface == address(0);
        _CHECK = true;

        _chaininterface = chaininterface;
        _setupMaster = setupMaster;
        _factory = msg.sender;

        // Names the ERC20 pool token //
        __name = name_;
        __symbol = symbol_;
        // END ERC20 //

        // Mint 1 pool token to the short-term pool owner.
        _mint(setupMaster, ONE);
    }

    /** @notice  Returns the current cross-chain unit capacity. */
    function getUnitCapacity() external view override returns (uint256) {
        uint256 MUF = _max_unit_inflow;
        // If the current time is more than DECAYRATE since the last update
        // then the maximum unit inflow applies
        if (block.timestamp > DECAYRATE + _last_change) return MUF;

        // The delta unit limit is: timePassed · slope = timePassed · Max/decayrate
        uint256 delta_flow = (MUF * (block.timestamp - _last_change)) /
            DECAYRATE;

        uint256 UF = _unit_flow;
        if (UF <= delta_flow) return MUF;

        // No underflow since _unit_flow > delta_flow
        if (MUF <= UF) return 0; // Amplified pools can have MUF <= UF since MUF is modified when swapping
        return MUF - (UF - delta_flow);
    }

    /**
     * @notice
     *     Checks if the pool supports an inflow of units and decreases
     *     unit capacity by units.
     * @param units The number of units to check and set.
     */
    function checkAndSetUnitCapacity(uint256 units) internal {
        uint256 MUF = _max_unit_inflow;
        if (block.timestamp > DECAYRATE + _last_change) {
            require(units < MUF, EXCEEDS_SECURITY_LIMIT);
            // After correcting self._unit_flow it would be 0.
            // Thus the new _unit_flow is units.
            _unit_flow = units;
            _last_change = block.timestamp;
            return;
        }

        uint256 delta_flow = (MUF * (block.timestamp - _last_change)) /
            DECAYRATE;
        _last_change = block.timestamp; // Here purely because of optimizations.
        // Otherwise it would have to be repeated twice. (small deployment savings)
        uint256 UF = _unit_flow; // Used twice, in memory (potentially) saves 100 gas.
        if (UF <= delta_flow) {
            require(units < MUF, EXCEEDS_SECURITY_LIMIT);
            // After correcting self._unit_flow it would be 0.
            // Thus the new _unit_flow is units.;
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

    // TODO do we want to limit max pool fee (max 1?)
    function setPoolFee(uint256 newPoolFeeX64) external override {
        require(msg.sender == _feeAdministrator); // dev: Only feeAdministrator can set new fee
        _poolFeeX64 = newPoolFeeX64;
    }

    // TODO here for testing purposes until governance fee logic is finalised
    // If left, add to contract interface
    function setGovernanceFee(uint256 newPoolGovernanceFeeX64) external {
        require(msg.sender == _feeAdministrator); // dev: Only feeAdministrator can set new fee
        require(newPoolGovernanceFeeX64 <= 2**63); // GovernanceFee is maximum 50%.
        _governanceFee = newPoolGovernanceFeeX64;
    }

    /**
     * @notice Creates a connection to the pool _poolReceiving on the channel _channelId.
     * @dev if _poolReceiving is an EVM pool, it can be computes as:
     *     Vyper: convert(_poolAddress, bytes32)
     *     Solidity: abi.encode(_poolAddress)
     *     Brownie: brownie.convert.to_bytes(_poolAddress, type_str="bytes32")
     * ! Notice, using tx.origin is not secure.
     * However, it makes it easy to bundle call from an external contract
     * and no assets are at risk because the pool should not be used without
     * setupMaster == ZERO_ADDRESS
     * @param channelId The _channelId of the target pool.
     * @param poolReceiving The bytes32 representation of the target pool
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

    /**
     * @notice Gives up short term ownership of the pool. This makes the pool unstoppable.
     * @dev ! Using tx.origin is not secure.
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
     * @notice
     *     External view function purely used to signal if a pool is safe to use.
     * @dev
     *     Just checks if the setup master has been set to ZERO_ADDRESS.
     *     In other words, has finishSetup been called?
     */
    function ready() external view override returns (bool) {
        return _setupMaster == address(0);
    }

    //-- Escrow Functions --//

    /** @notice Release the escrowed tokens into the pool.  */
    function releaseEscrowACK(
        bytes32 messageHash,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken
    ) external override {
        require(msg.sender == _chaininterface);

        address fallbackUser = _escrowedFor[messageHash]; // (2) Passing in an empty messageHash gives 0,0,0
        require(fallbackUser != address(0));
        delete _escrowedFor[messageHash]; // (2) This will pass, since empty can be deleted. (Nothing happens)
        _escrowedTokens[escrowToken] -= escrowAmount; // (2) This passes, since _escrowedTokens[0] = 0 => 0 - 0 = 0

        emit EscrowAck(messageHash, false); // (2) This passes,

        // Incoming swaps should be subtracted from the unit flow.
        // It is assumed if the router was fraudulent, that no-one would execute a trade.
        // As a result, if people swap into the pool, we should expect that there is exactly
        // the inswapped amount of trust in the pool. If this wasn't implemented, there would be
        // a maximum daily cross chain volume, which is bad for liquidity providers.
        {
            // Calling timeout and then ack should not be possible.
            // (1) Protects Timeout from being called after ack, since the information is zeroed.
            // (2) Checking for existing escrow is not done implicitly, so it has to be done explicitly.
            uint256 UF = _unit_flow;
            // If UF < U and we do UF - U < 0 underflow => bad.
            if (UF > U) {
                _unit_flow -= U;
            } else if (UF != 0) {
                // # Save ~100 gas if UF = 0.
                _unit_flow = 0;
            }
        }
    }

    /** @notice Returned the escrowed tokens to the user */
    function releaseEscrowTIMEOUT(
        bytes32 messageHash,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken
    ) external override {
        require(msg.sender == _chaininterface); // Never reverts when truely recovering funds.

        address fallbackUser = _escrowedFor[messageHash]; // If funds exist, then information exists.
        require(fallbackUser != address(0));
        delete _escrowedFor[messageHash]; // To remove the possibility of reentry.
        _escrowedTokens[escrowToken] -= escrowAmount; // Underflow? Only if someone is able to modify the _escrowedTokens value to be lower. (Reentry?: No)

        IERC20(escrowToken).safeTransfer(fallbackUser, escrowAmount); // (1) If escrowInformation.token == address(0), this fails. // 1. Would fail if there is no balance. We remove the balance of the escrow from what is claimable by users. 2. (Solana: If the user is valid.)

        emit EscrowTimeout(messageHash, false); // Cannot fail.
    }

    /** @notice Release the escrowed tokens into the pool.  */
    function releaseLiquidityEscrowACK(
        bytes32 messageHash,
        uint256 U,
        uint256 escrowAmount
    ) external override {
        require(msg.sender == _chaininterface);

        address fallbackUser = _escrowedLiquidityFor[messageHash];
        require(fallbackUser != address(0));
        delete _escrowedLiquidityFor[messageHash];
        _escrowedPoolTokens -= escrowAmount;

        emit EscrowAck(messageHash, true);

        // Incoming swaps should be subtracted from the unit flow.
        // It is assumed if the router was fraudulent, that no-one would execute a trade.
        // As a result, if people swap into the pool, we should expect that there is exactly
        // the inswapped amount of trust in the pool. If this wasn't implemented, there would be
        // a maximum daily cross chain volume, which is bad for liquidity providers.
        {
            // Calling timeout and then ack should not be possible.
            // (1) Protects Timeout from being called after ack, since the information is zeroed.
            // (2) Checking for existing escrow is not done implicitly, so it has to be done explicitly.
            uint256 UF = _unit_flow;
            // If UF < U and we do UF - U < 0 underflow => bad.
            if (UF > U) {
                _unit_flow -= U;
            } else if (UF != 0) {
                // # Save ~100 gas if UF = 0.
                _unit_flow = 0;
            }
        }
    }

    /** @notice Returned the escrowed tokens to the user. For liquidity escrows */
    function releaseLiquidityEscrowTIMEOUT(
        bytes32 messageHash,
        uint256 U,
        uint256 escrowAmount
    ) external override {
        require(msg.sender == _chaininterface); // Never reverts when truely recovering funds.

        address fallbackUser = _escrowedLiquidityFor[messageHash]; // If funds exist, then information exists.
        require(fallbackUser != address(0)); // Only fails if the escrow doesn't exist. (desired)
        delete _escrowedLiquidityFor[messageHash]; // To remove the possibility of reentry.
        _escrowedPoolTokens -= escrowAmount; // Underflow? Only if someone is able to modify the _escrowedPoolTokens value to be lower. (Reentry?: No)

        _mint(fallbackUser, escrowAmount); // 1. Does not fail. 2. (Solana: If the user is valid.)

        emit EscrowTimeout(messageHash, true); // Cannot fail
    }
}
