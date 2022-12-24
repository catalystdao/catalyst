from dataclasses import dataclass
from typing import List
from brownie.network.transaction import TransactionReceipt
from .swap_utils import get_swappool_spot_prices
from .utils import assert_relative_error, relative_error


@dataclass
class RunDepositResult:
    tx                : TransactionReceipt
    pool_token_amount : int
    assets_amounts    : List[int]

@dataclass
class RunWithdrawResult:
    tx                : TransactionReceipt
    pool_token_amount : int
    assets_amounts    : List[int]


def run_deposit(
    amount,
    swappool_info,
    depositor,
    gov               = None,
    large_error_bound = 1e-4,
    small_error_bound = 1e-6
) -> RunDepositResult:

    sp         = swappool_info.swappool
    sp_tokens  = swappool_info.tokens
    sp_weights = swappool_info.token_weights

    init_sp_token_balances         = [token.balanceOf(sp) for token in sp_tokens]
    init_sp_pool_token_supply      = sp.totalSupply()
    init_sp_depositor_pool_tokens  = sp.balanceOf(depositor)
    init_sp_balance0               = [sp._balance0(token) for token in sp_tokens]
    init_sp_invariant              = get_swappool_group_invariant([[sp, sp_tokens]])
    init_sp_spot_prices            = get_swappool_spot_prices(sp, sp_tokens)

    init_depositor_token_balances = [token.balanceOf(depositor) for token in sp_tokens]

    # Compute expected desposited token balances
    deposit_proportion = amount / init_sp_pool_token_supply
    expected_deposit_balances = [int(sp_balance * deposit_proportion) for sp_balance in init_sp_token_balances]

    # Verify that the depositor has enough assets to deposit
    for i, token in enumerate(sp_tokens):
        # ! Do not fail if the depositor does not have enough assets and no gov is provided => Let the deposit function fail
        if gov is not None and init_depositor_token_balances[i] < (expected_deposit_balances[i] + 1):                       # Extra 1 to allow for rounding errors
            token.transfer(depositor, expected_deposit_balances[i] - init_depositor_token_balances[i] + 1, {"from": gov})   # Extra 1 to allow for rounding errors
            init_depositor_token_balances[i] = expected_deposit_balances[i] + 1                                             # Extra 1 to allow for rounding errors

    # Approve fund for the swap pool
    for token, deposit_balance in zip(sp_tokens, expected_deposit_balances):
        token.approve(sp, deposit_balance + 1, {"from": depositor}) # Extra 1 to allow for rounding errors

    # Deposit tokens
    tx = sp.depositAll(amount, {"from": depositor})

    # Check transaction event
    assert len(tx.events['Deposit']) == 1
    deposit_event = tx.events['Deposit'][0]

    assert deposit_event['who']  == depositor
    assert deposit_event['mint'] == amount

    deposited_asset_balances = deposit_event['assets']

    # Check pool tokens
    assert sp.totalSupply() == init_sp_pool_token_supply + amount
    assert sp.balanceOf(depositor) == init_sp_depositor_pool_tokens + amount

    # Check balance0s
    for i, token in enumerate(sp_tokens):
        assert_relative_error(sp._balance0(token), init_sp_balance0[i] * (1 + deposit_proportion), -large_error_bound, small_error_bound)
    
    # Check deposited amounts
    for i, token in enumerate(sp_tokens):

        expected_deposit_amount = expected_deposit_balances[i]
        observed_deposit_amount = deposited_asset_balances[i]

        assert_relative_error(observed_deposit_amount, expected_deposit_amount, -small_error_bound, large_error_bound)

        # Swap pool balance
        assert token.balanceOf(sp) == init_sp_token_balances[i] + deposited_asset_balances[i]
        # Depositor balance
        assert token.balanceOf(depositor) == init_depositor_token_balances[i] - deposited_asset_balances[i]

    # Check pool invariant
    assert_relative_error(get_swappool_group_invariant([[sp, sp_tokens]]), init_sp_invariant, -small_error_bound, large_error_bound)

    # Check spot prices
    for init_spot_price, new_spot_price in zip(init_sp_spot_prices, get_swappool_spot_prices(sp, sp_tokens)):
        assert abs(relative_error(init_spot_price, new_spot_price)) < large_error_bound

    return RunDepositResult(tx, amount, deposited_asset_balances)


