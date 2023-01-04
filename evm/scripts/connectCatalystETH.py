import brownie
from .collectNetworkInformation import ETHCONFIG, getConfig


def main():
    # ON BSC, we need accounts from BSC
    acct = brownie.accounts.from_mnemonic(
        ETHCONFIG["accounts"][0]["mnemonic"]["phrase"]
    )

    swappool_BSC = getConfig()["bsc"]["swappool"]
    swappool_ETH = getConfig()["eth"]["swappool"]

    # The target pool is on ETH.
    swappool_bytes_BSC = brownie.convert.to_bytes(swappool_BSC.replace("0x", ""))

    # create connection to the pool on ETH
    brownie.SwapPool.at(swappool_ETH).createConnection(
        1234, swappool_bytes_BSC, True, {"from": acct}
    )
