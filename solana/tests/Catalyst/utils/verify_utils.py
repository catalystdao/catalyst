from typing import Any, List
from anchorpy import Program
from solana.publickey import PublicKey

from catalyst_simulator import CatalystSimulator # type: ignore
from utils.account_utils import get_swap_pool_asset_wallet, get_swap_pool_authority, get_swap_pool_token_mint
from utils.token_utils import get_account_info, get_mint_info, verify_token_mint, verify_token_wallet


DEFAULT_PUBLIC_KEY = PublicKey(0)

U64_MAX = 2**64-1

def compare_values(real: int | float, expected: int | float, allowed_deviation: int | float) -> bool:
    if expected == 0:
        return real == expected
    
    return abs(1 - real / expected) <= allowed_deviation


async def verify_catalyst_configuration(
    swap_pool_program: Program,
    swap_pool_state: PublicKey,
    expected_amplification: float | None,
    expected_pool_assets_mints: List[PublicKey],
    expected_pool_assets_weights: List[int],
    expected_ibc_interface: PublicKey | None,
    expected_setup_master: PublicKey | None
):
    assert(len(expected_pool_assets_mints) == len(expected_pool_assets_weights))


    # Get the state of the swap pool
    swap_pool_state_data = await swap_pool_program.account["SwapPoolState"].fetch(swap_pool_state)


    # Check the amplification constant
    if expected_amplification is None:
        assert "amplification_x64" not in dir(swap_pool_state_data)
    else:
        assert u256_x64_array_to_float(swap_pool_state_data.amplification_x64) == expected_amplification, \
            f"Unexpected Catalyst configuration: amplification constant mismatch  \
            ({swap_pool_state_data.amplification_x64} set, expected {expected_amplification})."
    

    # Verify authority
    expected_swap_pool_authority, expected_swap_pool_authority_bump = get_swap_pool_authority(swap_pool_program.program_id, swap_pool_state)
    
    assert swap_pool_state_data.authority_bump == expected_swap_pool_authority_bump, \
        f"Unexpected Catalyst configuration: authority bump mismatch \
            ({swap_pool_state_data.authority_bump} set, expected {expected_swap_pool_authority_bump})."


    # Check asset mints and weights
    assert len(expected_pool_assets_mints) <= len(swap_pool_state_data.pool_assets_mints), \
        f"Unexpected Catalyst configuration: too many expected pool assets mints provided, \
        maximum {len(swap_pool_state_data.pool_assets_mints)} assets allowed."
    
    for i, asset_mint in enumerate(swap_pool_state_data.pool_assets_mints):

        if i < len(expected_pool_assets_mints):
            # Asset mint
            assert asset_mint == expected_pool_assets_mints[i], \
                f"Unexpected Catalyst configuration: unexpected asset mint at position {i}  \
                ({swap_pool_state_data.pool_assets_mints[i]} set, expected {expected_pool_assets_mints[i]})."

            # Asset weight
            assert swap_pool_state_data.pool_assets_weights[i] == expected_pool_assets_weights[i], \
                f"Unexpected Catalyst configuration: asset weight mismatch for asset {i}  \
                ({swap_pool_state_data.pool_assets_weights[i]} set, expected {expected_pool_assets_weights[i]})."
            
            # Check pool asset wallet exists
            expected_asset_wallet, expected_asset_wallet_bump = get_swap_pool_asset_wallet(swap_pool_program.program_id, swap_pool_state, asset_mint)
    
            assert swap_pool_state_data.wallets_bumps[i] == expected_asset_wallet_bump, \
                f"Unexpected Catalyst configuration: swap pool asset wallet bump mismatch \
                    ({swap_pool_state_data.wallets_bumps[i]} set, expected {expected_asset_wallet_bump})."

            await verify_token_wallet(
                swap_pool_program.provider,
                expected_asset_wallet,
                asset_mint,
                expected_swap_pool_authority
            )
        
        else:
            assert asset_mint == DEFAULT_PUBLIC_KEY, \
                f"Unexpected Catalyst configuration: non-empty asset mint found at position {i}  \
                ({swap_pool_state_data.pool_assets_mints[i]} set, expected {DEFAULT_PUBLIC_KEY})."


    # Check pool token mint exists
    expected_token_mint, expected_token_mint_bump = get_swap_pool_token_mint(swap_pool_program.program_id, swap_pool_state)

    assert swap_pool_state_data.token_mint_bump == expected_token_mint_bump, \
        f"Unexpected Catalyst configuration: token mint bump mismatch \
            ({swap_pool_state_data.token_mint_bump} set, expected {expected_token_mint_bump})."

    await verify_token_mint(
        swap_pool_program.provider,
        expected_token_mint,
        expected_swap_pool_authority,
        None
    )


    # Check swap interface
    expected_ibc_interface = expected_ibc_interface or DEFAULT_PUBLIC_KEY     # Replace None with empty public key
    assert swap_pool_state_data.ibc_interface == expected_ibc_interface, \
        f"Unexpected Catalyst configuration: swap interface mismatch \
        ({swap_pool_state_data.ibc_interface} set, expected {expected_ibc_interface})."


    # Check setup master
    if expected_setup_master is None or expected_setup_master == DEFAULT_PUBLIC_KEY:
        # If setup is complete
        assert swap_pool_state_data.setup_master == DEFAULT_PUBLIC_KEY, \
            f"Unexpected Catalyst configuration: configuration not finalised - setup master not empty \
            ({swap_pool_state_data.setup_master} set, expected {DEFAULT_PUBLIC_KEY})."
        
        assert len(expected_pool_assets_mints) > 0, "Unexpected Catalyst configuration: pool setup complete without any assets."

        assert expected_ibc_interface != DEFAULT_PUBLIC_KEY, \
            "Unexpected Catalyst configuration: pool setup complete without a set swap interface."
    
    else:
        assert swap_pool_state_data.setup_master == expected_setup_master, \
            f"Unexpected Catalyst configuration: setup master mismatch \
            ({swap_pool_state_data.setup_master} set, expected {expected_setup_master})."