def run_amp_deposit(
    amount,
    swappool_info,
    depositor,
    gov               = None,
    large_error_bound = 1e-4,
    small_error_bound = 1e-6
) -> RunDepositResult:

    #TODO to be reviewed once deposit on amp pools get finalized

    sp         = swappool_info.swappool
    sp_tokens  = swappool_info.tokens
    sp_weights = swappool_info.token_weights

    init_sp_token_balances         = [token.balanceOf(sp) for token in sp_tokens]
    init_sp_pool_token_supply      = sp.totalSupply()
    init_sp_depositor_pool_tokens  = sp.balanceOf(depositor)
    init_sp_balance0               = [sp._balance0(token) for token in sp_tokens]
    init_sp_invariant              = get_amp_swappool_group_invariant([[sp, sp_tokens]])
    init_sp_spot_prices            = get_swappool_spot_prices(sp, sp_tokens)

    init_depositor_token_balances = [token.balanceOf(depositor) for token in sp_tokens]

    # Compute expected desposited token balances
    deposit_proportion = amount / init_sp_pool_token_supply
    expected_deposit_balances = [int(sp_balance * deposit_proportion) for sp_balance in init_sp_token_balances]

    # Verify that the depositor has enough assets to deposit
    for i, token in enumerate(sp_tokens):
        # ! Do not fail if the depositor does not have enough assets and no gov is provided => Let the deposit function fail
        if gov is not None and init_depositor_token_balances[i] < (expected_deposit_balances[i] + 1):                       # Extra 1 to allow for rounding errors
            token.transfer(depositor, expected_deposit_balances[i] - init_depositor_token_balances[i] + 1, {"from": gov})   # Extra 1 to allow for rounding errors
            init_depositor_token_balances[i] = expected_deposit_balances[i] + 1                                             # Extra 1 to allow for rounding errors

    # Approve fund for the swap pool
    for token, deposit_balance in zip(sp_tokens, expected_deposit_balances):
        token.approve(sp, deposit_balance + 1, {"from": depositor}) # Extra 1 to allow for rounding errors

    # Deposit tokens
    tx = sp.depositAll(amount, {"from": depositor})

    # Check transaction event
    assert len(tx.events['Deposit']) == 1
    deposit_event = tx.events['Deposit'][0]

    assert deposit_event['who']  == depositor
    assert deposit_event['mint'] == amount

    deposited_asset_balances = deposit_event['assets']

    # Check pool tokens
    assert sp.totalSupply() == init_sp_pool_token_supply + amount
    assert sp.balanceOf(depositor) == init_sp_depositor_pool_tokens + amount

    # Check balance0s
    for i, token in enumerate(sp_tokens):
        assert_relative_error(sp._balance0(token), init_sp_balance0[i] * (1 + deposit_proportion), -large_error_bound, small_error_bound)
    
    # Check deposited amounts
    for i, token in enumerate(sp_tokens):

        expected_deposit_amount = expected_deposit_balances[i]
        observed_deposit_amount = deposited_asset_balances[i]

        assert_relative_error(observed_deposit_amount, expected_deposit_amount, -small_error_bound, large_error_bound)

        # Swap pool balance
        assert token.balanceOf(sp) == init_sp_token_balances[i] + deposited_asset_balances[i]
        # Depositor balance
        assert token.balanceOf(depositor) == init_depositor_token_balances[i] - deposited_asset_balances[i]

    # Check pool invariant
    assert_relative_error(get_amp_swappool_group_invariant([[sp, sp_tokens]]), init_sp_invariant, -small_error_bound, large_error_bound)

    # Check spot prices
    for init_spot_price, new_spot_price in zip(init_sp_spot_prices, get_swappool_spot_prices(sp, sp_tokens)):
        assert abs(relative_error(init_spot_price, new_spot_price)) < large_error_bound

    return RunDepositResult(tx, amount, deposited_asset_balances)


def run_withdraw(
    amount,
    swappool_info,
    withdrawer,
    large_error_bound = 1e-4,
    small_error_bound = 1e-6
) -> RunWithdrawResult:

    sp         = swappool_info.swappool
    sp_tokens  = swappool_info.tokens
    sp_weights = swappool_info.token_weights

    init_sp_token_balances         = [token.balanceOf(sp) for token in sp_tokens]
    init_sp_pool_token_supply      = sp.totalSupply()
    init_sp_withdrawer_pool_tokens = sp.balanceOf(withdrawer)
    init_sp_balance0               = [sp._balance0(token) for token in sp_tokens]
    init_sp_invariant              = get_swappool_group_invariant([[sp, sp_tokens]])
    init_sp_spot_prices            = get_swappool_spot_prices(sp, sp_tokens)

    init_withdrawer_token_balances = [token.balanceOf(withdrawer) for token in sp_tokens]

    if amount is None:
        amount = init_sp_withdrawer_pool_tokens

    # Withdraw everything
    tx = sp.withdrawAll(amount, {"from": withdrawer})

    # Check transaction event
    assert len(tx.events['Withdraw']) == 1
    withdraw_event = tx.events['Withdraw'][0]

    assert withdraw_event['who']  == withdrawer
    assert withdraw_event['burn'] == amount

    withdrawn_asset_balances = withdraw_event['assets']

    # Check pool tokens
    assert sp.totalSupply()         == init_sp_pool_token_supply - amount
    assert sp.balanceOf(withdrawer) == init_sp_withdrawer_pool_tokens - amount

    # Check balance0s
    withdraw_proportion = amount / init_sp_pool_token_supply
    for i, token in enumerate(sp_tokens):
        assert_relative_error(sp._balance0(token), init_sp_balance0[i] * (1 - withdraw_proportion), -large_error_bound, small_error_bound)
    
    # Check withdrawn amounts
    for i, token in enumerate(sp_tokens):

        expected_withdrawn_amount = int(init_sp_token_balances[i] * withdraw_proportion)
        observed_withdrawn_amount = withdrawn_asset_balances[i]

        assert_relative_error(observed_withdrawn_amount, expected_withdrawn_amount, -large_error_bound, small_error_bound)

        # Swap pool balance
        assert token.balanceOf(sp) == init_sp_token_balances[i] - observed_withdrawn_amount
        # Withdrawer balance
        assert token.balanceOf(withdrawer) == init_withdrawer_token_balances[i] + observed_withdrawn_amount

    # Check pool invariant
    assert_relative_error(get_swappool_group_invariant([[sp, sp_tokens]]), init_sp_invariant, -small_error_bound, large_error_bound)

    # Check spot prices
    for init_spot_price, new_spot_price in zip(init_sp_spot_prices, get_swappool_spot_prices(sp, sp_tokens)):
        assert abs(relative_error(init_spot_price, new_spot_price)) < large_error_bound

    return RunWithdrawResult(tx, amount, withdrawn_asset_balances)


