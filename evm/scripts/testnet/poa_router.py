import web3
from web3 import Web3
from web3.middleware import geth_poa_middleware
from time import sleep
import json
import os
import argparse
from hashlib import sha256

with open("vault_abi.json", "r") as f:
    vault_abi = json.load(f)

with open("cci_abi.json", "r") as f:
    cci_abi = json.load(f)
    
with open("emulator_abi.json", "r") as f:
    e_abi = json.load(f)


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

    def compute_sendAsset_identifier(self, log):
        args = log["args"]
        toAccount = args["toAccount"]
        U = args["Units"]
        fee = args["fee"]
        amount = args["fromAmount"] - fee
        fromAsset = args["fromAsset"]
        blockNumberMod = log["blockNumber"] % 2**32-1
        poolAddress = log["address"]
        channelId = args["channelId"]
        return sha256(
            toAccount + str(U).encode() + str(amount).encode() + fromAsset.encode() + str(blockNumberMod).encode() + poolAddress.encode() + channelId
        ).hexdigest()
    
    def compute_sendLiquidity_identifier(self, log):
        args = log["args"]
        toAccount = args["toAccount"]
        U = args["Units"]
        fee = 0
        amount = args["fromAmount"] - fee
        blockNumberMod = log["blockNumber"] % 2**32-1
        poolAddress = log["address"]
        channelId = args["channelId"]
        return sha256(
            toAccount + str(U).encode() + str(amount).encode() + str(blockNumberMod).encode() + poolAddress.encode() + channelId
        ).hexdigest()
    
    def compute_sendAsset_callback(self, log):
        args = log["args"]
        toAccount = args["toAccount"]
        U = args["Units"]
        amount = args["escrowAmount"]
        fromAsset = args["fromAsset"]
        blockNumberMod = log["blockNumberMod"]
        poolAddress = log["address"]
        channelId = args["channelId"]
        return sha256(
            toAccount + str(U).encode() + str(amount).encode() + fromAsset.encode() + str(blockNumberMod).encode() + poolAddress.encode() + channelId
        ).hexdigest()
        
    def compute_sendLiquidity_callback(self, log):
        args = log["args"]
        toAccount = args["toAccount"]
        U = args["Units"]
        amount = args["escrowAmount"]
        blockNumberMod = log["blockNumberMod"]
        poolAddress = log["address"]
        channelId = args["channelId"]
        return sha256(
            toAccount + str(U).encode() + str(amount).encode() + str(blockNumberMod).encode() + poolAddress.encode() + channelId
        ).hexdigest()

    def compute_swap_identifier(self, log):
        if log["event"] == "sendLiquidity":
            return self.compute_sendAsset_identifier(log)
        elif log["event"] == "sendLiquidity":
            return self.compute_sendLiquidity_identifier(log)
        elif log["event"] in ["SendAssetFailure", "SendAssetSuccess"]:
            return self.compute_sendAsset_callback(log)
        elif log["event"] in ["SendLiquidityFailure", "SendLiquiditySuccess"]: 
            return self.compute_sendLiquidity_callback(log)
        else:
            raise NotImplementedError()
        
    def get_sendAssets(self, w3, fromBlock, ibc_endpoint):
        # get a log of all 
        filter = w3.eth.filter({'topics': ["0xde2589d56dbe5139a2f5676038c033a041a897fa65ef9fbd50a5363758e25334"], "fromBlock": fromBlock})
        entries = filter.get_all_entries()
        swap_vault = web3.eth.contract(abi=vault_abi)
        processed_logs = [swap_vault.events.sendAsset().processLog(entry) for entry in entries]
        # For each log, check that our emulator also emitted a message.
        validated_logs = []
        for log in processed_logs:
            receipt = w3.eth.getTransactionReceipt(log["transactionHash"])
            for r_log in receipt.logs:
                if r_log.topics[0].hex() == "0x2f7e9b1a1fac10099a9988fcda077b67b3809ede82c92a7662b82f0f96861604":
                    if r_log.address == ibc_endpoint:
                        validated_logs.append(log)
        
        # We now have an array of Catalyst swaps.
        swap_hashes = [self.compute_swap_indentifier(log) for log in validated_logs]
        
        return swap_hashes
    
    def get_sendLiquidity(self, w3, fromBlock, ibc_endpoint):
        # get a log of all 
        filter = w3.eth.filter({'topics': ["0x42990289d0d5107d040ab63a6b3021825797e28c667b8046161711da1ba20027"], "fromBlock": fromBlock})
        entries = filter.get_all_entries()
        swap_vault = web3.eth.contract(abi=vault_abi)
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
        swap_hashes = [self.compute_swap_indentifier(log) for log in validated_logs]
        
        return swap_hashes

    def get_receiveAsset(self, w3, fromBlock, ibc_endpoint):
        filter = w3.eth.filter({'topics': ["0x153d4669de60199f12c52620612b5b1cee036cefe97aec297f2695f37848432b"], "fromBlock": fromBlock})
        entries = filter.get_all_entries()
        swap_vault = web3.eth.contract(abi=vault_abi)
        processed_logs = [swap_vault.events.ReceiveAsset().processLog(entry) for entry in entries]
        # For each log, check that our emulator also emitted a message.
        validated_logs = []
        for log in processed_logs:
            receipt = w3.eth.getTransactionReceipt(log["transactionHash"])
            # TODO: Check that the call was on ibc_endpoint
        
        # We now have an array of Catalyst swaps.
        swap_hashes = [self.compute_swap_indentifier(log) for log in validated_logs]
        
        return swap_hashes

    def get_receiveLiquidity(self, w3, fromBlock, ibc_endpoint):
        filter = w3.eth.filter({'topics': ["0x9c74cf5512671bd46de8b6b37af7a7a33cc7d367e87bf953dbf0b653e5fab804"], "fromBlock": fromBlock})
        entries = filter.get_all_entries()
        swap_vault = web3.eth.contract(abi=vault_abi)
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
    
    def get_sendAsset_callback(self, w3, fromBlock, ibc_endpoint):
        filter_ack = w3.eth.filter({'topics': ["0xd286b8170ee44400102a0d9468d077acfdde788720f5426f65e36f5732450f82"], "fromBlock": fromBlock})
        entries_ack = filter_ack.get_all_entries()
        filter_timeout = w3.eth.filter({'topics': ["0xf0f7f3715617e78ca6629fbd1817eb745096aade249ebe327ae981a27a71df14"], "fromBlock": fromBlock})
        entries_timeout = filter_timeout.get_all_entries()
        
        entries = entries_ack + entries_timeout
        swap_vault = web3.eth.contract(abi=vault_abi)
        processed_logs = [swap_vault.events.ReceiveAsset().processLog(entry) for entry in entries]
    
    def get_sendLiquidity_callback(self, w3, fromBlock, ibc_endpoint):
        filter_ack = w3.eth.filter({'topics': ["0x95f3aaf194597b0083a807eb7d3c5a4c41b4e87728b90ff06b078117987ecb57"], "fromBlock": fromBlock})
        entries_ack = filter_ack.get_all_entries()
        filter_timeout = w3.eth.filter({'topics': ["0xf68c6a88fa1f6c067c8d250a1bdc5c389b069c77ddf8abd19bdc269c555eb6d2"], "fromBlock": fromBlock})
        entries_timeout = filter_timeout.get_all_entries()
        
        entries = entries_ack + entries_timeout
        swap_vault = web3.eth.contract(abi=vault_abi)
        processed_logs = [swap_vault.events.ReceiveAsset().processLog(entry) for entry in entries]
        
    def get_callbacks(self, w3, ibc_endpoint):
        # We need to get all callbacks.
        # We need to check that the calls were original executed on the ibc_emulator
        # We need to compute the hash.
        pass
    
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
        
    relayer = PoARouter(config_name=config_location)
    relayer.run()
    

if __name__ == "__main__":
    main()