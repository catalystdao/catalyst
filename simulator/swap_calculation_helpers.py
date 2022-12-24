from math import log2

from integer import TInt, TUint, Uint256

from fixed_point_math import LN2, binomial_expansion_neg_pow_x64, div_x64, inv_pow2_x64, inv_pow_x64, log2_x64, mul_x64, pow2_x64, pow_x64

ONE_X64  = Uint256(2**64)
ONE_X128 = Uint256(2**128)


# Deposits and Withdrawals ******************************************************************************************************

def calc_asset_amount_for_pool_tokens_i(pool_token_balance: TUint, asset_balance: TUint, asset_eq_balance: TUint) -> TUint:
    if asset_balance == asset_eq_balance:
        return pool_token_balance
    
    return (asset_balance * pool_token_balance) / asset_eq_balance

def calc_asset_amount_for_pool_tokens_f(pool_token_balance: float, asset_balance: float, asset_eq_balance: float) -> float:
    if asset_balance == asset_eq_balance:
        return pool_token_balance
    
    return (asset_balance * pool_token_balance) / asset_eq_balance



# Asset swaps *******************************************************************************************************************

def full_swap_i(
    input                : Uint256,     # x
    source_asset_balance : Uint256,     # At
    source_asset_weight  : Uint256,     # WA
    target_asset_balance : Uint256,     # Bt
    target_asset_weight  : Uint256,     # WB
    amplification_x64    : Uint256 | None
) -> Uint256:

    if amplification_x64 is not None:

        input_x64 = input << 64
        source_asset_balance_x64 = source_asset_balance << 64

        one_minus_amp_x64 = ONE_X64 - amplification_x64
        intermediate_x64  = target_asset_weight * pow_x64(target_asset_balance << 64, one_minus_amp_x64)

        return (
            target_asset_balance * (
                ONE_X64 - inv_pow_x64(
                    div_x64(
                        intermediate_x64,
                        intermediate_x64 - source_asset_weight * (
                            pow_x64(source_asset_balance_x64 + input_x64, one_minus_amp_x64) - pow_x64(source_asset_balance_x64, one_minus_amp_x64)
                        )
                    ),
                    div_x64(ONE_X64, one_minus_amp_x64)
                )
            )
        ) >> 64


    # Bt * (1 - (At + input) / At) ^ (-WA/WB))       NOTE: (At + input) / At >= 1 as input > 0

    if source_asset_weight == target_asset_weight:
        return (target_asset_balance * input) / (source_asset_balance + input)

    return (target_asset_balance * (
        ONE_X64 - inv_pow_x64(
            div_x64(source_asset_balance + input, source_asset_balance),
            div_x64(source_asset_weight, target_asset_weight)
        )
    )) >> 64


def full_swap_uniform_approx_i(
    input                : Uint256,     # x
    source_asset_balance : Uint256,     # At
    source_asset_weight  : Uint256,     # WA
    target_asset_balance : Uint256,     # Bt
    target_asset_weight  : Uint256,     # WB
    amplification_x64    : Uint256 | None
) -> Uint256:

    if amplification_x64 is not None:
        raise Exception("Not implemented")

    return (div_x64(
        source_asset_weight*target_asset_balance*input,
        target_asset_weight*source_asset_balance + (source_asset_weight + target_asset_weight)*input
    )) >> 64


def full_swap_binomial_approx_i(
    input                : Uint256,     # x
    source_asset_balance : Uint256,     # At
    source_asset_weight  : Uint256,     # WA
    target_asset_balance : Uint256,     # Bt
    target_asset_weight  : Uint256,     # WB
    amplification_x64    : Uint256 | None,
    rounds               : int
) -> Uint256:

    if rounds % 2 != 0:
        raise Exception("Binomial rounds must be even!")

    if amplification_x64 is not None:
        raise Exception("Not implemented")


    return (target_asset_balance * (
        ONE_X64 - binomial_expansion_neg_pow_x64(
            x = (input << 64)/source_asset_balance,
            n = div_x64(source_asset_weight, target_asset_weight),
            rounds = rounds
        )
    )) >> 64