def run_amp_withdraw(
    amount,
    swappool_info,
    withdrawer,
    large_error_bound = 1e-4,
    small_error_bound = 1e-6
) -> RunWithdrawResult:

    #TODO to be reviewed once withdraw on amp pools get finalized

    sp         = swappool_info.swappool
    sp_tokens  = swappool_info.tokens
    sp_weights = swappool_info.token_weights

    init_sp_token_balances         = [token.balanceOf(sp) for token in sp_tokens]
    init_sp_pool_token_supply      = sp.totalSupply()
    init_sp_withdrawer_pool_tokens = sp.balanceOf(withdrawer)
    init_sp_balance0               = [sp._balance0(token) for token in sp_tokens]
    init_sp_invariant              = get_amp_swappool_group_invariant([[sp, sp_tokens]])
    init_sp_spot_prices            = get_swappool_spot_prices(sp, sp_tokens)

    init_withdrawer_token_balances = [token.balanceOf(withdrawer) for token in sp_tokens]

    if amount is None:
        amount = init_sp_withdrawer_pool_tokens

    # Withdraw everything
    tx = sp.withdrawAll(amount, {"from": withdrawer})

    # Check transaction event
    assert len(tx.events['Withdraw']) == 1
    withdraw_event = tx.events['Withdraw'][0]

    assert withdraw_event['who']  == withdrawer
    assert withdraw_event['burn'] == amount

    withdrawn_asset_balances = withdraw_event['assets']

    # Check pool tokens
    assert sp.totalSupply()         == init_sp_pool_token_supply - amount
    assert sp.balanceOf(withdrawer) == init_sp_withdrawer_pool_tokens - amount

    # Check balance0s
    withdraw_proportion = amount / init_sp_pool_token_supply
    for i, token in enumerate(sp_tokens):
        assert_relative_error(sp._balance0(token), init_sp_balance0[i] * (1 - withdraw_proportion), -large_error_bound, small_error_bound)
    
    # Check withdrawn amounts
    for i, token in enumerate(sp_tokens):

        expected_withdrawn_amount = int(init_sp_token_balances[i] * withdraw_proportion)
        observed_withdrawn_amount = withdrawn_asset_balances[i]

        assert_relative_error(observed_withdrawn_amount, expected_withdrawn_amount, -large_error_bound, small_error_bound)

        # Swap pool balance
        assert token.balanceOf(sp) == init_sp_token_balances[i] - observed_withdrawn_amount
        # Withdrawer balance
        assert token.balanceOf(withdrawer) == init_withdrawer_token_balances[i] + observed_withdrawn_amount

    # Check pool invariant
    assert_relative_error(get_amp_swappool_group_invariant([[sp, sp_tokens]]), init_sp_invariant, -small_error_bound, large_error_bound)

    # Check spot prices
    for init_spot_price, new_spot_price in zip(init_sp_spot_prices, get_swappool_spot_prices(sp, sp_tokens)):
        assert abs(relative_error(init_spot_price, new_spot_price)) < large_error_bound

    return RunWithdrawResult(tx, amount, withdrawn_asset_balances)



def get_swappool_group_invariant(swappool_tokens_tuples):

    invariant = 1
    for swappool, tokens in swappool_tokens_tuples:
        for token in tokens:
            token_weight = swappool._weight(token)
            invariant *= token.balanceOf(swappool)**token_weight / swappool._balance0(token)**token_weight
    
    return invariant


def get_amp_swappool_group_invariant(swappool_tokens_tuples):

    top_invariant    = 0
    bottom_invariant = 0

    one_minus_amp    = None

    for swappool, tokens in swappool_tokens_tuples:

        if one_minus_amp is None:
            one_minus_amp = 1 - swappool._amp() / 2**64
        else:
            assert one_minus_amp == 1 - swappool._amp() / 2**64     # Make sure all pools share the same amplification constant

        for token in tokens:
            token_weight = swappool._weight(token)
            top_invariant    += token.balanceOf(swappool)**one_minus_amp * token_weight
            bottom_invariant += swappool._balance0(token)**one_minus_amp * token_weight

    
    return top_invariant/bottom_invariant