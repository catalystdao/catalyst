import brownie
from .collectNetworkInformation import getConfig
from time import time, sleep
import sys

TIMEOUT = 60 * 8


animation = "|/-\\"


def main():
    starting_height = brownie.chain.height
    print(f"Current chain height {starting_height}, awaiting height {starting_height+1}")
    passed = False
    start = time()
    i = 0
    while time() < start + TIMEOUT:
        sys.stdout.write(f"\r {animation[i % len(animation)]} {time()-start:.1f}s, chain height {brownie.chain.height} ")
        sys.stdout.flush()
        if brownie.chain.height > starting_height:
            tx_hash = brownie.chain[-1].transactions[0]
            tx = brownie.chain.get_transaction(tx_hash)
            try:
                if tx.events["SwapFromUnits"]["_output"] > 0:
                    print(f'\n\n{tx.events["SwapFromUnits"]["_output"]/10**18:.6f} Tokens were swapped! 🎉')
                    passed = True
                    break
            except:
                pass

        sleep(0.1)
        i += 1

    assert passed, "Test timed-out."
    print(f"Test passed. Took {(time()-start)/60:.1f} minutes")