async def verify_catalyst_state(
    swap_pool_program: Program,
    swap_pool_state: PublicKey,
    catalyst_simulator: CatalystSimulator
):
    # Get the state of the swap pool
    swap_pool_state_data = await swap_pool_program.account["SwapPoolState"].fetch(swap_pool_state)
    
    # Check balances
    for i, asset in enumerate(swap_pool_state_data.pool_assets_mints):

        if asset == DEFAULT_PUBLIC_KEY:
            continue    # 'break' not used intentionally to check all elements of pool_assets_mints. This is a redundant check (which is not strictly necessary)

        # Asset balance
        asset_wallet = get_swap_pool_asset_wallet(
            swap_pool_program.program_id,
            swap_pool_state,
            asset
        )[0]

        asset_wallet_info = await get_account_info(
            swap_pool_program.provider,
            asset,
            asset_wallet
        )
        assert asset_wallet_info.amount == catalyst_simulator.assets_balances_i[asset], \
            f"Verify catalyst state error: asset {i} balance mismatch \
            (read {asset_wallet_info.amount}, expected {catalyst_simulator.assets_balances_i[asset].value})"
        
        # Asset eq balance
        assert swap_pool_state_data.pool_assets_eq_balances[i] == catalyst_simulator.assets_eq_balances_i[asset], \
            f"Verify catalyst state error: asset {i} eq balance mismatch \
            (read {swap_pool_state_data.pool_assets_eq_balances[i]}, expected {catalyst_simulator.assets_eq_balances_i[asset].value})"
        
        # Escrowed assets
        assert swap_pool_state_data.escrowed_assets[i] == catalyst_simulator.escrowed_assets_i[asset], \
            f"Verify catalyst state error: escrowed asset {i} balance mismatch \
            (read {swap_pool_state_data.escrowed_assets[i]}, expected {catalyst_simulator.escrowed_assets_i[asset].value})"

    # Pool tokens balance
    pool_token_mint = get_swap_pool_token_mint(
        swap_pool_program.program_id,
        swap_pool_state
    )[0]

    pool_token_mint_info = await get_mint_info(
        swap_pool_program.provider,
        pool_token_mint
    )

    assert pool_token_mint_info.supply == catalyst_simulator.pool_tokens_supply_i, \
        f"Verify catalyst state error: pool tokens supply mismatch \
        (read {pool_token_mint_info.supply}, expected {catalyst_simulator.pool_tokens_supply_i.value})"


