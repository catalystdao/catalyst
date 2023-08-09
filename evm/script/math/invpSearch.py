from brownie import FixedPointMathX64, accounts
import numpy as np
import pandas as pd
from random import randrange
from tqdm import trange


# This script tries to figure out what is the cause of large errors when using
# inverse p function of FixedPointMathX64 (x^(-p))

P_RANGE = [2 ** 61 + 2 ** 60, 10 ** 9 * 2 ** 64]


def deploy(contract, *args):
    return contract.deploy(*args, {"from": accounts[0]})


def get_value_range(p):
    # returns False if no value exists for p.
    lowerBound = 2 ** 64 + 2 ** 40
    upperBound = min(12 * 2 ** 64, 2 ** ((41 * 2 ** 64) / p) * 2 ** 64)
    if lowerBound >= upperBound:
        return False

    return [lowerBound, upperBound]


def get_value_and_p():
    while True:
        # get p, P_RANGE
        p = randrange(*P_RANGE)

        # get value range:
        value_range = get_value_range(p)
        if value_range:
            value = randrange(*value_range)
            return [value, p]


def find_error(mathX64, value, p):
    assert np.log2(value / 2 ** 64) * p <= 41 * 2 ** 64, "Invalid p and value."

    calc = mathX64.invfpowX64(value, p) / 2 ** 64
    py = np.power(value / 2 ** 64, -p / 2 ** 64)

    div = calc / py

    return [value, p, div]


def test_iterator(mathX64):
    while True:
        r_vp = get_value_and_p()

        yield find_error(mathX64, *r_vp)


def get_iterator():
    fpm = deploy(FixedPointMathX64)
    return test_iterator(fpm)


def get_its(itterator, num_its=1000):
    return [next(itterator) for i in trange(num_its)]


def main():
    itt = get_iterator()

    its = get_its(itt, 10000)

    pd.DataFrame(its).to_csv("10k its.csv", index=False)
