from typing import List
from anchorpy import Program

from solana.publickey import PublicKey

from .verify_utils import verify_catalyst_configuration, verify_catalyst_state 

from catalyst_simulator import CatalystSimulator  # type: ignore
from integer import Uint64, Int64                 # type: ignore

async def create_and_verify_catalyst_simulator(
    swap_pool_program    : Program,
    swap_pool_state      : PublicKey,
    swap_interface_state : PublicKey,
    amplification        : int | None,
    assets               : List[PublicKey],
    assets_weights       : List[int],
    init_assets_balances : List[int],
    liquidity_provider   : PublicKey
) -> CatalystSimulator:

    catalyst_simulator = CatalystSimulator(
        amplification=amplification,
        assets=assets,
        assets_weights=assets_weights,
        init_assets_balances=init_assets_balances,
        depositor=liquidity_provider,
        uint_type=Uint64,
        int_type=Int64
    )

    await verify_catalyst_configuration(
        swap_pool_program            = swap_pool_program,
        swap_pool_state              = swap_pool_state,
        expected_amplification       = amplification,
        expected_pool_assets_mints   = catalyst_simulator.assets,
        expected_pool_assets_weights = assets_weights,
        expected_ibc_interface       = swap_interface_state,
        expected_setup_master        = None
    )

    await verify_catalyst_state(swap_pool_program, swap_pool_state, catalyst_simulator)

    return catalyst_simulator
