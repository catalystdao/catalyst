# @version =0.3.3
"""
@title Catalyst: The Multi-Chain Swap pool
@author Catalyst Labs
@dev
    This contract is deployed /broken/: It cannot be used as a
    swap pool as is. To use it, a proxy contract duplicating the
    logic of this contract needs to be deployed. In vyper, this
    can be done through (vy 0.3.4) create_minimal_proxy_to.
    After deployment of the proxy, call setup(...). This will 
    initialize the pool and prepare it for cross-chain transactions.

    If connected to a supported cross-chain interface, call 
    createConnection or createConnectionWithChain to connect the
    pool with pools on other chains.

    Finally, call finishSetup to give up the deployer's control
    over the pool.

@notice
    Catalyst multi-chain swap pool using the asset specific
    pricing curve: W/(w · ln(2)) where W is an asset specific
    weight and w is the pool balance.

    The following contract supports between 1 and 3 assets for
    atomic swaps. To increase the number of tokens supported,
    change NUMASSETS to the desired maximum token amount.
    
    Implements the ERC20 specification, such that the contract
    will be its own pool token.
"""

### Config ### 
# The following section contains the configurable variables.
# Do not change other constants than the below.

# NUMASSETS | Determines the maximum number of assets supported 
# by the pool. Sums over the tokens within the pools are 
# required, and Vyper requires an upper limit for these loops. 
# (and on memory).
NUMASSETS: constant(uint256) = 3

# DECAYRATE | Determines how fast the security limit decreases.
# Needs to be long enough for pool token providers to be 
# notified of a beach but short enough for volatility to not
# soft-freeze the pool.
DECAYRATE: constant(uint256) = 60*60*24


### ERC20 Compatible Pool Token ###
# To keep track of pool liquidity ownership, the pool uses pool
# credits tokens to credit and debit ownership when withdrawing.
# This also enables depositors to transfer liquidity to others.

# Part of the following section is from the Vyperlang github:
# https://github.com/vyperlang/vyper/blob/a3adc44c4bf1ebd46baa6413f0f8dc0f58ab0b0c/examples/tokens/ERC20.vy
# Thanks to the contributors and akayuki Jimba (@yudetamago)

from vyper.interfaces import ERC20

implements: ERC20

event Transfer:
    sender: indexed(address)
    receiver: indexed(address)
    value: uint256


event Approval:
    owner: indexed(address)
    spender: indexed(address)
    value: uint256


# Pool description
name: public(String[64])
symbol: public(String[32])

# Public variables related to ERC20 compatibility.
balanceOf: public(HashMap[address, uint256])
allowance: public(HashMap[address, HashMap[address, uint256]])
totalSupply: public(uint256)


@external
def decimals() -> uint256:
    """
    @notice Returns 64 as the number of decimals.
    @dev
        64 was chosen in case the contract needs
        to perform any mathematical function on the
        number of owned pool tokens. 
        It didn't.

        As a result, this number varies slightly between implementations.
    """
    return 64


@external
def transfer(_to : address, _value : uint256) -> bool:
    """
    @dev Transfer token for a specified address
    @param _to The address to transfer to.
    @param _value The amount to be transferred.
    @dev
        The spending protections is based on 
        Vyper's underflow protection.
    """
    self.balanceOf[msg.sender] -= _value
    self.balanceOf[_to] += _value
    log Transfer(msg.sender, _to, _value)
    return True


@external
def transferFrom(_from : address, _to : address, _value : uint256) -> bool:
    """
    @dev Transfer tokens from one address to another.
    @param _from address The address which you want to send tokens from
    @param _to address The address which you want to transfer to
    @param _value uint256 the amount of tokens to be transferred
    @dev
        The spending protections is based on 
        Vyper's underflow protection.
        Setting allowance to MAX_UINT256 saves gas when calling transferFrom.
    """
    self.balanceOf[_from] -= _value
    self.balanceOf[_to] += _value
    if self.allowance[_from][msg.sender] != MAX_UINT256:
        self.allowance[_from][msg.sender] -= _value
    log Transfer(_from, _to, _value)
    return True


@external
def approve(_spender : address, _value : uint256) -> bool:
    """
    @dev Approve the passed address to spend the specified amount of tokens on behalf of msg.sender.
         Beware that changing an allowance with this method brings the risk that someone may use both the old
         and the new allowance by unfortunate transaction ordering. One possible solution to mitigate this
         race condition is to first reduce the spender's allowance to 0 and set the desired value afterwards:
         https://github.com/ethereum/EIPs/issues/20#issuecomment-263524729
    @param _spender The address which will spend the funds.
    @param _value The amount of tokens to be spent.
    """
    self.allowance[msg.sender][_spender] = _value
    log Approval(msg.sender, _spender, _value)
    return True


@internal
def _mint(_to: address, _value: uint256):
    """
    @dev Mint an amount of the token and assigns it to an account.
         This encapsulates the modification of balances such that the
         proper events are emitted.
    @param _to The account that will receive the created tokens.
    @param _value The amount that will be created.
    """
    assert _to != ZERO_ADDRESS
    self.totalSupply += _value
    self.balanceOf[_to] += _value
    log Transfer(ZERO_ADDRESS, _to, _value)


@internal
def _burn(_from: address, _value: uint256):
    """
    @dev Internal function that burns an amount of the token of a given
         account.
    @param _from The account whose tokens will be burned.
    @param _value The amount that will be burned.
    """
    assert _from != ZERO_ADDRESS
    self.totalSupply -= _value
    self.balanceOf[_from] -= _value
    log Transfer(_from, ZERO_ADDRESS, _value)


### Implementation of safe ERC20 transfer functions ###
# Since not all ERC20 tokens implementations are equal,
# the below implementation wraps the transfer function such
# that non-ERC20 conforming tokens are still supported.
# (handles "returns false but doesn't revert"
# and "doesn't return anything but reverts")


# https://github.com/vyperlang/vyper/issues/2202#issuecomment-718274642
@internal
def safeTransfer(_token: address, _to: address, _value: uint256):
    """
    @dev 
        Wraps "transfer(address,uint256)" of ERC20
        tokens to support more ERC20 tokens. Read ERC20 transfer
        for further documentation.
    @param _token The token to transfer
    @param _to The recipient of _value of token _token.
    @param _value The amount of _token to send to _to
    """
    _response: Bytes[32] = raw_call(
        _token,
        concat(
            method_id("transfer(address,uint256)"),
            convert(_to, bytes32),
            convert(_value, bytes32)
        ),
        max_outsize=32
    )

    # If the call returns a response, we should check that it is true.
    if len(_response) > 0:
        assert convert(_response, bool), "Transfer failed!"
    # If the call doesn't return a response, we should assume the
    # recipient is now _value richer.


@internal
def safeTransferFrom(_token: address, _from: address, _to: address, _value: uint256):
    """
    @dev 
        Wraps "transferFrom(address,address,uint256)" of ERC20
        tokens to support more ERC20 tokens. Read ERC20 transferFrom
        for further documentation.
    @param _token The token to transfer
    @param _from The sender of _value of token _token to _to
    @param _to The recipient of _value of token _token from _from
    @param _value The amount of _token to transfer from _from to _to.
    """
    _response: Bytes[32] = raw_call(
        _token,
        concat(
            method_id("transferFrom(address,address,uint256)"),
            convert(_from, bytes32),
            convert(_to, bytes32),
            convert(_value, bytes32)
        ),
        max_outsize=32
    )

    # If the call returns a response, we should check that it is true.
    if len(_response) > 0:
        assert convert(_response, bool), "TransferFrom failed!"
    # If the call doesn't return a response, we should assume the
    # recipient is now _value richer.


### The Polymer FixedPointMath LIB ###

# Natural logarithm of 2.
LN2: constant(uint256) = 12786308645202655660 #6931471806

# The number of decimal points in the mathematical library.
# The number 1 will then be equal to 2**pXX.
pXX: constant(int128) = 64

ONE: constant(uint256) = 2**pXX # Instead of repeating 2**pXX everywhere, it is simpler to write ONE
ONEONE: constant(uint256) = 2**(pXX*2) # Shortcut for 2**pXX * 2**pXX = 2**(2*pXX)


