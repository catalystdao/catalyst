import argparse
import json
import logging
import os
from hashlib import sha256
from time import sleep

import web3
from eth_abi import encode
from eth_account import Account
from eth_account.signers.local import LocalAccount
from eth_abi.packed import encode_packed
from web3 import Web3
from web3.middleware import geth_poa
from poa_signer import MessageSigner

with open("external/IncentivizedMockEscrow.json", "r") as f:
    IncentivizedMockEscrow_abi = json.load(f)["abi"]


def decode_chain_from_channel(channelid):
    return channelid[0:16].decode().replace("\x00", "")


def convert_64_bytes_address(address) -> bytes:
    return encode_packed(
        ["uint8", "bytes32", "bytes32"], [20, 0, address]
    )
    

logging.basicConfig(level=logging.INFO)

GeneralizedIncentivesMock = Web3().eth.contract(abi=IncentivizedMockEscrow_abi)


class PoARelayer(MessageSigner):

    def __init__(
        self,
        private_key: str = os.environ["SIGNER"],
        chains={
            1: {
                "name": "scroll",
                "confirmations": 0,
                "url": "http://127.0.0.1:8546",  # os.environ["SCROLL_RPC"],
                # "middleware": geth_poa,
                "GI_contract": Web3.to_checksum_address("0x336615f193affdbeffcaeb2cc61d0dead286e242"),
                "key": os.environ["PRIVATE_KEY_ROUTER"]
            },
            2: {
                "name": "canto",
                "confirmations": 0,
                "url": "http://127.0.0.1:8545",  # os.environ["CRONOS_RPC"],
                "GI_contract": Web3.to_checksum_address("0x4fb0f5fb6c4c0b0b133e958c04465a27aef5bc60"),
                "key": os.environ["PRIVATE_KEY_ROUTER"]
            }
        }
    ):
        MessageSigner.__init__(self, private_key)
        self.private_key = private_key

        self.chains = chains
        for chain in self.chains.keys():
            w3 = Web3(web3.HTTPProvider(self.chains[chain]["url"]))

            middleware = self.chains[chain].get("middleware")
            if middleware is not None:
                w3.middleware_onion.inject(middleware, layer=0)

            self.chains[chain]["w3"] = w3
            self.chains[chain]["GI"] = w3.eth.contract(address=self.chains[chain]["GI_contract"], abi=IncentivizedMockEscrow_abi)
            self.chains[chain]["relayer"] = Account.from_key(self.chains[chain]["key"])
            self.chains[chain]["nonce"] = w3.eth.get_transaction_count(self.chains[chain]["relayer"].address)
        
    def checkConfirmations(self, chainId: int, confirmations: int) -> bool:
        return self.chains[chainId]["confirmations"] >= confirmations

    def signTransaction(self, chainId: int, transactionHash) -> list:
        w3: Web3 = self.chains[chainId]["provider"]
        
        transaction = w3.eth.get_transaction_receipt(transactionHash)
        confirmations: int = w3.eth.block_number - transaction["blockNumber"]
        
        assert self.checkConfirmations(chainId, confirmations), "Not enough confirmations"
        
        logs = transaction["logs"]
        
        signatures: list = []
        for log in logs:
            if log["topics"][0] == "":
                log = GeneralizedIncentivesMock.events.Message().processLog(log)
                
                emitter = log["address"]
                message = log["args"]["message"]
                
                newMessage = encode_packed(["bytes", "bytes"], [encode(["address"], [emitter]), bytes.fromhex(message)]).hex()
                
                sig = self.signMessage(
                    newMessage
                )
                
                signatures.append(
                    [
                        newMessage,
                        encode(["uint8", "bytes32", "bytes32"], [sig.v, sig.r, sig.s])
                    ]
                )
        
        return signatures

    def fetch_logs(self, chain, fromBlock, toBlock):
        logs = self.chains[chain]["GI"].events.Message.get_logs(
            fromBlock=fromBlock, toBlock=toBlock
        )
        return logs

    def execute(self, fromChain, event):
        toChain = event["args"]["destinationIdentifier"]
        
        GI = self.chains[toChain]["GI"]
        
        signatures = self.signTransaction(fromChain, event["transactionHash"])
        assert len(signatures) == 1, f"{len(signatures)} messages found, expected 1."
        signature = signatures[0]
        
        relayer_address = self.chains[toChain]["relayer"]
        
        w3 = self.chains[toChain]["w3"]

        # Execute the transaction on the target side:
        tx = GI.functions.processMessage(
            signature[1], signature[0], encode(["address"], [relayer_address])
        ).build_transaction(
            {
                "from": relayer_address,
                "nonce": self.chains[toChain]["nonce"]
            }
        )

        signed_txn = w3.eth.account.sign_transaction(
            tx, private_key=self.chains[toChain]["key"]
        )

        tx_hash = w3.eth.send_raw_transaction(signed_txn.rawTransaction)
        self.chains[toChain]["nonce"] = self.chains[toChain]["nonce"] + 1

        logging.info(f"Execute: {fromChain} -> {toChain, Web3.toHex(tx_hash)}")

        return tx_hash
          
    def run(self, wait=5):
        chains = self.chains.keys()
        blocknumbers = {}

        for chain in chains:
            blocknumber = self.chains[chain]["w3"].eth.block_number
            logging.info(
                f"Loaded {chain} at block: {blocknumber} with relayer {self.chains[chain]['relayer'].address}"
            )
            blocknumbers[chain] = blocknumber

        while True:
            for chain in chains:
                w3 = self.chains[chain]["w3"]
                fromBlock = blocknumbers[chain]
                toBlock = w3.eth.block_number

                if fromBlock <= toBlock:
                    blocknumbers[chain] = toBlock + 1
                    logs = self.fetch_logs(chain, fromBlock, toBlock)
                    logging.info(
                        f"{chain}: {len(logs)} logs between block {fromBlock}-{toBlock}"
                    )

                    executes = []
                    for log in logs:
                        executes.append((log, self.execute(chain, log)))

            sleep(wait)


def main():
    parser = argparse.ArgumentParser("proxy relayer")
    parser.add_argument(
        "log_location", nargs="?", help="The log location. If not set, print to std-out.", type=str
    )
    args = parser.parse_args()
    if args.log_location:
        # setup log
        logging.basicConfig(level=logging.INFO, filename=args.log_location, filemode="a")
    else:
        logging.basicConfig(level=logging.INFO)

    relayer = PoARelayer()
    relayer.run()


if __name__ == "__main__":
    main()
