import web3
from web3 import Web3
from web3.middleware import geth_poa_middleware
from emulator_abi import abi as e_abi
from cci_abi import abi as cci_abi
from time import sleep
import json
import os
import argparse


class ProxyRelayer:
    def read_config(self):
        with open(self.config_name) as f:
            return json.load(f)
        
    def write_config(self):
        with open(self.config_name, 'w') as f:
            json.dump(self.config, f, indent=4)
    
    def __init__(
        self,
        chains={
            "mumbai": {"url": os.environ['MUMBAI_ALCHEMY'], "middleware": geth_poa_middleware, "key": os.environ['PRIVATE_KEY_ROUTER']},
            "sepolia": {"url": os.environ['SEPOLIA_ALCHEMY'], "key": os.environ['PRIVATE_KEY_ROUTER']}
        },
        config_name="./scripts/deploy_config.json"
    ):
        self.config_name = config_name
        self.config = self.read_config()
        
        self.chains = chains
        for chain in self.chains.keys():
            w3 = Web3(web3.HTTPProvider(self.chains[chain]["url"]))
            
            middleware = self.chains[chain].get("middleware")
            if middleware is not None:
                w3.middleware_onion.inject(middleware, layer=0)
                
            self.chains[chain]["w3"] = w3
            
            # ibcinterface
            self.chains[chain]["ibcinterface"] = w3.eth.contract(address=self.config["chain_config"][chain]["ibcinterface"], abi=e_abi)
            
            # crosschaininterface
            self.chains[chain]["crosschaininterface"] = w3.eth.contract(address=self.config["chain_config"][chain]["crosschaininterface"], abi=cci_abi)
            
            # Router
            self.chains[chain]["acct"] = w3.eth.account.from_key(self.chains[chain]["key"])
            
    def fetch_logs(self, chain, fromBlock, toBlock):
        logs = self.chains[chain]["ibcinterface"].events.Packet.getLogs(fromBlock=fromBlock, toBlock=toBlock)
        return logs

    def relay(self, from_chain, event):
        packet = event["args"]["packet"]
        target_chain = packet[1][1]
        target_chain = target_chain.decode().replace("\x00", "")
        relayer_address = self.chains[target_chain]["acct"].address
        
        try:
            # Execute the transaction on the target side:
            target_ibcinterface = self.chains[target_chain]["ibcinterface"]
            target_cci = self.chains[target_chain]["crosschaininterface"]
            target_w3 = self.chains[target_chain]["w3"]
            tx = target_ibcinterface.functions.execute(
                target_cci.address,
                packet
            ).build_transaction({
                'from': relayer_address,
                'nonce': target_w3.eth.get_transaction_count(relayer_address),
                "gas": 300000
            })
            
            signed_txn = target_w3.eth.account.sign_transaction(tx, private_key=self.chains[target_chain]["key"])
            
            tx_hash = target_w3.eth.send_raw_transaction(signed_txn.rawTransaction)
            
            print("Execute", target_chain, Web3.toHex(tx_hash))
            
            # Check if transaction has been mined.
            receipt = target_w3.eth.wait_for_transaction_receipt(tx_hash)
        
            sending_ibcinterface = self.chains[from_chain]["ibcinterface"]
            sending_cci = self.chains[from_chain]["crosschaininterface"]
            sending_w3 = self.chains[from_chain]["w3"]
            if receipt.status != 1:
                tx_timeout = sending_ibcinterface.functions.timeout(
                    sending_cci.address,
                    packet
                ).build_transaction({
                    'from': relayer_address,
                    'nonce': sending_w3.eth.get_transaction_count(relayer_address),
                })
                signed_txn = sending_w3.eth.account.sign_transaction(tx_timeout, private_key=self.chains[from_chain]["key"])
                tx_hash = sending_w3.eth.send_raw_transaction(signed_txn.rawTransaction)
                print("Timeout:", from_chain, Web3.toHex(tx_hash))
            else:
                ack = sending_ibcinterface.events.Acknowledgement().processReceipt(receipt)[0]["args"]["acknowledgement"]
                tx_ack = sending_ibcinterface.functions.ack(
                    sending_cci.address,
                    ack,
                    packet
                ).build_transaction({
                    'from': relayer_address,
                    'nonce': sending_w3.eth.get_transaction_count(relayer_address),
                })
                signed_txn = sending_w3.eth.account.sign_transaction(tx_ack, private_key=self.chains[from_chain]["key"])
                tx_hash = sending_w3.eth.send_raw_transaction(signed_txn.rawTransaction)
                print("Ack:", from_chain, Web3.toHex(tx_hash))
        except Exception as e:
            print(e)
            sending_ibcinterface = self.chains[from_chain]["ibcinterface"]
            sending_cci = self.chains[from_chain]["crosschaininterface"]
            sending_w3 = self.chains[from_chain]["w3"]
            tx_timeout = sending_ibcinterface.functions.timeout(
                sending_cci.address,
                packet
            ).build_transaction({
                'from': relayer_address,
                'nonce': sending_w3.eth.get_transaction_count(relayer_address),
            })
            signed_txn = sending_w3.eth.account.sign_transaction(tx_timeout, private_key=self.chains[from_chain]["key"])
            tx_hash = sending_w3.eth.send_raw_transaction(signed_txn.rawTransaction)
            print("error -> Timeout:", from_chain, Web3.toHex(tx_hash))

    def run(self, wait=5):
        chains = self.chains.keys()
        blocknumbers = {}
        
        for chain in chains:
            blocknumber = self.chains[chain]['w3'].eth.blockNumber
            print(f"Loaded {chain} at block: {blocknumber} with relayer {self.chains[chain]['acct'].address}")
            blocknumbers[chain] = blocknumber
    
        while True:
            for chain in chains:
                w3 = self.chains[chain]['w3']
                fromBlock = blocknumbers[chain]
                toBlock = w3.eth.blockNumber
            
                if fromBlock <= toBlock:
                    blocknumbers[chain] = toBlock + 1
                    logs = self.fetch_logs(chain, fromBlock, toBlock)
                    print(
                        f"{chain}: {len(logs)} logs between block {fromBlock}-{toBlock}"
                    )
                    for log in logs:
                        self.relay(chain, log)
            
            sleep(wait)


def main():
    parser = argparse.ArgumentParser("proxy relayer")
    parser.add_argument("config_location", nargs='?', help="The path to the config location", type=str)
    args = parser.parse_args()
    config_location = "./scripts/deploy_config.json"
    if args.config_location:
        config_location = args.config_location
        
    relayer = ProxyRelayer(config_name=config_location)
    relayer.run()
    

if __name__ == "__main__":
    main()