# Credit: https://medium.com/wicketh/mathemagic-full-multiply-27650fec525d
@pure
@internal 
def pMULX64(a : uint256, b : uint256) -> uint256:
    """
     @notice Safely calculates a : X64 times b : X64 while returning X64.
     @dev Reverts if a · b > 2**(256-64)-1
     @param a uint256, X64 Factor 1
     @param b uint256, X64 Factor 2
     @return uint256, X64:  a times b
    """
    r0: uint256 = unsafe_mul(a, b)  # same as uint256_mulmod(a,b, 2**256)
    r1: uint256 = uint256_mulmod(a, b, MAX_UINT256)
    r1 = (r1 - r0) - convert(r1 < r0, uint256)  # (r1 - r0) - {r1 < r0 ? 1 : 0}

    # !CRITICAL! Check for overflow into the third resolution.
    # We know r1 expand with shift(r1, 256-pXX)
    # r1 · 2^(256-64) = r1 · 2^192 < 2^256
    # r1 < 2^64
    # r1 < 2**64 - 1. Minus because we are indexing from 0.
    assert r1 < 2**64 - 1

    # The overflow part is now stored in r1 while the remainder is in r0.
    # The true number is thus r1 · 2^256 + r0
    # This could be improved to 
    points: uint256 = shift(r0, -pXX)

    return shift(r1, 256-pXX) + points


# While one could theoretically make these dynamically, hardcoding
# the shift is cheaper.
# Credit: https://github.com/vyperlang/vyper/issues/1086
@view
@internal
def bigdiv64(a : uint256, b : uint256) -> uint256:
    """
     @notice Safely calculates (a << 64)/b
     @dev 
        Reverts normally if result overflows.
        To get (a << p)/b replace (2**64 - 1) by (2**p - 1)
     @param a uint256 numerator
     @param b uint256 denominator
     @return uint256 (a << 64)/b
    """
    m: uint256 = (2**64 - 1) % b
    r: uint256 = (2**64 - 1) / b
    return r * a + (m + 1) * a / b


@pure
@internal
def _log2X64(x : uint256) -> uint256:
    """
    @notice
        Fixed point number can be written as
        x = m · 2^(-pXX)
        log2(x) = log2(m · 2^(-pXX)) = log2(m) + log2(2^(-pXX))
        log2(x) = log2(m) - pXX
        This finds the integer part

        Let a be the integer part of log2, then
        log2(x) - a is the decimal part.
        log2(x) - log2(2^a) = log2(x/2^a)
        x/2^a is definitely in [1, 2) and only 1 if number could be expressed as 2^a.
     @dev 
        for i in range(V) goes through the remaining bits. 
        Set v to the smaller bit one wants included
     @param x uint256, X64
     @return uint256, X64 as log2(x/2**64)*2**64
    """

    assert x >= 2**64
    x_i: uint256 = x
    log2_intermediate: uint256 = 0
    
    if x_i >= 2**128: 
        x_i = shift(x_i, -128)
        log2_intermediate += 128
    if x_i >= 2**64: 
        x_i = shift(x_i, -64)
        log2_intermediate += 64
    if x_i >= 2**32: 
        x_i = shift(x_i, -32)
        log2_intermediate += 32
    if x_i >= 2**16: 
        x_i = shift(x_i, -16)
        log2_intermediate += 16
    if x_i >= 2**8: 
        x_i = shift(x_i, -8)
        log2_intermediate += 8
    if x_i >= 2**4: 
        x_i = shift(x_i, -4)
        log2_intermediate += 4
    if x_i >= 2**2: 
        x_i = shift(x_i, -2)
        log2_intermediate += 2
    if x_i >= 2**1: 
        # x_i = shift(x_i, -1)
        log2_intermediate += 1
    log2_intermediate -= convert(pXX, uint256)

    # Secure the decimal point
    x_i = x/shift(1, convert(log2_intermediate, int128))
    log2_intermediate = shift(log2_intermediate, pXX)
    for i in range(24):  # 24 is Supposedly: 1/2**24 => .0.0000059605% deviation, but I am getting more like 1/2**20 deviation => .0000953674% deviation
        if x_i >= shift(2, pXX):
            log2_intermediate += shift(1, pXX - i)
            x_i = shift(x_i, -1)
        x_i = shift(x_i * x_i, -pXX)  # Since x_i is max 2**64*2*2 => (2**66) ** 2 = 2**132 => x_i · x_i < 2**132. No overflow
    
    return log2_intermediate


@pure
@internal
def _p2X64(x : uint256) -> uint256:
    """
    @notice
        We can write x as
        x = 2^y = 2^v + 2^-1 + 2^-2 + ...
        
        2^x = 2^(2^v + 2^-1 + 2^-2 + ...) = 2^major · 2^(2^-1) · 2^(2^-2) · ...
        2^(2^-i) is precomputed.
     @dev 
        for i in range(1, 20-1) surfs over the 63 to 0 bits.
     @param x uint256, X64
     @return uint256, X64 as 2**(x/2**64)*2**64
    """
    
    # Get major of x
    major_x: uint256 = shift(x, -64)
    assert major_x < 192

    # 2^(2^(-i)) * 2^64, i = 1..
    # Bug in Vyper, preferably as a constant.
    # https://github.com/vyperlang/vyper/issues/2156
    TWOTWOMINUSI: uint256[24-1] = [
    26087635650665564425, 21936999301089678047, 20116317054877281742, 19263451207323153962, 18850675170876015534, 18647615946650685159, 18546908069882975960, 18496758270674070881, 18471734244850835106, 18459234930309000272, 18452988445124272033, 18449865995240371898, 18448304968436414829, 18447524504564044946, 18447134285009651015, 18446939178327825412, 18446841625760745902, 18446792849670663277, 18446768461673986097 , 18446756267687738522, 18446750170697637486, 18446747122203342655, 18446745597956384162 ] # 18446744835832952145,
    # 18446744454771247945, 18446744264240398796, 18446744168974974960, 18446744121342263227,
    # 18446744097525907406, 18446744085617729507, 18446744079663640561, 18446744076686596088,
    # 18446744075198073852, 18446744074453812734, 18446744074081682175, 18446744073895616895,
    # 18446744073802584256, 18446744073756067936, 18446744073732809776, 18446744073721180696,
    # 18446744073715366156, 18446744073712458886, 18446744073711005251, 18446744073710278433,
    # 18446744073709915024, 18446744073709733320, 18446744073709642468, 18446744073709597042,
    # 18446744073709574329, 18446744073709562973, 18446744073709557294, 18446744073709554455,
    # 18446744073709553036, 18446744073709552326, 18446744073709551971, 18446744073709551793,
    # 18446744073709551705, 18446744073709551660, 18446744073709551638, 18446744073709551627,
    # 18446744073709551622, 18446744073709551619, 18446744073709551617, 18446744073709551617 ]


    intermediate: uint256 = 2**64 
    # for i in range(1, 64-16-1):
    for i in range(1, 24-1):
        cond: uint256 = bitwise_and(x, shift(1, 64-i))
        if cond > 0:
            intermediate = shift(intermediate*TWOTWOMINUSI[i-1], -64)
        # I think the if statement is cheaper.
        # intermediate = shift(intermediate * (TWOTWOMINUSI[i-1] if cond > 0 else ONE), -64)
    
    # The major part is added here to increase the size of the number we can compute.
    return shift(intermediate, convert(major_x, int128))


