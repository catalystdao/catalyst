from brownie import FixedPointMathX64, accounts
import numpy as np
import pandas as pd
from random import randrange
from tqdm import trange


# This script tries to figure out what is the cause of large errors when using
# inverse p function of FixedPointMathX64 (x^(-p))

VALUE_RANGE = [2 ** 61, 16 * 2 ** 64]


def deploy(contract, *args):
    return contract.deploy(*args, {"from": accounts[0]})


def get_value():
    while True:
        # get p, P_RANGE

        # get value range:
        value_range = VALUE_RANGE
        if value_range:
            value = randrange(*value_range)
            return [value]


def find_error(mathX64, value):
    calc = mathX64.invExpX64(value) / 2 ** 64
    py = np.exp(value/2**64)

    div = calc / py

    return [value, div]


def test_iterator(mathX64):
    while True:
        r_v = get_value()

        yield find_error(mathX64, *r_v)


def get_iterator():
    fpm = deploy(FixedPointMathX64)
    return test_iterator(fpm)


def get_its(itterator, num_its=1000):
    return [next(itterator) for i in trange(num_its)]


def main():
    itt = get_iterator()

    its = get_its(itt, 10000)

    pd.DataFrame(its).to_csv("10k its.csv", index=False)