def full_swap_f(
    input                : float,
    source_asset_balance : float,
    source_asset_weight  : float,
    target_asset_balance : float,
    target_asset_weight  : float,
    amplification        : float | None
) -> float:

    if amplification is not None:

        one_minus_amp = 1 - amplification
        intermediate = target_asset_weight * target_asset_balance**one_minus_amp

        return target_asset_balance * (
            1 - (
                intermediate/(intermediate - source_asset_weight * (
                    (source_asset_balance + input)**one_minus_amp - source_asset_balance**one_minus_amp
                ))
            )**(-1/one_minus_amp))

    return target_asset_balance * (
        1 - ((source_asset_balance + input) / source_asset_balance)**(-source_asset_weight/target_asset_weight)
    )



def out_swap_i_x64(
    input                : Uint256,  # x
    source_asset_balance : Uint256,  # At
    source_asset_weight  : Uint256,  # WA
    amplification_x64    : Uint256 | None   # k
) -> Uint256:
    # Computes the integral
    # int_{At}^{At+x} WA/(w ln(2)) dw

    if amplification_x64 is not None:

        input_x64 = input << 64
        source_asset_balance_x64 = source_asset_balance << 64

        one_minus_amp_x64 = ONE_X64 - amplification_x64

        return source_asset_weight * (
            pow_x64(source_asset_balance_x64 + input_x64, one_minus_amp_x64) - pow_x64(source_asset_balance_x64, one_minus_amp_x64)
        )
    
    # WA * log2((At + x)/At)
    return source_asset_weight * log2_x64(
        div_x64(source_asset_balance + input, source_asset_balance)
    )


def out_swap_uniform_approx_i_x64(
    input                : Uint256,  # x
    source_asset_balance : Uint256,  # At
    source_asset_weight  : Uint256,  # WA
    amplification_x64    : Uint256 | None   # k
) -> Uint256:

    if amplification_x64 is not None:
        raise Exception("Not implemented")
    
    return div_x64(
        (source_asset_weight * input) << 64,
        LN2*(source_asset_balance + input)
    )


def out_swap_f(
    input                : float,  # x
    source_asset_balance : float,  # At
    source_asset_weight  : float,  # WA
    amplification        : float | None   # k
) -> float:
    # Computes the integral
    # int_{At}^{At+x} WA/w dw

    if amplification is not None:
        one_minus_amp = 1 - amplification

        return source_asset_weight * ((source_asset_balance + input)**one_minus_amp - source_asset_balance**one_minus_amp)
    
    # WA * log2((At + x)/At)
    return source_asset_weight * log2( (source_asset_balance + input) / source_asset_balance )


def in_swap_i(
    units_x64            : Uint256,  # U
    target_asset_balance : Uint256,  # Bt
    target_asset_weight  : Uint256,  # WB
    amplification_x64    : Uint256 | None   # k
) -> Uint256:
    # Solves the following integral for 'y'
    # int_{Bt-y}^{Bt} WB/(w ln(2)) dW

    if amplification_x64 is not None:

        one_minus_amp_x64 = ONE_X64 - amplification_x64
        intermediate_x64  = target_asset_weight * pow_x64(target_asset_balance << 64, one_minus_amp_x64)

        return (
            target_asset_balance * (
                ONE_X64 - inv_pow_x64(div_x64(intermediate_x64, intermediate_x64 - units_x64), div_x64(ONE_X64, one_minus_amp_x64))
            )
        ) >> 64
    
    # Bt * (1 - 2^(-U/WB))
    return (
        target_asset_balance * (ONE_X64 - inv_pow2_x64(units_x64/target_asset_weight))
    ) >> 64