@pure
@internal
def _invp2X64(x : uint256) -> uint256:
    """
    @notice
        We can write x as
        x = 2^y = 2^v + 2^-1 + 2^-2 + ...
        
        2^-x = 2^(-(2^v + 2^-1 + 2^-2 + ...)) = 2^-major · 2^(-2^-1) · 2^(-2^-2) · ...
        2^(-2^-i) is precomputed.
     @dev 
        for i in range(1, 20-1) surfs over the 63 to 0 bits.
     @param x uint256, X64
     @return uint256, X64 as 2**(-x/2**64)*2**64
    """
    
    # Get major of x
    major_x: uint256 = shift(x, -64)
    assert major_x < 41  # dev: Major larger then fixed points. Reserve a few (64-41=23) bits for accuracy

    # 2^(-2^(-i)) * 2^64, i = 1..
    # Bug in Vyper, preferably as a constant.
    # https://github.com/vyperlang/vyper/issues/2156
    TWOTWOMINUSI: uint256[19] = [13043817825332782212, 15511800964685064948, 16915738899553466670, 17664662643191237676, 18051468387014017850, 18248035989933441396, 18347121020861646923, 18396865112328554661, 18421787711448657617, 18434261669329232139, 18440501815349552981, 18443622680442407997, 18445183311048607332, 18445963675871538003, 18446353870663572145, 18446548971154807802, 18446646522174239825, 18446695297877410579, 18446719685777359790 ] #,  18446731879739425374 ] #, 18446737976723480912, 18446741025216264368, 18446742549462845018, 18446743311586182573, 18446743692647863158, 18446743883178706403, 18446743978444128763, 18446744026076840128, 18446744049893195856, 18446744061801373732, 18446744067755462673, 18446744070732507144]


    intermediate: uint256 = 2**64 
    for i in range(1, 20-1):
        cond: uint256 = bitwise_and(x, shift(1, 64-i))
        if cond > 0:
            intermediate = shift(intermediate*TWOTWOMINUSI[i-1], -64)
        # I think the if statement is cheaper.
        # intermediate = shift(intermediate * (TWOTWOMINUSI[i-1] if cond > 0 else ONE), -64)
    

    # Since we are computing 2^(-x) we are not worried about the increased since
    # the major contributes with, but with how many decimals it removes from the
    # calculation. Rather do that late, than early. 
    return shift(intermediate, -convert(major_x, int128))


@view
@internal
def _fpowX64(x : uint256, p : uint256) -> uint256:
    """
    @notice
        x^p = 2^(p · log2(x))
     @dev Depends heavily on log2 and p2. Remember that. 
     @param x uint256, X64
     @return uint256, X64
    """
    return self._p2X64(
        self.pMULX64(p, self._log2X64(x))
    )


@view
@internal
def _invfpowX64(x : uint256, p : uint256) -> uint256:
    """
    @notice
        x^p = 2^(-(p · log2(x)))
     @dev Depends heavily on log2 and invp2. Remember that. 
     @param x uint256, X64
     @return uint256, X64
    """
    return self._invp2X64(
        self.pMULX64(self._log2X64(x), p)
    )


### Swap Pool ### 


# Describes an atomic swap between the 2 tokens: _fromAsset and _toAsset.
event LocalSwap:
    _fromAsset : indexed(address)  # The asset which was sold in exchange for _toAsset
    _toAsset : indexed(address)  # The asset which was purchased with _fromAsset 
    _who : indexed(address)  # The user / exchange who facilitated the trade (msg.sender)
    _input : uint256  # The number of _fromAsset sold
    _output : uint256  # The number of tokens provided to _who
    _fees : uint256  # The pool fee. Taken from the input. 
                     # Numerical losses, also qualify as fees, are for obvious
                     # reasons not included.


# Describes the creation of an external swap: Cross-chain swap.
# If _fromAsset is the proxy contract or _toAsset is 2**8-1, the swap is a liquidity swap.
event SwapToUnits:
    _who : indexed(bytes32)  # The recipient of the trade. The person who bought the trade is not present.
    _fromAsset : indexed(address)  # The asset which was sold in exchange for _toAsset.
    _toAsset : uint256  # The token index of the asset to purchase on _toChain. 
    _toChain : uint256  # The target chain.
    _input : uint256  # The number of _fromAsset sold
    _output : uint256  # The calculated number of units bought. Will be sold to buy _toAsset
    _fees : uint256  # The pool fee. Taken from the input. 
                     # Numerical losses, also qualify as fees, are for obvious
                     # reasons not included.
    sourceSwapId : uint256  # Used to identify escrows.


# Describes the arrival of an external swap: Cross-chain swap.
# If _fromAsset is the proxy contract, the swap is a liquidity swap.
event SwapFromUnits:
    _who : indexed(address)  # The recipient of the trade.
    _toAsset : indexed(address)  # The asset which was purchased with _fromAsset 
    _input : uint256  # The number of units sent from the other chain.
    _output : uint256  # The number of tokens provided to _who


# Called upon successful swap.
event EscrowAck:
    sourceSwapId : uint256  # Used to identify escrows.


# Called upon failed swap.
event EscrowTimeout:
    sourceSwapId : uint256  # Used to identify escrows.


# Emitted on liquidity deposits.
event Deposit:
    _who : indexed(address)  # The depositor. Is credited with _mints pool tokens.
    _mints : uint256  # The number of minted pool tokens credited to _who
    _assets : uint256[NUMASSETS]  # An array of the number of deposited assets.


# Emitted on liquidity withdrawal.
event Withdraw:
    _who : indexed(address)  # The withdrawer. Is debited _burns pool tokens.
    _burns : uint256  # The number of burned pool tokens.
    _assets : uint256[NUMASSETS]  # An array of the token amounts returned


interface IBCInterface:
    def CreateConnection(channelId : bytes32, pool : bytes32, state : bool): nonpayable
    def CreateConnectionWithChain(chainId : uint256, pool : bytes32, state : bool): nonpayable
    def crossChainSwap(chainId : uint256, approx : bool, pool : bytes32, who : bytes32, C :  uint256, asset : uint256, minOut : uint256, sourceSwapId : uint256): nonpayable
    def liquiditySwap(chainId : uint256, pool : bytes32, who : bytes32, C :  uint256, fallbackAddress : address): nonpayable


struct TokenEscrow:
    _user : address
    _amount : uint256
    _token : address
    

# CHAININTERFACE: constant(address) = ZERO_ADDRESS
chaininterface: public(address)

# If the pool has no cross chain connection, this is true.
# Should not be trusted if setupMaster != ZERO_ADDRESS
onlyLocal: public(bool)

# To indicate which token is desired on the target pool,
# the _toAsset is an integer from 0 to NUMASSETS indicating
# which asset the pool should purchase with units.
tokenIndexing: public(HashMap[uint256, address])

# Liquidity reference.
balance0: public(HashMap[address, uint256])

# Escrow reference
escrowedTokens: public(HashMap[address, uint256])
escrowedFor: public(HashMap[uint256, TokenEscrow])
escrowIterator: uint256

# The token weights. Used for maintaining a non symmetric pool balance.
weight: public(HashMap[address, uint256])
sumWeights: public(uint256)

# The pool fee in X64. Implementation of fee: self.pMULX64(_amount, self.poolFeeX64)
poolFeeX64: uint256

# The setupMaster is the short-term owner of the pool.
# They can connect the pool to pools on other chains.
setupMaster: public(address)

# Messaging router limit #
# The router is not completely trusted. Some limits are
# imposed on the DECAYRATE-ly unidirectional liquidity flow. That is:
# if the pool observes more than self.max_unit_inflow of incoming 
# units, then it will not accept further volume. This means the router
# can only drain a prefigured percentage of the pool every DECAYRATE

# Outgoing flow is subtracted incoming flow until 0.

# The max incoming liquidity flow from the router.
max_unit_inflow: public(uint256)
# max_liquidity_unit_inflow: public(uint256) = totalSupply / 2
_unit_flow: uint256
_last_change: uint256
_liquidity_flow: uint256
_last_liquidity_change: uint256


CHECK: bool
@external
def __init__():
    self.CHECK = True


