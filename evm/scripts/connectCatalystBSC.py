import brownie
from .collectNetworkInformation import BSCCONFIG, getConfig


def main():
    # ON BSC, we need accounts from BSC
    acct = brownie.accounts.from_mnemonic(
        BSCCONFIG["accounts"][0]["mnemonic"]["phrase"]
    )

    swappool_BSC = getConfig()["bsc"]["swappool"]
    swappool_ETH = getConfig()["eth"]["swappool"]

    # The target pool is on ETH.
    swappool_bytes_ETH = brownie.convert.to_bytes(swappool_ETH.replace("0x", ""))

    # create connection to the pool on ETH
    brownie.SwapPool.at(swappool_BSC).createConnection(
        1337, swappool_bytes_ETH, True, {"from": acct}
    )
