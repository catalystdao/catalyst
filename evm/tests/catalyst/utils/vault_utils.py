import pytest
from brownie import convert, ZERO_ADDRESS, web3
from decimal import Decimal
from functools import reduce
from .common_utils import convert_64_bytes_address

# The following mathematical functions are implemented with full precision, rather than mirroring the exact contract implementation


# Constants *********************************************************************************************************************

WAD = Decimal(10**18)



# State Utils *******************************************************************************************************************

def compute_invariant(weights, balances, amp) -> Decimal:

    balances = [Decimal(balance) for balance in balances]
    weights  = [Decimal(weight) for weight in weights]

    # Amplified
    if amp != 10**18:

        one_minus_amp = Decimal(1) - Decimal(amp)/WAD

        return sum([(w * b)**one_minus_amp for w, b in zip(weights, balances)])
    
    # Volatile
    return reduce((lambda x, y: x*y), [b**w for w, b in zip(weights, balances)])


def compute_balance_0(weights, balances, unit_tracker, amp) -> Decimal:

    # TODO! currently amplified vaults CAN have amp = 10**18
    if amp == 10**18:
        pytest.fail("Unable to compute the balance0 of a volatile vault.")
    
    invariant = compute_invariant(balances, weights, amp)

    walpha_theta = (invariant - Decimal(unit_tracker)/WAD)/Decimal(len(balances))

    return walpha_theta ** (WAD/(WAD-amp))




# Swap Utils ********************************************************************************************************************

def compute_expected_swap(
    swap_amount,
    from_weight,
    from_balance,
    to_weight,
    to_balance,
    amp,
    to_amp = None,       # Allow to specify a different amplification value for the target vault
    vault_fee = 0,
    governance_fee = 0
):
    if to_amp is not None and amp != to_amp:
        #TODO implement amp/to_amp distinction
        raise Exception("to_amp NOT IMPLEMENTED YET")

    to_amp = amp if to_amp is None else to_amp
    
    swap_amount = Decimal(swap_amount)

    w_a = Decimal(from_weight)
    w_b = Decimal(to_weight)
    a = Decimal(from_balance)
    b = Decimal(to_balance)

    pf = Decimal(vault_fee)
    gf = Decimal(governance_fee)

    net_fee = pf * swap_amount
    net_vault_fee = int(pf * (1 - gf) * swap_amount)
    net_governance_fee = int(pf * gf * swap_amount)

    x = swap_amount - net_fee

    # Amplified vaults
    if amp != 10**18:

        amp /= WAD
        one_minus_amp = Decimal(1) - amp

        aw_a = a * w_a
        bw_b = b * w_b
        xw_a = x * w_a
        
        b_amp = bw_b**one_minus_amp
        
        U = (aw_a + xw_a)**one_minus_amp - aw_a**one_minus_amp

        return {
            'U': int(U * 10**18),
            'to_amount': int(b * (1 - ((b_amp - U)/(b_amp))**(1/one_minus_amp))),
            'vault_fee': net_vault_fee,
            'governance_fee': net_governance_fee
        }
    
    # Volatile vaults
    U = w_a * ((a + x)/a).ln()

    return {
        'U': int(U * 10**18),
        'to_amount': int(b * (1 - (-U/w_b).exp())),
        'vault_fee': net_vault_fee,
        'governance_fee': net_governance_fee
    }