@external
def setup(chaininterface : address, init_assets : address[NUMASSETS], weights : uint256[NUMASSETS], amp : uint256, name : String[16], symbol: String[8], setupMaster : address):
    """
    @notice Setup a pool. 
    @dev
        The @param amp is only used as a sanity check and needs to be set to 2**64.
        If less than NUMASSETS are used to setup the pool, let the remaining init_assets be ZERO_ADDRESS
        The unused weights can be whatever. (however, 0 is recommended.)
        The initial token amounts should have been sent to the pool before setup.
        If any token has token amount 0, the pool will never be able to have more than
        0 tokens for that token.
    """
    assert amp == ONE


    # The pool is only designed to be used by a proxy and not as a standalone.
    # as a result self.check is set to TRUE on init, to stop anyone from using
    # the pool without a proxy.
    assert not self.CHECK
    self.CHECK = True
    self.onlyLocal = True


    self.chaininterface = chaininterface
    self.setupMaster = setupMaster

    # Names the ERC20 pool token #
    self.name = name
    self.symbol = symbol
    # END ERC20 #

    it: uint256 = 0
    max_unit_inflow : uint256 = 0
    for asset in init_assets:
        # The pool supports less than NUMASSETS assets. However, the
        # array: init_assets still needs to be 3 assets long. The solution
        # is setting the unused spaces to ZERO_ADDRESS.
        if asset == ZERO_ADDRESS:
            break
        self.tokenIndexing[it] = asset
        self.weight[asset] = weights[it]
        self.sumWeights += weights[it]
        # The contract expect the tokens to have been sent to it before setup is
        # called. Make sure the pool has more than 0 tokens.
        balanceOfSelf : uint256 = ERC20(asset).balanceOf(self)

        # The maximum unit flow is \sum Weights. The value is shifted 64
        # since units are always X64.
        max_unit_inflow += shift(weights[it], 64)
        # Balance0 is set to the initial balance. There is no correct balance0
        # value and initial balance is arbitrary. However, it provides a good
        # relative value to the actual balance. (mathematical importance)
        self.balance0[asset] += balanceOfSelf

        it += 1
    
    self.max_unit_inflow = max_unit_inflow

    # Mint 1 pool token to the short-term pool owner.
    self._mint(setupMaster, 1 * ONE)


@view
@external
def getUnitCapacity() -> uint256:
    """
    @notice Returns the current cross-chain unit capacity.
    """
    MUF : uint256 = self.max_unit_inflow
    # If the current time is more than DECAYRATE since the last update
    # then the maximum unit inflow applies
    if block.timestamp > DECAYRATE + self._last_change:
        return MUF

    # The delta unit limit is: timePassed · slope = timePassed · Max/decayrate
    delta_flow : uint256 = (MUF * (block.timestamp-self._last_change)) / DECAYRATE
    if self._unit_flow <= delta_flow:
        return MUF

    # No underflow since self._unit_flow > delta_flow
    return MUF - (self._unit_flow - delta_flow)


@view
@external
def getLiquidityUnitCapacity() -> uint256:
    """
    @notice
        Returns the current cross-chain liquidity unit capacity in
        terms of the totalSupply. These are not true units but rather
        the conversion of units to pool tokens.
    """
    # A maximum liquidity unit inflow of half the totalSupply means
    # someone could drain, currentSupply=1 
    # currentSupply/2 => currentSupply_2 = 1 + 1/2 => 1/2/(1+1/2) = 1/3 of the pool.
    # through liquidity swaps.
    MUF : uint256 = self.totalSupply / 2
    # If the current time is more than DECAYRATE since the last update
    # then the maximum unit inflow applies
    if block.timestamp > DECAYRATE + self._last_liquidity_change:
        return MUF

    delta_flow : uint256 = (MUF * (block.timestamp-self._last_liquidity_change)) / DECAYRATE
    if self._liquidity_flow <= delta_flow:
        return MUF
    return MUF - (self._liquidity_flow - delta_flow)


@internal
def checkAndSetUnitCapacity(_units : uint256):
    """
    @notice
        Checks if the pool supports an inflow of _units and decreases
        unit capacity by _units.
    @param _units The number of units to check and set.
    """
    MUF : uint256 = self.max_unit_inflow
    if block.timestamp > DECAYRATE + self._last_change:
        assert _units < MUF, "Too large swap"
        # After correcting self._unit_flow it would be 0.
        # Thus the new _unit_flow is _units.
        self._unit_flow = _units
        self._last_change = block.timestamp
        return

    delta_flow : uint256 = (MUF * (block.timestamp-self._last_change)) / DECAYRATE
    self._last_change = block.timestamp  # Here purely because of optimizations. 
    # Otherwise it would have to be repeated twice. (deployment savings)
    UF : uint256 = self._unit_flow  # Used twice, in memory (potentially) saves 100 gas.
    if UF <= delta_flow:
        assert _units < MUF, "Too large swap"
        # After correcting self._unit_flow it would be 0.
        # Thus the new _unit_flow is _units.
        self._unit_flow = _units
        return
    
    newUnitFlow : uint256 = (UF + _units) - delta_flow
    assert newUnitFlow < MUF, "Large one-sided flow detected"
    self._unit_flow = newUnitFlow


@internal
def checkAndSetLiquidityCapacity(_value : uint256):
    """
    @notice 
        Checks if the pool supports an inflow of _value pool token
        and decreases unit capacity by _value pool tokens..
    @param _value The number of pool tokens to check and set.
    """
    # Allows 1/3 of the pool to be drained through liquidity swaps
    MUF : uint256 = self.totalSupply / 2
    if block.timestamp > DECAYRATE + self._last_liquidity_change:
        assert _value < MUF, "Too large swap"
        # After correction would be 0, set to _value.
        self._liquidity_flow = _value
        self._last_liquidity_change = block.timestamp
        return

    delta_flow : uint256 = (MUF * (block.timestamp-self._last_liquidity_change)) / DECAYRATE
    self._last_liquidity_change = block.timestamp  # Optimizations
    UF : uint256 = self._liquidity_flow
    if UF < delta_flow:
        assert _value < MUF, "Too large swap"
        # After correction would be 0, set to _value.
        self._liquidity_flow = _value
        return
    
    newUnitFlow : uint256 = (UF + _value) - delta_flow
    assert newUnitFlow < MUF, "Large one-sided flow detected"
    self._liquidity_flow = newUnitFlow


@external
def createConnection(_channelId : bytes32, _poolReceiving : bytes32, _state : bool):
    """
    @notice
        Creates a connection to the pool _poolReceiving on the channel _channelId.
    @dev 
        if _poolReceiving is an EVM pool, it can be computes as:
        Vyper: convert(_poolAddress, bytes32)
        Solidity: abi.encode(_poolAddress)
        Brownie: brownie.convert.to_bytes(_poolAddress, type_str="bytes32")
        ! Notice, using tx.origin is not secure.
        However, it makes it easy to bundle call from an external contract
        and no assets are at risk because the pool should not be used without
        setupMaster == ZERO_ADDRESS
    @param _channelId The _channelId of the target pool.
    @param _poolReceiving The bytes32 representation of the target pool
    @param _state  # todo: Should we also set this to True? Aka: should we disable disabling a connection?
    """
    assert (tx.origin == self.setupMaster) or (msg.sender == self.setupMaster)  # ! tx.origin ! Read @dev.
    self.onlyLocal = False

    IBCInterface(self.chaininterface).CreateConnection(_channelId, _poolReceiving, _state)


@external
def createConnectionWithChain(_chainId : uint256, _poolReceiving : bytes32, _state : bool):
    """
    @notice Creates a connection to the pool _poolReceiving using the lookup table of the interface.
    @dev 
        if _poolReceiving is an EVM pool, it can be computes as:
        Vyper: convert(_poolAddress, bytes32)
        Solidity: abi.encode(_poolAddress)
        Brownie: brownie.convert.to_bytes(_poolAddress, type_str="bytes32")
        ! Notice, using tx.origin is not secure.
        However, it makes it easy to bundle call from an external contract
        and no assets are at risk because the pool should not be used without
        setupMaster == ZERO_ADDRESS
    @param _chainId 
        The _chainId of the target pool. The interface will convert the chainId
        to the correct channelId.
    @param _poolReceiving The bytes32 representation of the target pool
    @param _state  # todo: Should we also set this to True? Aka: should we disable disabling a connection?
    """
    assert (tx.origin == self.setupMaster) or (msg.sender == self.setupMaster)  # ! tx.origin ! Read @dev.
    self.onlyLocal = False

    IBCInterface(self.chaininterface).CreateConnectionWithChain(_chainId, _poolReceiving, _state)