def verify_deposit_event(
    deposit_event               : Any,
    swap_pool                   : PublicKey,
    depositor_asset_wallets     : list[PublicKey],
    depositor_pool_token_wallet : PublicKey,
    deposited_asset_amounts     : list[int] | None = None,
    withdrawn_pool_token_amount : int | None = None
) -> None:
    assert deposit_event.data.swapPool                 == swap_pool
    assert deposit_event.data.depositorAssetWallets    == depositor_asset_wallets
    assert deposit_event.data.depositorPoolTokenWallet == depositor_pool_token_wallet

    if deposited_asset_amounts is not None:
        assert deposit_event.data.depositedAssetAmounts == deposited_asset_amounts

    if withdrawn_pool_token_amount is not None:
        assert deposit_event.data.withdrawnPoolTokenAmount == withdrawn_pool_token_amount


def verify_withdraw_event(
    withdraw_event               : Any,
    swap_pool                    : PublicKey,
    withdrawer_asset_wallets     : list[PublicKey],
    withdrawer_pool_token_wallet : PublicKey,
    withdrawn_asset_amounts      : list[int] | None = None,
    burnt_pool_token_amount      : int | None = None
):
    assert withdraw_event.data.swapPool                  == swap_pool
    assert withdraw_event.data.withdrawerAssetWallets    == withdrawer_asset_wallets
    assert withdraw_event.data.withdrawerPoolTokenWallet == withdrawer_pool_token_wallet

    if withdrawn_asset_amounts is not None:
        assert withdraw_event.data.withdrawnAssetAmounts == withdrawn_asset_amounts
    
    if burnt_pool_token_amount is not None:
        assert withdraw_event.data.burntPoolTokenAmount  == burnt_pool_token_amount


def verify_local_swap_event(
    local_swap_event        : Any,
    swap_pool               : PublicKey,
    deposited_asset_mint    : PublicKey,
    depositor_asset_wallet  : PublicKey,
    withdrawn_asset_mint    : PublicKey,
    withdrawer_asset_wallet : PublicKey,
    deposited_asset_amount  : int,
    withdrawn_asset_amount  : int | None = None
):
    assert local_swap_event.data.swapPool              == swap_pool
    assert local_swap_event.data.depositedAssetMint    == deposited_asset_mint
    assert local_swap_event.data.depositorAssetWallet  == depositor_asset_wallet
    assert local_swap_event.data.withdrawnAssetMint    == withdrawn_asset_mint
    assert local_swap_event.data.withdrawerAssetWallet == withdrawer_asset_wallet

    assert local_swap_event.data.depositedAssetAmount     == deposited_asset_amount

    if withdrawn_asset_amount is not None:
        assert local_swap_event.data.withdrawnAssetAmount == withdrawn_asset_amount


def verify_out_swap_event(
    out_swap_event           : Any,
    swap_pool                : PublicKey,
    target_pool              : PublicKey,
    target_asset_index       : int,
    target_withdrawer        : PublicKey,
    target_chain             : int,
    deposited_asset_mint     : PublicKey,
    depositor_asset_wallet   : PublicKey,
    deposited_asset_amount   : int,
    source_swap_id           : int,
    withdrawn_pool_units_x64 : int | None = None
):
    assert out_swap_event.data.swapPool             == swap_pool
    assert out_swap_event.data.targetPool           == target_pool
    assert out_swap_event.data.targetAssetIndex     == target_asset_index
    assert out_swap_event.data.targetWithdrawer     == target_withdrawer
    assert out_swap_event.data.targetChain          == target_chain
    assert out_swap_event.data.depositedAssetMint   == deposited_asset_mint
    assert out_swap_event.data.depositorAssetWallet == depositor_asset_wallet
    assert out_swap_event.data.depositedAssetAmount == deposited_asset_amount
    assert out_swap_event.data.escrowNonce          == source_swap_id

    if withdrawn_pool_units_x64 is not None:
        assert u256_array_to_int(out_swap_event.data.withdrawnPoolUnitsX64) == withdrawn_pool_units_x64


