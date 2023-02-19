import brownie


def get_swap_return(x: int, fromToken, toToken, fromPool, toPool=None) -> int:
    if toPool is None:
        toPool = fromPool
    a = fromToken.balanceOf(fromPool)
    b = toToken.balanceOf(toPool)
    w1 = fromPool._weight(fromToken)
    w2 = toPool._weight(toToken)
    w = w1 / w2
    y = b * (1 - (a / (a + x)) ** w)
    if w1 == w2:
        y = max(y, (b * x) / (a + x))

    return y


max_swap_div = 0.02 / 100


def check_swap_return(y: int, y_control: int, size_control=None, div=max_swap_div):
    if y_control == 0:
        assert y <= y_control
        return

    if size_control is not None:
        if size_control[0] < size_control[1] / 1000:
            assert y <= y_control
            return

    assert 1 + div >= y / y_control >= 1 - div * 100


def return_swap_check(x, y, token2, token1, swappool, acct, allow_revert=False):
    if allow_revert:
        try:
            swappool.localSwap(token2, token1, y, 0, {"from": acct})
        except brownie.exceptions.VirtualMachineError:
            print("Reverted, but that is /mostly/ okay.")
    else:
        token2.approve(swappool, y, {"from": acct})
        swappool.localSwap(token2, token1, y, 0, {"from": acct})
        x2 = token1.balanceOf(acct)

        assert x2 <= x