@external
def finishSetup():
    """
    @notice
        Gives up short term ownership of the pool. This makes the pool unstoppable.
    @dev 
        ! Notice, using tx.origin is not secure.
        However, it makes it easy to bundle call from an external contract
        and no assets are at risk because the pool should not be used without
        setupMaster == ZERO_ADDRESS
    """
    assert (tx.origin == self.setupMaster) or (msg.sender == self.setupMaster) # ! tx.origin ! Read @dev.
    
    # Disable cross-chain interactions if the pool has no connections.
    if self.onlyLocal == True:
        self.chaininterface = ZERO_ADDRESS

    self.setupMaster = ZERO_ADDRESS


@view
@external
def ready() -> bool:
    """
    @notice
        External view function purely used to signal if a pool is safe to use.
    @dev 
        Just checks if the setup master has been set to ZERO_ADDRESS. 
        In other words, has finishSetup been called?
    """
    return self.setupMaster == ZERO_ADDRESS


### Swap Integrals ### 

@view
@internal
def _compute_integral(_in : uint256, _A : uint256, _W : uint256) -> uint256:
    """
    @notice
        Computes the integral \int_{A}^{A+in} W/(w · ln(2)) dw
            = W ln(A_2/A_1)
        The value is returned as units, which is always in X64.
    @dev 
        All input amounts should be the raw numbers and not X64.
        Since units are always denominated in X64, the function
        should be treated as mathematically *native*.
    @param _in The input amount.
    @param _A The current pool balance.
    @param _W The pool weight of the token.
    @return Group specific units in X64 (units are **always** X64).
    """

    # Notice, _A + _in and _A are not X64 but self.bigdiv64 is used anyway.
    # That is because _log2X64 requires an X64 number.
    # Shifting _A + _in and _A before dividing returns:
    # ((_A + _in) * 2**64) / ((_A) * 2**64) * 2**64 = (_A + _in) / _A * 2**64
    # Thus the shifting cancels and is not needed.
    return _W * self._log2X64(self.bigdiv64(_A + _in, _A))


@external
def compute_integral(_in : uint256, _A : uint256, _W : uint256) -> uint256:
    return self._compute_integral(_in, _A, _W)


@view
@internal
def _approx_compute_integral(_in : uint256, _A : uint256, _W : uint256) -> uint256:
    """
    @notice
        Computes the integral \int_{A}^{A+in} W/(w · ln(2)) dw by lower approximation.
        After swapping, we know the price to be W/((_A + _in) · ln(2)). The amount provided
        to the seller is then (W · _in)/((_A + _in) · ln(2)).

        The value is returned as units, which is always in X64.
    @dev 
        This function should sometimes be used over _compute_integral
        if _in/_A <= 0.1% since it is cheaper and mathematically simpler.
        All input amounts should be the raw numbers and not X64.
        Since units are always denominated in X64, the function
        should be treated as mathematically *native*.
    @param _in The input amount.
    @param _A The current pool balance of the _in token.
    @param _W The pool weight of the _in token.
    @return Group specific units in X64 (units are **always** X64).
    """
    return self.bigdiv64(shift(_W * _in, 64), ((_A + _in) * LN2))


@view
@internal
def _solve_integral(_U : uint256, _B : uint256, _W : uint256) -> uint256:
    """
    @notice
        Solves the equation U = \int_{A-_out}^{A} W/(w · ln(2)) dw for _out
            = B_1 · (1 - 2^(-U/W_0))

        The value is returned as output token.
    @dev 
        All input amounts should be the raw numbers and not X64.
        Since units are always denominated in X64, the function
        should be treated as mathematically *native*.
    @param _U Input units. Is technically X64 but can be treated as not.
    @param _B The current pool balance of the _out token.
    @param _W The pool weight of the _out token.
    @return Output denominated in output token. 
    """
    return shift(_B * (ONE - self._invp2X64(_U/_W)), -64)


@external
def solve_integral(_U : uint256, _B : uint256, _W : uint256) -> uint256:
    return self._solve_integral(_U, _B, _W)


@view
@internal
def _approx_solve_integral(_U : uint256, _B : uint256, _W : uint256) -> uint256:
    """
    @notice
        Solves the equation U = \int_{_B-_out}^{_B} W/(w · ln(2)) dw for _out
        by upper approximation. After swapping, we know the price to be W/((_B - out) · ln(2)).
        The amount provided to the seller is then (W · _out)/((_A - _out) · ln(2)).
            => _out = (_B · _U · LN2)/(_W + U · LN2)
    @dev 
        This function should sometimes be used over _solve_integral.
        Since the error is relative to _out and _B, it is difficult
        to provide relative bounds when the approximation is better
        than the true equation.
        All input amounts should be the raw numbers and not X64.
        Since units are always denominated in X64, the function
        should be treated as mathematically *native*.
    @param _U Input units. Is technically X64 but can be treated as not.
    @param _B The current pool balance of the _out token.
    @param _W The pool weight of the _out token.
    @return Output denominated in output token.
    """
    UnitTimesLN2 : uint256 = self.pMULX64(_U, LN2)
    return self.bigdiv64(_B * UnitTimesLN2, shift(_W, 64) + UnitTimesLN2)


@view
@internal
def _complete_integral(_in : uint256, _A : uint256, _B : uint256, _W_A : uint256, _W_B : uint256) -> uint256:
    """
    @notice
        Solves the equation 
            \int_{A}^{A + _in} W_A/(w · ln(2)) dw = \int_{B-_out}^{B} W_B/(w · ln(2)) dw for _out
                => out = B · (1 - ((A+in)/A)^(-W_A/W_B))

        Alternatively, the integral can be computed through:
            _solve_integral(_compute_integral(_in, _A, _W_A), _B, _W_B).
            However, _complete_integral is very slightly cheaper since it delays a division.
            (Apart from that, the mathematical operations are the same.)
    @dev 
        All input amounts should be the raw numbers and not X64.
    @param _in The input amount.
    @param _A The current pool balance of the _in token.
    @param _B The current pool balance of the _out token.
    @param _W_A The pool weight of the _in token.
    @param _W_B The pool weight of the _out token.
    @return Output denominated in output token.
    """
    # (A+in)/A >= 1 as in >= 0. As a result, invfpow should be used.
    # Notice, bigdiv64 is used twice on value not x64. This is because a x64
    # shifted valued is required for invfpow in both arguments.
    U: uint256 = ONE - self._invfpowX64(self.bigdiv64(_A + _in, _A), self.bigdiv64(_W_A, _W_B))
    return shift(_B * U, -64)
    

@view
@internal
def _approx_integral(_in : uint256, _A : uint256, _B : uint256, _W_A : uint256, _W_B : uint256) -> uint256:
    """
    @notice
        Solves the equation 
            \int_{A}^{A + _in} W_A/(w · ln(2)) dw = \int_{B-_out}^{B} W_B/(w · ln(2)) dw for _out
        by approximation. For the mathematical explanation, see _approx_solve_integral
        and _approx_compute_integral.
            => out = (_B · _W_A · _in)/(_W_B · _A + (_W_A + _W_B) · _in)
        
        Alternatively, the integral can be computed through:
            _approx_solve_integral(_approx_compute_integral(_in, _A, _W_A), _B, _W_B).
        However, this approximation never uses X64 numbers which makes it slightly cheaper.
    @dev 
        This function never use any X64 mathematics.
    @param _in The input amount.
    @param _A The current pool balance of the _in token.
    @param _B The current pool balance of the _out token.
    @param _W_A The pool weight of the _in token.
    @param _W_B The pool weight of the _out token.
    @return Output denominated in output token.
    """
    return (_B * _W_A * _in)/(_W_B * _A + (_W_A + _W_B) * _in)


@view
@internal
def _dry_swap_to_unit(_from : address, _amount : uint256, approx : bool) -> uint256:
    """
    @notice
        Computes the return of a SwapToUnits, without executing one.
    @param _from The address of the token to sell.
    @param _amount The amount of _from token to sell.
    @param approx True if SwapToUnits should be approximated (read: _approx_compute_integral)
    @return Group specific units in X64 (units are **always** X64).
    """
    A: uint256 = ERC20(_from).balanceOf(self) 
    W: uint256 = self.weight[_from]

    if approx:
        return self._approx_compute_integral(_amount, A, W)

    return self._compute_integral(_amount, A, W)