def verify_in_swap_event(
    in_swap_event            : Any,
    swap_pool                : PublicKey,
    withdrawn_asset_mint     : PublicKey,
    withdrawer_asset_wallet  : PublicKey,
    deposited_pool_units_x64 : int,
    withdrawn_asset_amount   : int | None = None
):
    assert in_swap_event.data.swapPool              == swap_pool
    assert in_swap_event.data.withdrawnAssetMint    == withdrawn_asset_mint
    assert in_swap_event.data.withdrawerAssetWallet == withdrawer_asset_wallet

    assert u256_array_to_int(in_swap_event.data.depositedPoolUnitsX64) == deposited_pool_units_x64

    if withdrawn_asset_amount is not None:
        assert in_swap_event.data.withdrawnAssetAmount == withdrawn_asset_amount



def verify_out_liquidity_swap_event(
    out_swap_event           : Any,
    swap_pool                : PublicKey,
    target_pool              : PublicKey,
    target_beneficiary       : PublicKey,
    target_chain             : int,
    pool_token_mint          : PublicKey,
    source_pool_token_wallet : PublicKey,
    pool_token_amount        : int,
    liquidity_units_x64      : int | None = None
):
    assert out_swap_event.data.swapPool              == swap_pool
    assert out_swap_event.data.targetPool            == target_pool
    assert out_swap_event.data.targetBeneficiary     == target_beneficiary
    assert out_swap_event.data.targetChain           == target_chain
    assert out_swap_event.data.poolTokenMint         == pool_token_mint
    assert out_swap_event.data.sourcePoolTokenWallet == source_pool_token_wallet

    assert out_swap_event.data.poolTokenAmount       == pool_token_amount

    if liquidity_units_x64 is not None:
        assert u256_array_to_int(out_swap_event.data.liquidityUnitsX64) == liquidity_units_x64


def verify_in_liquidity_swap_event(
    out_swap_event           : Any,
    swap_pool                : PublicKey,
    pool_token_mint          : PublicKey,
    target_pool_token_wallet : PublicKey,
    liquidity_units_x64      : int,
    pool_token_amount        : int | None = None
):
    assert out_swap_event.data.swapPool              == swap_pool
    assert out_swap_event.data.poolTokenMint         == pool_token_mint
    assert out_swap_event.data.targetPoolTokenWallet == target_pool_token_wallet

    assert u256_array_to_int(out_swap_event.data.liquidityUnitsX64) == liquidity_units_x64

    if pool_token_amount is not None:
        assert out_swap_event.data.poolTokenAmount == pool_token_amount


def int_to_u256_array(value: int) -> List[int]:
    return [
        value & U64_MAX,
        (value >> 64) & U64_MAX,
        (value >> 128) & U64_MAX,
        (value >> 192) & U64_MAX,
    ]

def u256_array_to_int(array: List[int]) -> int:
    return array[0] + (array[1]<<64) + (array[2]<<128) + (array[3]<<192)

def u256_x64_array_to_float(array: List[int]) -> float:
    return u256_array_to_int(array) / 2**64

def units_bytes_to_int(bytes: List[int]) -> int:
    return int.from_bytes(bytes[:8], 'big')             \
        + (int.from_bytes(bytes[8:16], 'big') << 64)    \
        + (int.from_bytes(bytes[16:24], 'big') << 128)  \
        + (int.from_bytes(bytes[24:32], 'big') << 192)