def compute_expected_swap_given_U(U, to_weight, to_balance, amp):

    U = Decimal(U) / WAD
    w_b = Decimal(to_weight)
    b = Decimal(to_balance)
    
    # Amplified vaults
    if amp != 10**18:

        amp /= WAD

        b_amp = (b * w_b)**(1-amp)
        
        return int(b * (1 - ((b_amp - U)/(b_amp))**(1/(1-amp))))
    
    # Volatile vaults
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
    to_amp = None       # Allow to specify a different amplification value for the target vault
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

        amp /= WAD
        one_minus_amp = Decimal(1) - amp

        a0 = compute_balance_0(from_weights, from_balances, from_unit_tracker, amp)
        b0 = compute_balance_0(to_weights, to_balances, to_unit_tracker, amp)
        
        
        U = ( (a0 + (a0 * pt)/ts_a)**one_minus_amp - a0**one_minus_amp ) * len(from_balances)
        wpt = ( b0**one_minus_amp + U/len(to_balances) )**(1/(one_minus_amp)) - b0
        return {
            'Units': int(U * 10**18),
            'to_amount': int(wpt*ts_b/b0)
        }
    
    # Volatile
    from_w_sum = Decimal(sum(from_weights))
    to_w_sum   = Decimal(sum(to_weights))

    U = Decimal((ts_a/(ts_a-pt)).ln()) * from_w_sum
    
    share = Decimal(1) - Decimal(-U/to_w_sum).exp()

    return {
        'Units': int(U*10**18),
        'to_amount': int(ts_b * (share/(Decimal(1)-share)))
    }




# Deposit/Withdraw Utils ********************************************************************************************************