@view
@external
def dry_swap_to_unit(_from : address, _amount : uint256, approx : bool) -> uint256:
    """
    @notice
        Computes the return of a SwapToUnits, without executing one.
    @dev
        Before letting a user swap, it can be beneficial to viewing the
        return approximated and not approximated. Then choose the one with
        the lowest cost (max return & min gas cost).
    @param _from The address of the token to sell.
    @param _amount The amount of _from token to sell.
    @param approx True if SwapToUnits should be approximated (read: _approx_compute_integral)
    @return Group specific units in X64 (units are **always** X64).
    """
    return self._dry_swap_to_unit(_from, _amount, approx)


@view
@internal
def _dry_swap_from_unit(_to : address, U : uint256, approx : bool) -> uint256:
    """
    @notice
        Computes the output of a SwapFromUnits, without executing one.
    @param _to The address of the token to buy.
    @param U The number of units used to buy _to.
    @param approx True if SwapToUnits should be approximated (read: _approx_solve_integral)
    @return Output denominated in output token.
    """
    B: uint256 = ERC20(_to).balanceOf(self) - self.escrowedTokens[_to]
    W: uint256 = self.weight[_to]

    if approx:
        return  self._approx_solve_integral(U, B, W)

    return self._solve_integral(U, B, W)


@view
@external
def dry_swap_from_unit(_to : address, U : uint256, approx : bool) -> uint256:
    """
    @notice
        Computes the output of a SwapFromUnits, without executing one.
    @dev
        Before letting a user swap, it can be beneficial to viewing the
        return approximated and not approximated. Then choose the one with
        the lowest cost (max return & min gas cost).
    @param _to The address of the token to buy.
    @param U The number of units used to buy _to.
    @param approx True if SwapToUnits should be approximated (read: _approx_solve_integral)
    @return Output denominated in _to token.
    """
    return self._dry_swap_from_unit(_to, U, approx)


@view
@internal
def _dry_swap_both(_from : address, _to : address, _in: uint256, approx : bool) -> uint256:
    """
    @notice
        Computes the return of a SwapToAndFromUnits, without executing one.
    @dev
        If the pool weights of the 2 tokens are equal, a very simple curve
        is used and argument approx is ignored.
    @param _from The address of the token to sell.
    @param _to The address of the token to buy.
    @param _in The amount of _from token to sell for _to token.
    @param approx 
        True if SwapToUnits should be approximated (read: _approx_compute_integral)
        Is ignored if the tokens weights are equal.
    @return Output denominated in _to token.
    """
    A: uint256 = ERC20(_from).balanceOf(self)
    B: uint256 = ERC20(_to).balanceOf(self) - self.escrowedTokens[_to]
    W_A: uint256 = self.weight[_from]
    W_B: uint256 = self.weight[_to]

    # The swap equation simplifies to the ordinary constant product if the
    # token weights are equal. This equation is even simpler than approx.
    if W_A == W_B:  # Saves ~7500 gas.
        return (B * _in) / (A + _in)

    if approx:
        return self._approx_integral(
            _in, A, B, W_A, W_B
        )
    
    return self._complete_integral(
        _in, A, B, W_A, W_B
    )


@view
@external
def dry_swap_both(_from : address, _to : address, _amount: uint256, approx : bool) -> uint256:
    """
    @notice
        Computes the return of a SwapToAndFromUnits, without executing one.
    @dev
        Before letting a user swap, it can be beneficial to viewing the
        return approximated and not approximated. Then choose the one with
        the lowest cost (max return & min gas cost).

        If the pool weights of the 2 tokens are equal, a very simple curve
        is used and argument approx is ignored..
    @param _from The address of the token to sell.
    @param _to The address of the token to buy.
    @param _amount The amount of _from token to sell for _to token.
    @param approx 
        True if SwapToUnits should be approximated (read: _approx_compute_integral)
        Is ignored if the tokens weights are equal.
    @return Output denominated in _to token.
    """
    return self._dry_swap_both(_from, _to, _amount, approx)


# @view
# @internal
# def _liquidity_equation_tk(_asset : address, poolTokens : uint256) -> uint256:
#     """
#     @notice
#         Returns the number of tokens needed to mint a certain number of pool tokens.
        
#         Solves \int_{A_{0}}^{A_{t}}P \ \left(w\right)d w = \int_{A_{0} +pt}^{A_{t} +tk}P \ \left( w\right)d w
#         for tk.
#     @param _asset The token address used as underlying for deposit.
#     @param poolTokens The number of _asset specific pool tokens to mint.
#     @return Number of tokens required to mint poolTokens.
#     """
#     At: uint256 = ERC20(_asset).balanceOf(self)
#     A0: uint256 = self.balance0[_asset]
#     if At == A0:
#         return poolTokens
    
#     return (At * poolTokens)/A0


# TODO Implement a dry deposit function?


@external
@nonreentrant('lock')
def depositAll(_baseAmount : uint256):
    """
    @notice
        Deposits a symmetrical number of tokens such that
        _baseAmount of pool tokens are minted.
        This doesn't change the pool price.
    @dev
        Requires approvals for all tokens within the pool.
    @param _baseAmount The number of pool tokens to mint.
    """
    # Update the liquidity security limit. Since the limit is based on the current
    # totalSupply, changing the totalSupply upwards by depositing changes the 
    # limit.
    self.checkAndSetLiquidityCapacity(0)

    # Cache totalSupply. This saves up to ~200 gas.
    initial_totalSupply : uint256 = self.totalSupply

    # For later event logging, the amounts transferred to the pool are stored.
    amounts: uint256[NUMASSETS] = empty(uint256[NUMASSETS])
    for asset_num in range(NUMASSETS):
        asset : address = self.tokenIndexing[asset_num]
        if asset == ZERO_ADDRESS:
            break
        
        # Deposits should returns less, so the escrowed tokens are not subtracted.
        At: uint256 = ERC20(asset).balanceOf(self)
        A0: uint256 = self.balance0[asset]
        # The number of pool tokens (balance0s) owned by 1 pool tokens is 
        # self.balance0[asset]/self.totalSupply. _baseAmount owns
        # self.balance0[asset]/self.totalSupply · _baseAmount
        # Alternatively, to mint _baseAmount poolTokens, _baseAmount/self.totalSupply
        # of the existing liquidity needs to be provided. That is:
        # _baseAmount/self.totalSupply · self.balance0[asset]
        pt_Token : uint256 = (A0 * _baseAmount)/initial_totalSupply

        # Find the number of tokens required to mint pt_Token balance0s.
        base: uint256 = pt_Token  # = _liquidity_equation_tk(asset, pt_Token)
        if At != A0:
            base = (At * pt_Token)/A0

        # Increase the balance0 by the newly minted amount.
        self.balance0[asset] += pt_Token

        # Transfer the appropriate number of pool tokens from the user
        # to the pool. (And store for event logging) 
        amounts[asset_num] = base
        self.safeTransferFrom(asset, msg.sender, self, base)  #dev: User doesn't have enough tokens

    # Mint the desired number of pool tokens to the user.
    self._mint(msg.sender, _baseAmount)

    # Emit the event
    log Deposit(msg.sender, _baseAmount, amounts)


@external
@nonreentrant('lock')
def withdrawAll(_baseAmount : uint256):
    """
    @notice
        Burns _baseAmount and releases the symmetrical share
        of tokens to the burner.
        This doesn't change the pool price.
    @param _baseAmount The number of pool tokens to burn.
    """
    # Update the liquidity security limit. Since the limit is based on the current
    # totalSupply, changing the totalSupply upwards by depositing changes the 
    # limit.
    self.checkAndSetLiquidityCapacity(0)

    # cache totalSupply. This saves up to ~200 gas.
    initial_totalSupply : uint256 = self.totalSupply

    # Since we have already cached totalSupply, we might as well burn the tokens
    # now. If the user doesn't have enough tokens, they save a bit of gas.
    self._burn(msg.sender, _baseAmount)

    # For later event logging, the amounts transferred to the pool are stored.
    amounts: uint256[NUMASSETS] = empty(uint256[NUMASSETS])
    for asset_num in range(NUMASSETS):
        asset : address = self.tokenIndexing[asset_num]
        if asset == ZERO_ADDRESS:
            break
        
        # Withdrawals should returns less, so the escrowed tokens are subtracted.
        At: uint256 = ERC20(asset).balanceOf(self) - self.escrowedTokens[asset]
        A0: uint256 = self.balance0[asset]

        # Read comment in depositAll. The user's share of the pool is found.
        pt_Token : uint256 = (A0 * _baseAmount)/initial_totalSupply

        # Number of tokens which can be released given pt_Token pool tokens.
        base: uint256 = pt_Token  # = _liquidity_equation_tk(asset, pt_Token)
        if At != A0:
            base = (At * pt_Token)/A0

        # Remove the pool tokens from balance0.
        self.balance0[asset] -= pt_Token

        # Transferring of the released tokens.
        amounts[asset_num] = base
        self.safeTransfer(asset, msg.sender, base)
    
    # Emit the event
    log Withdraw(msg.sender, _baseAmount, amounts)



