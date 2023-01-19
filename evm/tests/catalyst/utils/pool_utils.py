import pytest
from decimal import Decimal
from functools import reduce

# The following mathematical functions are implemented with full precision, rather than mirroring the exact contract implementation




# State Utils *******************************************************************************************************************

def compute_invariant(weights, balances, amp) -> Decimal:

    balances = [Decimal(balance) for balance in balances]
    weights  = [Decimal(weight) for weight in weights]

    # Amplified
    if amp != 10**18:

        one_minus_amp = Decimal(1) - Decimal(amp)/Decimal(10**18)

        return sum([(w * b)**one_minus_amp for w, b in zip(weights, balances)])
    
    # Volatile
    return reduce((lambda x, y: x*y), [b**w for w, b in zip(weights, balances)])


def compute_balance_0(weights, balances, unit_tracker, amp) -> Decimal:

    # TODO! currently amplified pools CAN have amp = 10**18
    if amp == 10**18:
        pytest.fail("Unable to compute the balance0 of a volatile pool.")
    
    invariant = compute_invariant(balances, weights, amp)

    walpha_theta = (invariant - Decimal(unit_tracker)/Decimal(10**18))/Decimal(len(balances))

    return walpha_theta ** (Decimal(10**18)/(Decimal(10**18)-amp))




# Swap Utils ********************************************************************************************************************

def compute_expected_swap(
    swap_amount,
    from_weight,
    from_balance,
    to_weight,
    to_balance,
    amp,
    to_amp = None       # Allow to specify a different amplification value for the target pool
):
    if to_amp is not None and amp != to_amp:
        #TODO implement amp/to_amp distinction
        raise Exception("to_amp NOT IMPLEMENTED YET")

    to_amp = amp if to_amp is None else to_amp
    
    x = Decimal(swap_amount)
    w_a = Decimal(from_weight)
    w_b = Decimal(to_weight)
    a = Decimal(from_balance)
    b = Decimal(to_balance)

    # Amplified pools
    if amp != 10**18:

        amp /= Decimal(10**18)
        one_minus_amp = Decimal(1) - amp

        aw_a = a * w_a
        bw_b = b * w_b
        xw_a = x * w_a
        
        b_amp = bw_b**one_minus_amp
        
        U = (aw_a + xw_a)**one_minus_amp - aw_a**one_minus_amp

        return {
            'U': int(U * 10**18),
            'output': int(b * (1 - ((b_amp - U)/(b_amp))**(1/one_minus_amp)))
        }
    
    # Volatile pools
    U = w_a * ((a + x)/a).ln()

    return {
        'U': int(U * 10**18),
        'output': int(b * (1 - (-U/w_b).exp()))
    }


def compute_expected_swap_given_U(U, to_weight, to_balance, amp):

    U = Decimal(U) / Decimal(10**18)
    w_b = Decimal(to_weight)
    b = Decimal(to_balance)
    
    # Amplified pools
    if amp != 10**18:

        amp /= Decimal(10**18)

        b_amp = (b * w_b)**(1-amp)
        
        return int(b * (1 - ((b_amp - U)/(b_amp))**(1/(1-amp))))
    
    # Volatile pools
    return int(b * (1 - (-U/w_b).exp()))




# Liquidity Swap Utils **********************************************************************************************************

def compute_expected_liquidity_swap(
    swap_amount,
    from_weights,
    from_balances,
    from_total_supply,
    from_unit_tracker,
    to_weights,
    to_balances,
    to_total_supply,
    to_unit_tracker,
    amp,
    to_amp = None       # Allow to specify a different amplification value for the target pool
):
    if to_amp is not None and amp != to_amp:
        #TODO implement amp/to_amp distinction
        raise Exception("to_amp NOT IMPLEMENTED YET")

    to_amp = amp if to_amp is None else to_amp

    pt = Decimal(swap_amount)
    ts_a = Decimal(from_total_supply)
    ts_b = Decimal(to_total_supply)
    
    # Amplified
    if amp != 10**18:

        amp /= Decimal(10**18)
        one_minus_amp = Decimal(1) - amp

        a0 = compute_balance_0(from_weights, from_balances, from_unit_tracker, amp)
        b0 = compute_balance_0(to_weights, to_balances, to_unit_tracker, amp)
        
        
        U = ( (a0 + (a0 * pt)/ts_a)**one_minus_amp - a0**one_minus_amp ) * len(from_balances)
        wpt = ( b0**one_minus_amp + U/len(to_balances) )**(Decimal(10**18)/(Decimal(10**18)-amp)) - b0

        return {
            'l_U': int(U * 10**18),
            'output': int(wpt*ts_b/b0)
        }
    
    # Volatile
    from_w_sum = Decimal(sum(from_weights))
    to_w_sum   = Decimal(sum(to_weights))

    U = Decimal((ts_a/(ts_a-pt)).ln()) * from_w_sum
    
    share = Decimal(1) - Decimal(2)**(-U/to_w_sum)

    return {
        'l_U': int(U*10**18),
        'output': int(ts_b * (share/(1-share)))
    }




# Deposit/Withdraw Utils ********************************************************************************************************

def compute_equal_withdrawal(withdraw_amount, weights, balances, total_supply, amp, unit_tracker=0):
    
    pt = Decimal(withdraw_amount)
    ts = Decimal(total_supply)
    balances = [Decimal(b) for b in balances]

    # Amplified pools
    if amp != 10**18:
        amp = Decimal(amp)/Decimal(10**18)
        one_minus_amp = Decimal(1) - amp
    
        walpha = compute_balance_0(weights, balances, unit_tracker, amp)
        balances = [Decimal(b * w) for b, w in zip(balances, weights)]
        inner = ((ts + pt)/ts)**one_minus_amp - 1
        inner *= walpha**one_minus_amp
        
        return [int((b**one_minus_amp + inner)**(Decimal(1)/one_minus_amp) - b) // w for b, w in zip(balances, weights)]
    
    # Volatile pools
    return [int(balance * pt / ts) for balance in balances]