def compute_equal_withdrawal(withdraw_amount, weights, balances, total_supply, amp, unit_tracker=0):
    
    pt = Decimal(withdraw_amount)
    ts = Decimal(total_supply)
    balances = [Decimal(b) for b in balances]

    # Amplified vaults
    if amp != 10**18:
        amp = Decimal(amp)/WAD
        one_minus_amp = Decimal(1) - amp
    
        walpha = compute_balance_0(weights, balances, unit_tracker, amp)
        balances = [Decimal(b * w) for b, w in zip(balances, weights)]
        inner = 1 - ((ts - pt)/ts)**one_minus_amp
        assert inner < 1
        inner *= walpha**one_minus_amp
        
        return [
            (
                (int(b - (b**one_minus_amp - inner)**(Decimal(1)/one_minus_amp)) // w) 
                if (b**one_minus_amp >= inner)
                else (b // w)
            ) for b, w in zip(balances, weights)
        ]
    
    # Volatile vaults
    return [int(balance * pt / ts) for balance in balances]




# Security Limit Utils **********************************************************************************************************

def compute_expected_max_unit_inflow(weights, balances, amp):

    # Amplified vaults
    if amp != 10**18:
        weighted_sum = sum([Decimal(weight * balance) for weight, balance in zip(weights, balances)])

        return int(weighted_sum)

    # Volatile vaults
    return int(Decimal(sum(weights)) * Decimal(2).ln() * WAD)


def compute_expected_units_capacity(
    ref_capacity,
    ref_capacity_timestamp,
    change_timestamp,
    change_capacity_delta,
    current_timestamp,
    max_capacity,
    decayrate=24*60*60
):
    # Since the units capacity is time dependant, it must be taken into account two changes:
    #   - The capacity change since the ref_capacity value was taken until the capacity was modified by a transaction (the change_timestamp and change_capacity_delta)
    #   - The capacity change since the transaction until now

    # Compute the capacity at the time of the change
    ref_capacity_at_change = min(max_capacity, ref_capacity + int(Decimal(max_capacity)*Decimal(change_timestamp - ref_capacity_timestamp)/Decimal(decayrate)))

    # Compute the capacity after the change
    change_capacity = max(0, min(max_capacity, ref_capacity_at_change + change_capacity_delta))

    # Compute the capacity at the current time
    return min(max_capacity, change_capacity + int(Decimal(max_capacity)*Decimal(current_timestamp - change_timestamp)/Decimal(decayrate)))

# Escrow Utils ******************************************************************************************************************

def compute_asset_swap_hash(
    to_account,
    units,
    escrowed_amount,
    escrowed_token,
    block_number
):
    
    return web3.keccak(
        convert_64_bytes_address(to_account)
        + convert.to_bytes(units, type_str="bytes32")
        + convert.to_bytes(escrowed_amount, type_str="bytes32")
        + convert.to_bytes(escrowed_token, type_str="bytes20")
        + convert.to_bytes(block_number % 2**32, type_str="bytes4")
    ).hex()


def compute_liquidity_swap_hash(
    to_account,
    units,
    escrowed_amount,
    block_number
):
    
    return web3.keccak(
        convert_64_bytes_address(to_account)
        + convert.to_bytes(units, type_str="bytes32")
        + convert.to_bytes(escrowed_amount, type_str="bytes32")
        + convert.to_bytes(block_number % 2**32, type_str="bytes4")
    ).hex()




# Interface Utils ***************************************************************************************************************


def decode_payload(data):

    context = data[0]

    # Liquidity swap payload
    if context & 1:
        # Asset swap payload
        custom_data_length = convert.to_uint(data[328:330], type_str="uint16")
        return {
            "_context": data[0],
            "_fromVault": (data[1:66]).hex(),
            "_toVault": (data[66:131]).hex(),
            "_toAccount": (data[131:196]).hex(),
            "_U": convert.to_uint(data[196:228]),
            "_minVaultToken": convert.to_uint(data[228:260]),
            "_minReferenceAsset": convert.to_uint(data[260:292]),
            "_escrowAmount": convert.to_uint(data[292:324]),
            "_blockNumber": convert.to_uint(data[324:328]),
            "customDataLength": custom_data_length,
            "_customDataTarget": (data[330:350]) if custom_data_length > 0 else None,
            "_customData": data[350:350+custom_data_length - 20] if custom_data_length > 0 else None
        }
    
    # Asset swap payload
    custom_data_length = convert.to_uint(data[362:364], type_str="uint16")
    return {
        "_context": data[0],
        "_fromVault": (data[1:66]).hex(),
        "_toVault": (data[66:131]).hex(),
        "_toAccount": (data[131:196]).hex(),
        "_U": convert.to_uint(data[196:228]),
        "_assetIndex": convert.to_uint(data[228], type_str="uint8"),
        "_minOut": convert.to_uint(data[229:261]),
        "_escrowAmount": convert.to_uint(data[261:293]),
        "_escrowToken": (data[293:358]).hex(),
        "_blockNumber": convert.to_uint(data[358:3662]),
        "customDataLength": custom_data_length,
        "_customDataTarget": (data[364:384]) if custom_data_length > 0 else None,
        "_customData": data[384:384+custom_data_length - 20] if custom_data_length > 0 else None
    }


# Encode a Catalyst swap message
#TODO allow for customData
def encode_swap_payload(
    from_vault,
    to_vault,
    to_account,
    U,
    asset_index=0,
    min_out=0,
    escrow_amount=0,
    escrow_token=ZERO_ADDRESS,
    block_number=0,
    custom_data=None
):
    if custom_data is not None:
        raise Exception("custom_data is not implemented on encode_swap_payload")

    return (
        convert.to_bytes(0, "bytes1")
        + convert_64_bytes_address(from_vault)
        + convert_64_bytes_address(to_vault)
        + convert_64_bytes_address(to_account)
        + convert.to_bytes(U)
        + convert.to_bytes(asset_index, "bytes1")
        + convert.to_bytes(min_out)
        + convert.to_bytes(escrow_amount)
        + convert_64_bytes_address(escrow_token)
        + convert.to_bytes(block_number, "bytes4")
        + convert.to_bytes(0, "bytes2")
    )
    
    
# Encode a Catalyst liquidity swap message
def encode_liquidity_swap_payload(
    from_vault,
    to_vault,
    to_account,
    U,
    min_out=[0,0],
    escrow_amount=0,
    block_number=0
):
    return (
        convert.to_bytes(1, "bytes1")
        + convert_64_bytes_address(from_vault)
        + convert_64_bytes_address(to_vault)
        + convert_64_bytes_address(to_account)
        + convert.to_bytes(U)
        + convert.to_bytes(min_out[0])
        + convert.to_bytes(min_out[1])
        + convert.to_bytes(escrow_amount)
        + convert.to_bytes(block_number, "bytes4")
        + convert.to_bytes(0, type_str="bytes2")
    )