@external
@nonreentrant('lock')
def localswap(_fromAsset : address, _toAsset : address, _amount : uint256, minOut: uint256, approx : bool = False) -> uint256:
    """
    @notice
        A swap between 2 assets which both are inside the pool. Is atomic.
    @param _fromAsset The asset the user wants to sell.
    @param _toAsset The asset the user wants to buy
    @param _amount The amount of _fromAsset the user wants to sell
    @param minOut The minimum output of _toAsset the user wants.
    @param approx 
        If true, uses (worse) but simpler swapping which can improve swap
        return and gas costs. If assets weights are equal, this is ignored.
    """

    fee: uint256 = self.pMULX64(_amount, self.poolFeeX64)

    # Calculate the swap return value.
    out: uint256 = self._dry_swap_both(_fromAsset, _toAsset, _amount - fee, approx)

    # Check if the calculated returned value is more than the minimum output.
    assert out >= minOut, "Not enough returned"

    # Swap tokens with the user.
    self.safeTransfer(_toAsset, msg.sender, out)
    self.safeTransferFrom(_fromAsset, msg.sender, self, _amount)

    log LocalSwap(_fromAsset, _toAsset, msg.sender, _amount, out, fee)

    return out


@external
@nonreentrant('lock')
def swapToUnits(_chain : uint256, _targetPool : bytes32, _fromAsset : address, _toAsset : uint256, _who : bytes32, _amount : uint256, _minOut : uint256, fallbackUser : address = msg.sender, approxFrom : bool = False, approxTo : bool = False) -> uint256:
    """
    @notice
        Initiate a cross-chain swap by purchasing units and transfer them to another pool.
    @param _chain The target chain. Will be converted by the interface to channelId.
    @param _targetPool 
        The target pool on the target chain encoded in bytes32. 
        For EVM chains this can be computed as:
            Vyper: convert(_poolAddress, bytes32)
            Solidity: abi.encode(_poolAddress)
            Brownie: brownie.convert.to_bytes(_poolAddress, type_str="bytes32")
    @param _fromAsset The asset the user wants to sell.
    @param _toAsset The index of the asset the user wants to buy in the target pool.
    @param _who 
        The recipient of the transaction on _chain. Encoded in bytes32.
        For EVM chains it can be found similarly to _targetPool.
    @param _amount The number of _fromAsset to sell to the pool.
    @param fallbackUser 
        If the transaction fails send the escrowed funds to this address
    @param approxFrom Should SwapToUnits be computed by approximation?
    @param approxTo Should SwapFromUnits be computed using approximation?
    @dev 
        Use the appropriate dry swaps to decide if approximation makes sense.
        These are the same functions as used by the swap functions, so they will
        accurately predict the gas cost and swap return.
    """

    fee: uint256 = self.pMULX64(_amount, self.poolFeeX64)

    # Calculate the group specific units bought.
    U: uint256 = self._dry_swap_to_unit(_fromAsset, _amount - fee, approxFrom)

    # Create the escrow identifier
    sourceSwapId : uint256 = self.escrowIterator % 2**(32)
    self.escrowIterator += 1

    # Send the purchased units to _targetPool on _chain.
    IBCInterface(self.chaininterface).crossChainSwap(_chain, approxTo, _targetPool, _who, U, _toAsset, _minOut, sourceSwapId)
    # Collect the tokens from the user.
    self.safeTransferFrom(_fromAsset, msg.sender, self, _amount)

    # Escrow the tokens
    self.escrowedTokens[_fromAsset] += _amount
    self.escrowedFor[sourceSwapId] = TokenEscrow({ _user: fallbackUser, _amount: _amount, _token: _fromAsset})

    # Incoming swaps should be subtracted from the unit flow.
    # It is assumed if the router was fraudulent, that no-one would execute a trade.
    # As a result, if people swap into the pool, we should expect that there is exactly
    # the inswapped amount of trust in the pool.
    # If this wasn't implemented, there would be a maximum daily cross chain volume,
    # which is bad for liquidity providers.
    UF : uint256 = self._unit_flow
    # If UF < U and we do UF - U < 0 underflow => bad.
    if UF > U:
        self._unit_flow -= U
    elif UF != 0:  # Save ~100 gas if UF = 0.
        self._unit_flow = 0

    log SwapToUnits(_who, _fromAsset, _toAsset, _chain, _amount, U, fee, sourceSwapId)

    return U


@external
def releaseEscrowACK(sourceSwapId : uint256):
    """
    @notice Release the escrowed tokens into the pool.
    """
    assert msg.sender == self.chaininterface
    escrowInformation : TokenEscrow = self.escrowedFor[sourceSwapId]
    self.escrowedTokens[escrowInformation._token] -= escrowInformation._amount
    self.escrowedFor[sourceSwapId] = empty(TokenEscrow)

    log EscrowAck(sourceSwapId)


@external
def releaseEscrowTIMEOUT(sourceSwapId : uint256):
    """
    @notice Returned the escrowed tokens to the user
    """
    assert msg.sender == self.chaininterface
    escrowInformation : TokenEscrow = self.escrowedFor[sourceSwapId]
    self.escrowedTokens[escrowInformation._token] -= escrowInformation._amount
    self.escrowedFor[sourceSwapId] = empty(TokenEscrow)

    self.safeTransfer(escrowInformation._token, escrowInformation._user, escrowInformation._amount)

    log EscrowTimeout(sourceSwapId)


@external
@nonreentrant('lock')
def swapFromUnits(_toAsset : uint256, _who : address, _U : uint256, approx : bool = False) -> uint256:
    """
    @notice
        Completes a cross-chain swap by converting units to the desired token (_toAsset)
        Called exclusively by the chaininterface.
    @dev
        Can only be called by the chaininterface, as there is no way to check the
        validity of units.
    @param _toAsset Index of the asset to be purchased with _U units.
    @param _who The recipient of _toAsset
    @param _U Number of units to convert into _toAsset.
    @param approx If the swap approximation should be used over the "true" swap.
    """
    # The chaininterface is the only valid caller of this function, as there cannot
    # be a check of _U. (It is purely a number)
    assert msg.sender == self.chaininterface
    # Convert the asset index (_toAsset) into the asset to be purchased.
    toAsset: address = self.tokenIndexing[_toAsset]

    # Check if the swap is according to the swap limits
    self.checkAndSetUnitCapacity(_U)

    # Calculate the swap return value.
    purchasedTokens: uint256 = self._dry_swap_from_unit(toAsset, _U, approx)
    # Send the return value to the user.
    self.safeTransfer(toAsset, _who, purchasedTokens)

    log SwapFromUnits(_who, toAsset, _U, purchasedTokens)

    return purchasedTokens  # Unused.


### Liquidity swapping ### 
# Because of the wap pool tokens work in a group of pools, there
# needs to be a way to manage an equilibrium between pool token
# value and token pool value.


