import web3 as w3
from web3 import Web3
from web3.middleware import geth_poa_middleware
from emulator_abi import abi as e_abi
from cci_abi import abi as cci_abi
from time import sleep
import os

mumbai_url = os.environ['MUMBAI_ALCHEMY']
sepolia_url = os.environ['SEPOLIA_ALCHEMY']

w3_mumbai = Web3(w3.WebsocketProvider(mumbai_url))
w3_mumbai.middleware_onion.inject(geth_poa_middleware, layer=0)
w3_sepolia = Web3(w3.WebsocketProvider(sepolia_url))

emulator_mumbai = w3_mumbai.eth.contract(address="0xbD0125334A81087bEFa472dDB4755dB4F80b23D0", abi=e_abi)
emulator_sepolia = w3_sepolia.eth.contract(address="0xcAB0F1618A89abF9CaC22D1ad1a4928b5018Ce54", abi=e_abi)

CCI_mumbai = w3_mumbai.eth.contract(address="0xFc44709dD35fe4321f39Fe96a6Cde2F13638896B", abi=cci_abi)
CCI_sepolia = w3_sepolia.eth.contract(address="0x3647d390c083AA81Fc4b6F86A0b39fA3AC6F16a5", abi=cci_abi)

account_mumbai = w3_mumbai.eth.account.from_key(os.environ['PRIVATE_KEY_ROUTER'])
account_sepolia = w3_sepolia.eth.account.from_key(os.environ['PRIVATE_KEY_ROUTER'])

Mumbai = dict(
    target_emulator=emulator_mumbai, target_contract=CCI_mumbai, target_account=account_mumbai,
    sending_emulator=emulator_sepolia, sending_contract=CCI_sepolia, sending_account=account_sepolia,
    target_web3=w3_mumbai, sending_web3=w3_sepolia, target="Mumbai", sending="Sepolia"
)
Sepolia = dict(
    sending_emulator=emulator_mumbai, sending_contract=CCI_mumbai, sending_account=account_mumbai,
    target_emulator=emulator_sepolia, target_contract=CCI_sepolia, target_account=account_sepolia,
    target_web3=w3_sepolia, sending_web3=w3_mumbai, sending="Mumbai", target="Sepolia"
)

def fetch_logs(Chain, fromBlock, toBlock):
    logs = Chain["target_emulator"].events.Packet.getLogs(fromBlock=fromBlock, toBlock=toBlock)
    return logs


def relay(Chain, event):
    target_emulator = Chain["target_emulator"]
    sending_emulator = Chain["sending_emulator"]
    target_contract = Chain["target_contract"]
    sending_contract = Chain["sending_contract"]
    (sending_account, target_account) = (Chain["sending_account"], Chain["target_account"])
    (sending_web3, target_web3) = (Chain["sending_web3"], Chain["target_web3"])
    (sending, target) = (Chain["sending"], Chain["target"])
    
    packet = event["args"]["packet"]
    try:
        tx = target_emulator.functions.execute(
            target_contract.address,
            packet
        ).build_transaction({
            'from': target_account.address,
            'nonce': target_web3.eth.get_transaction_count(target_account.address),
        })
        
        signed_txn = target_web3.eth.account.sign_transaction(tx, private_key=os.environ['PRIVATE_KEY_ROUTER'])
        
        tx_hash = target_web3.eth.send_raw_transaction(signed_txn.rawTransaction)
        
        print("Execute", target, Web3.toHex(tx_hash))
        
        ## Check if transaction has been mined.
        receipt = target_web3.eth.wait_for_transaction_receipt(tx_hash)
        
        if receipt.status != 1:
            tx_timeout = sending_emulator.functions.timeout(
                sending_contract.address,
                packet
            ).build_transaction({
                'from': sending_account.address,
                'nonce': sending_web3.eth.get_transaction_count(sending_account.address),
            })
            signed_txn = sending_web3.eth.account.sign_transaction(tx_timeout, private_key=os.environ['PRIVATE_KEY_ROUTER'])
            tx_hash = sending_web3.eth.send_raw_transaction(signed_txn.rawTransaction)
            print("Ack:", sending, Web3.toHex(tx_hash))
        else:
            tx_ack = sending_emulator.functions.ack(
                sending_contract.address,
                packet
            ).build_transaction({
                'from': sending_account.address,
                'nonce': sending_web3.eth.get_transaction_count(sending_account.address),
            })
            signed_txn = sending_web3.eth.account.sign_transaction(tx_ack, private_key=os.environ['PRIVATE_KEY_ROUTER'])
            tx_hash = sending_web3.eth.send_raw_transaction(signed_txn.rawTransaction)
            print("Ack:", sending, Web3.toHex(tx_hash))
    except Exception as e:
        print(e)
        tx_timeout = sending_emulator.functions.timeout(
            sending_contract.address,
            packet
        ).build_transaction({
            'from': sending_account.address,
            'nonce': sending_web3.eth.get_transaction_count(sending_account.address),
        })
        signed_txn = sending_web3.eth.account.sign_transaction(tx_timeout, private_key=os.environ['PRIVATE_KEY_ROUTER'])
        tx_hash = sending_web3.eth.send_raw_transaction(signed_txn.rawTransaction)
        print("Timeout:", sending, Web3.toHex(tx_hash))


def main():
    print(f"Sepolia account: {account_sepolia.address}, Mumbai account {account_mumbai.address}")
    blockNumber = {"Sepolia": w3_sepolia.eth.blockNumber, "Mumbai": w3_mumbai.eth.blockNumber}
    while True:
        fromBlock = blockNumber["Sepolia"]
        toBlock = w3_sepolia.eth.blockNumber
        
        if fromBlock <= toBlock:
            blockNumber["Sepolia"] = toBlock + 1
            sepolia_logs = fetch_logs(Sepolia, fromBlock, toBlock)
            print(
                f"Sepolia: {len(sepolia_logs)} logs between block {fromBlock}-{toBlock}"
            )
            for log in sepolia_logs:
                relay(Mumbai, log)
        
        fromBlock = blockNumber["Mumbai"]
        toBlock = w3_mumbai.eth.blockNumber
        if fromBlock <= toBlock:
            blockNumber["Mumbai"] = toBlock + 1
            mumbai_logs = fetch_logs(Mumbai, fromBlock, toBlock)
            print(
                f"Mumbai: {len(mumbai_logs)} logs between block {fromBlock}-{toBlock}"
            )
            for log in mumbai_logs:
                relay(Sepolia, log)
            
            sleep(5)
    

if __name__ == "__main__":
    main()