def in_swap_uniform_approx_i(
    units_x64            : Uint256,  # U
    target_asset_balance : Uint256,  # Bt
    target_asset_weight  : Uint256,  # WB
    amplification_x64    : Uint256 | None   # k
) -> Uint256:

    if amplification_x64 is not None:
        raise Exception("Not implemented")
    
    intermediate_x64 = mul_x64(units_x64, LN2)

    return (target_asset_balance * intermediate_x64) / ((target_asset_weight << 64) + intermediate_x64)     # NOTE: div_x64 not used here, as we want the output to not be in x64 form


def in_swap_f(
    units                : float,  # U
    target_asset_balance : float,  # Bt
    target_asset_weight  : float,  # WB
    amplification        : float | None   # k
) -> float:
    # Solves the following integral for 'y'
    # int_{Bt-y}^{Bt} WB/w dW

    if amplification is not None:

        one_minus_amp = 1 - amplification
        intermediate = target_asset_weight * target_asset_balance**one_minus_amp

        return target_asset_balance * (1 - (intermediate/(intermediate - units))**(-1/one_minus_amp))
    
    # Bt * (1 - 2^(-U/WB))
    return target_asset_balance * (1 - 2**(-units/target_asset_weight))



# Liquidity swaps calculations **************************************************************************************************

def calc_out_liquidity_swap_i_x64(
    input_liquidity         : Uint256,
    source_asset_eq_balance : Uint256,
    source_asset_weight     : Uint256,
    amplification_x64       : Uint256 | None
) -> Uint256:

    if amplification_x64 is not None:

        one_minus_amp_x64 = ONE_X64 - amplification_x64
        return source_asset_weight * (
            pow_x64(source_asset_eq_balance << 64, one_minus_amp_x64) - pow_x64((source_asset_eq_balance - input_liquidity) << 64, one_minus_amp_x64)
        )

    return source_asset_weight * log2_x64(
        div_x64(source_asset_eq_balance, source_asset_eq_balance - input_liquidity)
    )

def calc_out_liquidity_swap_f(
    input_liquidity         : float,
    source_asset_eq_balance : float,
    source_asset_weight     : float,
    amplification           : float | None
) -> float:

    if amplification is not None:

        one_minus_amp = 1 - amplification
        return source_asset_weight * (
            source_asset_eq_balance**one_minus_amp - (source_asset_eq_balance - input_liquidity)**one_minus_amp
        )

    return source_asset_weight * log2(source_asset_eq_balance/(source_asset_eq_balance - input_liquidity))


def calc_in_liquidity_swap_i(
    liquidity_units_x64           : Uint256,
    target_asset_eq_balance       : Uint256,
    target_assets_aggr_weight_x64 : Uint256,
    amplification_x64             : Uint256 | None
) -> Uint256:
    
    if amplification_x64 is not None:

        one_minus_amp_x64 = ONE_X64 - amplification_x64
        return target_asset_eq_balance * (
            pow_x64(
                div_x64(target_assets_aggr_weight_x64 + liquidity_units_x64, target_assets_aggr_weight_x64),
                div_x64(ONE_X64, one_minus_amp_x64)
            )
        )

    return (target_asset_eq_balance * (
        pow2_x64(liquidity_units_x64 / (target_assets_aggr_weight_x64 >> 64)) - ONE_X64
    )) >> 64

def calc_in_liquidity_swap_f(
    liquidity_units           : float,
    target_asset_eq_balance   : float,
    target_assets_aggr_weight : float,
    amplification             : float | None
) -> float:
    
    if amplification is not None:
        one_minus_amp = 1 - amplification
        return target_asset_eq_balance * (
            ((target_assets_aggr_weight + liquidity_units)/target_assets_aggr_weight)**(1/one_minus_amp) - 1
        )

    return target_asset_eq_balance * (
        2**(liquidity_units/target_assets_aggr_weight) - 1
    )