@view
@internal
def _compute_liquidity(_in : uint256, _A : uint256, _W : uint256) -> uint256:
    """
    @notice
        Computes the integral \int_{A-in}^{A} W/(w · ln(2)) dw
            = - W ln(A/(A - in))
        For optimizations, U is always assumed to be negative for liquidity units.
        Thus the function computes: W ln(A/(A - in))
        The value is returned as negative liquidity units, which is always in X64.
    @dev 
        All input amounts should be the raw numbers and not X64.
        Since units are always denominated in X64, the function
        should be treated as mathematically *native*.
    @param _in The input amount.
    @param _A The current pool balance0.
    @param _W The pool weight of the token.
    @return Group specific liquidity units in X64 (units are **always** X64).
    """

    # Notice, _A - _in and _A are not X64 but self.bigdiv64 is used anyway.
    # That is because _log2X64 requires an X64 number.
    # Shifting _A - _in and _A before dividing returns:
    # ((_A) * 2**64) / ((_A - _in) * 2**64) * 2**64 = _A / (_A - _in) * 2**64
    # Thus the shifting cancels and is not needed.
    return _W * self._log2X64(self.bigdiv64(_A, _A - _in))


@view
@internal
def _solve_liquidity(_U : uint256, _B : uint256, _WSUM : uint256) -> uint256:
    """
    @notice
        Solves the equation U = \int_{A}^{A+_out} W/(w · ln(2)) dw for _out
            = B_1 · (2^-U/W) - 1). Since _U is negative units, the
            function actually computes:  B_1 · (2^_U/W) - 1)

        The value is returned as output pool token.
    @dev 
        All input amounts should be the raw numbers and not X64.
        Since units are always denominated in X64, the function
        should be treated as mathematically *native*.
    @param _U Input negative liquidity units. Is technically X64 but can be treated as not.
    @param _B The current pool balance0 of the _out token.
    @param _W The pool weight of the _out token.
    @return Output denominated in output pool token. 
    """
    return shift(_B * (self._p2X64(_U/_WSUM) - ONE), -64)


@view
@internal
def _dry_liquidity_to_unit(_from : address, _amount : uint256) -> uint256:
    """
    @notice
        Computes the return of a liquidity swap from liquidity into units, without executing one.
    @param _from The address of the liquidity to sell.
    @param _amount The amount of _from liquidity to sell.
    # TODO: Approx?
    @return Liquidity units in X64 (units are **always** X64).
    """
    A: uint256 = self.balance0[_from]
    W: uint256 = self.weight[_from]
    return self._compute_liquidity(_amount, A, W)


@view
@external
def dry_liquidity_to_unit(_from : address, _amount : uint256) -> uint256:
    """
    @notice
        Computes the return of a liquidity swap from liquidity into units, without executing one.
    @param _from The address of the liquidity to sell.
    @param _amount The amount of _from liquidity to sell.
    # TODO: Approx?
    @return Liquidity units in X64 (units are **always** X64).
    """
    return self._dry_liquidity_to_unit(_from, _amount)


@view
@internal
def _dry_liquidity_from_unit(_to : address, U : uint256, _WSUM : uint256) -> uint256:
    """
    @notice
        Computes the output of a liquidity swap from units into liquidity, without executing one.
    @param _to The token address of the target balance0.
    @param U The number of units used to buy liquidity of _to.
    @param _WSUM The weighted sum used to solve the liquidity constraints.
    # TODO: Approx?
    @return Output denominated in balance0s.
    """
    B: uint256 = self.balance0[_to]

    return self._solve_liquidity(U, B, _WSUM)


@view
@external
def dry_liquidity_from_unit(_to : address, U : uint256, _WSUM : uint256) -> uint256:
    """
    @notice
        Computes the output of a liquidity swap from units into liquidity, without executing one.
    @param _to The token address of the target balance0.
    @param U The number of units used to buy liquidity of _to.
    @param _WSUM The weighted sum used to solve the liquidity constraints.
    @return Output denominated in balance0s.
    """
    return self._dry_liquidity_from_unit(_to, U, _WSUM)


@external
@nonreentrant('lock')
def outLiquidity(_chain : uint256, _targetPool : bytes32, _who : bytes32, _baseAmount : uint256) -> uint256:
    """
    @notice
        Initiate a cross-chain liquidity swap by lowering liquidity and transfer
        the liquidity units to another pool.
    @param _chain The target chain. Will be converted by the interface to channelId.
    @param _targetPool 
        The target pool on the target chain encoded in bytes32. 
        For EVM chains this can be computed as:
            Vyper: convert(_poolAddress, bytes32)
            Solidity: abi.encode(_poolAddress)
            Brownie: brownie.convert.to_bytes(_poolAddress, type_str="bytes32")
    @param _who 
        The recipient of the transaction on _chain. Encoded in bytes32.
        For EVM chains it can be found similarly to _targetPool.
    @param _baseAmount The number of pool tokens to liquidity Swap
    """
    
    # Cache totalSupply. This saves up to ~200 gas.
    initial_totalSupply : uint256 = self.totalSupply

    # Since we have already cached totalSupply, we might as well burn the tokens
    # now. If the user doesn't have enough tokens, they save a bit of gas.
    self._burn(msg.sender, _baseAmount)

    # The number of units corresponding gained by lowering _baseAmount liquidity
    # Should be found. This will be done by looping over each conversion.
    U: uint256 = 0
    for i in range(NUMASSETS):
        token : address = self.tokenIndexing[i]
        if token == ZERO_ADDRESS:
            break

        # The pool ownership given up, converted to balance0s (given up)
        pt_Token : uint256 = (self.balance0[token] * _baseAmount)/initial_totalSupply
        # Swapping to units.
        U += self._dry_liquidity_to_unit(token, pt_Token)
        # Subtracting the swapped units.
        self.balance0[token] -= pt_Token

    # Sending the liquidity units over.
    IBCInterface(self.chaininterface).liquiditySwap(_chain, _targetPool, _who, U, msg.sender)

    # Correcting the routing security limit. (To increase maximum daily volume)
    LUF : uint256 = self._liquidity_flow
    if LUF > _baseAmount:
        self._liquidity_flow -= _baseAmount
    elif LUF != 0:
        self._liquidity_flow = 0

    # The SwapToUnits event is reused.  # TODO: Separate event?
    log SwapToUnits(_who, self, 2**8-1, _chain, _baseAmount, U, 0, 0)  # TODO: sourceSwapId

    return U


@external
@nonreentrant('lock')
def inLiquidity(_who : address,  _U : uint256) -> uint256:
    """
    @notice
        Completes a cross-chain swap by converting liquidity units to pool tokens
        Called exclusively by the chaininterface.
    @dev
        Can only be called by the chaininterface, as there is no way to check the
        validity of units.
    @param _who The recipient of pool tokens
    @param _U Number of units to convert into pool tokens.
    """
    # The chaininterface is the only valid caller of this function, as there cannot
    # be a check of _U. (It is purely a number)
    assert msg.sender == self.chaininterface

    # ratios of the balance0s should remain mostly unchanged. As a result, the
    # units should be used to buy pool tokens in roughly the same amount.
    # Since it is independent of the current balance, we cache it.
    WSUM : uint256 = self.sumWeights  # Is not X64.

    token0 : address = self.tokenIndexing[0]
    # Because of the balance0 constraint of wrapped pool tokens,
    # only the first balance0 change needs to be computed.
    token0B0 : uint256 = self._dry_liquidity_from_unit(token0, _U, WSUM)

    ts : uint256 = self.totalSupply
    # Token0B0 * _baseAmount = self.balance0
    # To increase the resolution, the balance0s are converted into pool tokens.
    # Since 2**64 is the reference liquidity and 2**64 is a lot larger than most token
    # amounts, it is assumed that it provides a better resolution. (for loop)
    poolTokens : uint256 = (token0B0*ts)/self.balance0[token0]
    self.balance0[token0] += token0B0


    for i in range(1, NUMASSETS):
        token : address = self.tokenIndexing[i]
        if token == ZERO_ADDRESS:
            break
        # The pool token is used as reference, since it providers a higher resolution
        # than the balance0 constraint.
        self.balance0[token] += (self.balance0[token] * poolTokens)/ts
        # Alternative based on balance0 constraint  <- Lower resolution
        # self.balance0[token] += (token0B0*self.balance0[token])/self.balance0[token0]

    # Check if the swap honors the security limit.
    self.checkAndSetLiquidityCapacity(poolTokens)

    # Mint pool tokens for _who
    self._mint(_who, poolTokens)

    # The SwapToUnits event is reused.  # TODO: Separate event?
    log SwapFromUnits(_who, self, _U, poolTokens)

    return poolTokens
