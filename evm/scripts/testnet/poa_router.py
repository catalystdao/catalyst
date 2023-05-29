import argparse
import json
import os
from hashlib import sha256
from time import sleep
from brownie import convert

import web3
from web3 import Web3
from web3.middleware import geth_poa_middleware

with open("vault_abi.json", "r") as f:
    vault_abi = json.load(f)

with open("cci_abi.json", "r") as f:
    cci_abi = json.load(f)
    
with open("emulator_abi.json", "r") as f:
    e_abi = json.load(f)

def decode_chain_from_channel(channelid):
    return channelid[0:16].decode().replace("\x00", "")

def convert_64_bytes_address(address):
    return convert.to_bytes(20, "bytes1")+convert.to_bytes(0)+convert.to_bytes(address.replace("0x", ""))

class PoARouter:
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
        target_chain = decode_chain_from_channel(packet[1][1])
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

    def compute_sendAsset_identifier(self, log, chain_identifier):
        args = log["args"]
        toAccount = args["toAccount"]
        U = args["units"]
        fee = args["fee"]
        amount = args["fromAmount"] - fee
        fromAsset = args["fromAsset"]
        blockNumberMod = log["blockNumber"] % 2**32
        poolAddress = log["address"]
        return sha256(
            toAccount + str(U).encode() + str(amount).encode() + convert_64_bytes_address(fromAsset) + str(blockNumberMod).encode() + poolAddress.encode() + chain_identifier
        ).hexdigest()
    
    def compute_receiveAsset_identifier(self, log):
        args = log["args"]
        toAccount = args["toAccount"]
        U = args["units"]
        amount = args["fromAmount"]
        fromAsset = args["fromAsset"]
        blockNumberMod = args["sourceBlockNumberMod"]
        poolAddress = log["address"]
        channelId = args["channelId"]
        return sha256(
            convert_64_bytes_address(toAccount) + str(U).encode() + str(amount).encode() + fromAsset + str(blockNumberMod).encode() + poolAddress.encode() + channelId
        ).hexdigest()
    
    def compute_sendLiquidity_identifier(self, log, chain_identifier):
        args = log["args"]
        toAccount = args["toAccount"]
        U = args["units"]
        fee = 0
        amount = args["fromAmount"] - fee
        blockNumberMod = log["blockNumber"] % 2**32
        poolAddress = log["address"]
        return sha256(
            toAccount + str(U).encode() + str(amount).encode() + str(blockNumberMod).encode() + poolAddress.encode() + chain_identifier
        ).hexdigest()
    
    def compute_receiveLiquidity_identifier(self, log):
        args = log["args"]
        toAccount = args["toAccount"]
        U = args["units"]
        amount = args["fromAmount"]
        blockNumberMod = args["sourceBlockNumberMod"]
        poolAddress = log["address"]
        channelId = args["channelId"]
        return sha256(
            toAccount + str(U).encode() + str(amount).encode() + str(blockNumberMod).encode() + poolAddress.encode() + channelId
        ).hexdigest()
    
    def compute_sendAsset_callback(self, log, chain_identifier):
        args = log["args"]
        toAccount = args["toAccount"]
        U = args["U"]
        amount = args["escrowAmount"]
        fromAsset = args["escrowToken"]
        blockNumberMod = args["blockNumberMod"]
        poolAddress = log["address"]
        return sha256(
            toAccount + str(U).encode() + str(amount).encode() + convert_64_bytes_address(fromAsset) + str(blockNumberMod).encode() + poolAddress.encode() + chain_identifier
        ).hexdigest()
        
    def compute_sendLiquidity_callback(self, log, chain_identifier):
        args = log["args"]
        toAccount = args["toAccount"]
        U = args["U"]
        amount = args["escrowAmount"]
        blockNumberMod = args["blockNumberMod"]
        poolAddress = log["address"]
        return sha256(
            toAccount + str(U).encode() + str(amount).encode() + str(blockNumberMod).encode() + poolAddress.encode() + chain_identifier
        ).hexdigest()

    def compute_swap_identifier(self, log, chain_identifier):
        if log["event"] == "SendAsset":
            return self.compute_sendAsset_identifier(log, chain_identifier)
        elif log["event"] == "ReceiveAsset":
            return self.compute_receiveAsset_identifier(log)
        elif log["event"] == "SendLiquidity":
            return self.compute_sendLiquidity_identifier(log, chain_identifier)
        elif log["event"] == "ReceiveLiquidity":
            return self.compute_receiveLiquidity_identifier(log)
        elif log["event"] in ["SendAssetFailure", "SendAssetSuccess"]:
            return self.compute_sendAsset_callback(log, chain_identifier)
        elif log["event"] in ["SendLiquidityFailure", "SendLiquiditySuccess"]: 
            return self.compute_sendLiquidity_callback(log, chain_identifier)
        else:
            raise NotImplementedError()
        
    def get_sendAssets(self, w3, fromBlock, ibc_endpoint, chain_identifier):
        # get a log of all 
        filter = w3.eth.filter({'topics': ["0xe1c4c822c15df23f17ad636820a990981caf1d4e40f2f46cf3bb7ad003deaec8"], "fromBlock": fromBlock})
        entries = filter.get_all_entries()
        print(len(entries))
        swap_vault = w3.eth.contract(abi=vault_abi)
        processed_logs = [swap_vault.events.SendAsset().processLog(entry) for entry in entries]
        # For each log, check that our emulator also emitted a message.
        validated_logs = []
        for log in processed_logs:
            receipt = w3.eth.getTransactionReceipt(log["transactionHash"])
            for r_log in receipt.logs:
                if r_log.topics[0].hex() == "0x2f7e9b1a1fac10099a9988fcda077b67b3809ede82c92a7662b82f0f96861604":
                    if r_log.address == ibc_endpoint:
                        validated_logs.append(log)
                    
        
        # We now have an array of Catalyst swaps.
        swap_hashes = [self.compute_swap_identifier(log, chain_identifier) for log in validated_logs]
        
        return swap_hashes
    
    def get_sendLiquidity(self, w3, fromBlock, ibc_endpoint, chain_identifier):
        # get a log of all 
        filter = w3.eth.filter({'topics': ["0x8c9503be4db35b4e3d31565a9616d1dc3f1b3024e5e9e9d65052de46a5149f1c"], "fromBlock": fromBlock})
        entries = filter.get_all_entries()
        swap_vault = w3.eth.contract(abi=vault_abi)
        processed_logs = [swap_vault.events.SendLiquidity().processLog(entry) for entry in entries]
        # For each log, check that our emulator also emitted a message.
        validated_logs = []
        for log in processed_logs:
            receipt = w3.eth.getTransactionReceipt(log["transactionHash"])
            for r_log in receipt.logs:
                if r_log.topics[0].hex() == "0x2f7e9b1a1fac10099a9988fcda077b67b3809ede82c92a7662b82f0f96861604":
                    if r_log.address == ibc_endpoint:
                        validated_logs.append(log)
        
        # We now have an array of Catalyst swaps.
        swap_hashes = [self.compute_swap_identifier(log, chain_identifier) for log in validated_logs]
        
        return swap_hashes

    def get_receiveAsset(self, w3, fromBlock, ibc_endpoint, chain_identifier):
        filter = w3.eth.filter({'topics': ["0x6b7977bd09a2e845fb431e372aac95edfb358014e167149b4f4d09021c87a79d"], "fromBlock": fromBlock})
        entries = filter.get_all_entries()
        swap_vault = w3.eth.contract(abi=vault_abi)
        processed_logs = [swap_vault.events.ReceiveAsset().processLog(entry) for entry in entries]
        # For each log, check that the emulator was called.
        validated_logs = []
        for log in processed_logs:
            receipt = w3.eth.getTransactionReceipt(log["transactionHash"])
            if receipt["to"] == ibc_endpoint:
                validated_logs.append(log)
        
        # We now have an array of Catalyst swaps.
        swap_hashes = [self.compute_swap_identifier(log, chain_identifier) for log in validated_logs]
        
        return swap_hashes

    def get_receiveLiquidity(self, w3, fromBlock, ibc_endpoint, chain_identifier):
        filter = w3.eth.filter({'topics': ["0x7af4b988c9949d39dbe6398b8332fa201574208c2656602a23f1624c428bfe91"], "fromBlock": fromBlock})
        entries = filter.get_all_entries()
        swap_vault = w3.eth.contract(abi=vault_abi)
        processed_logs = [swap_vault.events.ReceiveAsset().processLog(entry) for entry in entries]
        # For each log, check that our emulator also emitted a message.
        validated_logs = []
        for log in processed_logs:
            receipt = w3.eth.getTransactionReceipt(log["transactionHash"])
            if receipt["to"] == ibc_endpoint:
                validated_logs.append(log)
        
        # We now have an array of Catalyst swaps.
        swap_hashes = [self.compute_swap_indentifier(log) for log in validated_logs]
        
        return swap_hashes
    
    def get_sendAsset_callback(self, w3, fromBlock, ibc_endpoint, chain_identifier):
        filter_ack = w3.eth.filter({'topics': ["0xe6db00361b6a35af0ded81ba5696c1633e945a81008cd7da44fb8a78422a7d42"], "fromBlock": fromBlock})
        entries_ack = filter_ack.get_all_entries()
        filter_timeout = w3.eth.filter({'topics': ["0xcab6c1a18a9c89efaab5ea5a8c665ffe2c5aac9ddd9301ccad01fd4fed7c7e3d"], "fromBlock": fromBlock})
        entries_timeout = filter_timeout.get_all_entries()
        
        swap_vault = w3.eth.contract(abi=vault_abi)
        processed_acks = [swap_vault.events.SendAssetSuccess().processLog(entry) for entry in entries_ack]
        processed_timeouts = [swap_vault.events.SendAssetFailure().processLog(entry) for entry in entries_timeout]
        processed_logs = processed_acks + processed_timeouts
        
        # For each log, check that the emulator was called.
        validated_logs = []
        for log in processed_logs:
            receipt = w3.eth.getTransactionReceipt(log["transactionHash"])
            if receipt["to"] == ibc_endpoint:
                validated_logs.append(log)
                
        # We now have an array of Catalyst swaps.
        swap_hashes = [self.compute_swap_identifier(log, chain_identifier) for log in validated_logs]
        
        return swap_hashes
    
    def get_sendLiquidity_callback(self, w3, fromBlock, ibc_endpoint, chain_identifier):
        filter_ack = w3.eth.filter({'topics': ["0x8a49f1dbb0b988d0421183f74b9866ce7c88256f1b88cf865bf7f3a74706fe68"], "fromBlock": fromBlock})
        entries_ack = filter_ack.get_all_entries()
        filter_timeout = w3.eth.filter({'topics': ["0x97cc161fb90f5cdec9c65ba7aac2279e32df11368946590b82fd6fe8e76b39e0"], "fromBlock": fromBlock})
        entries_timeout = filter_timeout.get_all_entries()
        
        entries = entries_ack + entries_timeout
        swap_vault = w3.eth.contract(abi=vault_abi)
        processed_logs = [swap_vault.events.ReceiveAsset().processLog(entry) for entry in entries]
        
    def get_callbacks(self, w3, ibc_endpoint, chain_identifier):
        # We need to get all callbacks.
        # We need to check that the calls were original executed on the ibc_emulator
        # We need to compute the hash.
        pass

    def backcheck(self):
        chain = "sepolia"
        w3 = self.chains[chain]['w3']
        fromBlock = 0
        ibc_emulator = self.chains[chain]["ibcinterface"]
        a = self.get_sendAssets(w3, fromBlock, ibc_emulator.address, chain_identifier=convert.to_bytes(chain.encode()))
        chain = "mumbai"
        w3 = self.chains[chain]['w3']
        fromBlock = 0
        ibc_emulator = self.chains[chain]["ibcinterface"]
        b = self.get_receiveAsset(w3, fromBlock, ibc_emulator.address, chain_identifier=convert.to_bytes(chain.encode()))
        
        chain = "sepolia"
        w3 = self.chains[chain]['w3']
        fromBlock = 0
        ibc_emulator = self.chains[chain]["ibcinterface"]
        c = self.get_sendAsset_callback(w3, fromBlock, ibc_emulator.address, chain_identifier=convert.to_bytes(chain.encode()))
        
        return (a, b, c)
    
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
                fromBlock = 3583227 #blocknumbers[chain]
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
        
    relayer = PoARouter(config_name=config_location)
    relayer.run()
    

if __name__ == "__main__":
    main()
    
    
    
    
    
    