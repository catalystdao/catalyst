import brownie
import web3
import json
from .collectNetworkInformation import ETHCONFIG, BSCCONFIG, getConfig


def main():
    acct = brownie.accounts.from_mnemonic(
        ETHCONFIG["accounts"][0]["mnemonic"]["phrase"]
    )

    swappool_BSC = getConfig()["bsc"]["swappool"]
    swappool_ETH = getConfig()["eth"]["swappool"]
    token0_ETH = getConfig()["eth"]["token0"]

    # The target pool is on ETH.
    swappool_bytes_BSC = brownie.convert.to_bytes(swappool_BSC.replace("0x", ""))

    brownie.Token.at(token0_ETH).approve(swappool_ETH, 2**256 - 1, {"from": acct})
    target = brownie.accounts.from_mnemonic(
        BSCCONFIG["accounts"][0]["mnemonic"]["phrase"]
    )
    target_bytes = brownie.convert.to_bytes(target.address.replace("0x", ""))
    brownie.SwapPool.at(swappool_ETH).swapToUnits(
        1234,
        swappool_bytes_BSC,
        target_bytes,
        brownie.Token.at(token0_ETH),
        0,
        1 * 10**18,
        0,
        {"from": acct},
    )
