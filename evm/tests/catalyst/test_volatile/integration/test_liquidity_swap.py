import brownie
import pytest
from brownie import ZERO_ADDRESS, chain, convert, reverts, web3
from brownie.test import given, strategy

pytestmark = pytest.mark.usefixtures("group_finish_setup", "group_connect_pools")


@pytest.mark.no_call_coverage
@given(deposit_percentage=strategy("uint256", max_value=20000), swap_percentage=strategy("uint256", max_value=10000))
def test_liquidity_swap(
    channel_id,
    source_pool,
    target_pool,
    source_pool_tokens,
    berg,
    deployer,
    ibc_emulator,
    compute_expected_liquidity_swap,
    swap_percentage,
    deposit_percentage
    ):
    swap_percentage /= 10000
    deposit_percentage /= 10000
    
    deposit_amounts = [int(token.balanceOf(source_pool) * deposit_percentage) for token in source_pool_tokens]
    [token.transfer(berg, amount, {'from': deployer}) for amount, token in zip(deposit_amounts, source_pool_tokens)]
    [token.approve(source_pool, amount, {'from': berg}) for amount, token in zip(deposit_amounts, source_pool_tokens)]
    
    estimatedPoolTokens = int(source_pool.totalSupply()*deposit_percentage)
    
    tx = source_pool.depositMixed(deposit_amounts, int(estimatedPoolTokens*0.999), {"from": berg})
    
    pool1_tokens = tx.return_value
    
    pool1_tokens_swapped = int(pool1_tokens * swap_percentage)
    
    computation = compute_expected_liquidity_swap(pool1_tokens_swapped)
    U, estimatedPool2Tokens = computation["U"], computation["output"]
    
    tx = source_pool.outLiquidity(
        channel_id,
        convert.to_bytes(target_pool.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        pool1_tokens_swapped,
        int(estimatedPool2Tokens*9/10),
        berg,
        {"from": berg}
    )
    assert source_pool.balanceOf(berg) == pool1_tokens - pool1_tokens_swapped
    
    if target_pool.getUnitCapacity() < tx.events["SwapToLiquidityUnits"]["output"]:
        with reverts("Swap exceeds maximum swap amount"):
            txe = ibc_emulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})
        return
    else:
        txe = ibc_emulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})
    
    purchased_tokens = txe.events["SwapFromLiquidityUnits"]["output"]
    
    assert purchased_tokens == target_pool.balanceOf(berg)
    
    
    assert purchased_tokens <= int(estimatedPool2Tokens*1.000001), "Swap returns more than theoretical"
    
    if swap_percentage < 1e-05:
        return
    
    assert (estimatedPool2Tokens * 9 /10) <= purchased_tokens, "Swap returns less than 9/10 theoretical"
    

