import argparse
import json
import logging
import os
from hashlib import sha256
from time import sleep

import web3
from brownie import convert
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
    return (
        convert.to_bytes(20, "bytes1")
        + convert.to_bytes(0)
        + convert.to_bytes(address.replace("0x", ""))
    )


logging.basicConfig(level=logging.INFO)

class PoARelayer:
    def read_config(self):
        with open(self.config_name) as f:
            return json.load(f)

    def get_channel_pair(self, channelId):
        return bytes.fromhex(self.config["channel_pairs"][channelId.hex()])

    def __init__(
        self,
        chains={
            "mumbai": {
                "url": os.environ["MUMBAI_ALCHEMY"],
                "middleware": geth_poa_middleware,
                "key": os.environ["PRIVATE_KEY_ROUTER"],
            },
            "sepolia": {
                "url": os.environ["SEPOLIA_ALCHEMY"],
                "key": os.environ["PRIVATE_KEY_ROUTER"],
            },
        },
        config_name="./scripts/deploy_config.json",
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
            self.chains[chain]["ibcinterface"] = w3.eth.contract(
                address=self.config["chain_config"][chain]["ibcinterface"], abi=e_abi
            )

            # crosschaininterface
            self.chains[chain]["crosschaininterface"] = w3.eth.contract(
                address=self.config["chain_config"][chain]["crosschaininterface"],
                abi=cci_abi,
            )

            # Router
            self.chains[chain]["acct"] = w3.eth.account.from_key(
                self.chains[chain]["key"]
            )
            self.chains[chain]["nonce"] = w3.eth.get_transaction_count(
                self.chains[chain]["acct"].address
            )

    def fetch_logs(self, chain, fromBlock, toBlock):
        logs = self.chains[chain]["ibcinterface"].events.Packet.getLogs(
            fromBlock=fromBlock, toBlock=toBlock
        )
        return logs

    def execute(self, from_chain, event):
        packet = event["args"]["packet"]
        target_chain = decode_chain_from_channel(packet[1][1])
        relayer_address = self.chains[target_chain]["acct"].address

        try:
            # Execute the transaction on the target side:
            target_ibcinterface = self.chains[target_chain]["ibcinterface"]
            target_cci = self.chains[target_chain]["crosschaininterface"]
            target_w3 = self.chains[target_chain]["w3"]
            tx = target_ibcinterface.functions.execute(
                target_cci.address, packet
            ).build_transaction(
                {
                    "from": relayer_address,
                    "nonce": self.chains[target_chain]["nonce"]
                }
            )
            self.chains[target_chain]["nonce"] = self.chains[target_chain]["nonce"] + 1

            signed_txn = target_w3.eth.account.sign_transaction(
                tx, private_key=self.chains[target_chain]["key"]
            )

            tx_hash = target_w3.eth.send_raw_transaction(signed_txn.rawTransaction)

            logging.info(f"Execute: {target_chain, Web3.toHex(tx_hash)}")

            return tx_hash
        except Exception as e:
            logging.error(e)
            sending_ibcinterface = self.chains[from_chain]["ibcinterface"]
            sending_cci = self.chains[from_chain]["crosschaininterface"]
            sending_w3 = self.chains[from_chain]["w3"]
            tx_timeout = sending_ibcinterface.functions.timeout(
                sending_cci.address, packet
            ).build_transaction(
                {
                    "from": relayer_address,
                    "nonce": self.chains[from_chain]["nonce"],
                }
            )
            self.chains[from_chain]["nonce"] = self.chains[from_chain]["nonce"] + 1
            signed_txn = sending_w3.eth.account.sign_transaction(
                tx_timeout, private_key=self.chains[from_chain]["key"]
            )
            tx_hash = sending_w3.eth.send_raw_transaction(signed_txn.rawTransaction)
            logging.error("error -> Timeout:", from_chain, Web3.toHex(tx_hash))

    def callback(self, from_chain, event, tx_hash):
        packet = event["args"]["packet"]
        target_chain = decode_chain_from_channel(packet[1][1])
        relayer_address = self.chains[target_chain]["acct"].address

        target_w3 = self.chains[target_chain]["w3"]
        receipt = target_w3.eth.wait_for_transaction_receipt(tx_hash)

        sending_ibcinterface = self.chains[from_chain]["ibcinterface"]
        sending_cci = self.chains[from_chain]["crosschaininterface"]
        sending_w3 = self.chains[from_chain]["w3"]
        if receipt.status != 1:
            tx_timeout = sending_ibcinterface.functions.timeout(
                sending_cci.address, packet
            ).build_transaction(
                {
                    "from": relayer_address,
                    "nonce": self.chains[from_chain]["nonce"],
                }
            )
            self.chains[from_chain]["nonce"] = self.chains[from_chain]["nonce"] + 1
            signed_txn = sending_w3.eth.account.sign_transaction(
                tx_timeout, private_key=self.chains[from_chain]["key"]
            )
            tx_hash = sending_w3.eth.send_raw_transaction(signed_txn.rawTransaction)
            logging.info(f"Timeout: {from_chain, Web3.toHex(tx_hash)}")
        else:
            ack = sending_ibcinterface.events.Acknowledgement().processReceipt(receipt)[
                0
            ]["args"]["acknowledgement"]
            tx_ack = sending_ibcinterface.functions.ack(
                sending_cci.address, ack, packet
            ).build_transaction(
                {
                    "from": relayer_address,
                    "nonce": self.chains[from_chain]["nonce"],
                }
            )
            self.chains[from_chain]["nonce"] = self.chains[from_chain]["nonce"] + 1
            signed_txn = sending_w3.eth.account.sign_transaction(
                tx_ack, private_key=self.chains[from_chain]["key"]
            )
            tx_hash = sending_w3.eth.send_raw_transaction(signed_txn.rawTransaction)
            logging.info(f"Ack:{from_chain, Web3.toHex(tx_hash)}")

    def compute_sendAsset_identifier(self, log):
        args = log["args"]
        toAccount = args["toAccount"]
        U = args["units"]
        fee = args["fee"]
        amount = args["fromAmount"] - fee
        fromAsset = args["fromAsset"]
        blockNumberMod = log["blockNumber"] % 2**32
        poolAddress = log["address"]
        channelId = self.get_channel_pair(args["channelId"])
        return sha256(
            toAccount
            + str(U).encode()
            + str(amount).encode()
            + convert_64_bytes_address(fromAsset)
            + str(blockNumberMod).encode()
            + poolAddress.encode()
            + channelId
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
            convert_64_bytes_address(toAccount)
            + str(U).encode()
            + str(amount).encode()
            + fromAsset
            + str(blockNumberMod).encode()
            + poolAddress.encode()
            + channelId
        ).hexdigest()

    def compute_sendLiquidity_identifier(self, log):
        args = log["args"]
        toAccount = args["toAccount"]
        U = args["units"]
        fee = 0
        amount = args["fromAmount"] - fee
        blockNumberMod = log["blockNumber"] % 2**32
        poolAddress = log["address"]
        channelId = self.get_channel_pair(args["channelId"])
        return sha256(
            toAccount
            + str(U).encode()
            + str(amount).encode()
            + str(blockNumberMod).encode()
            + poolAddress.encode()
            + channelId
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
            convert_64_bytes_address(toAccount)
            + str(U).encode()
            + str(amount).encode()
            + str(blockNumberMod).encode()
            + poolAddress.encode()
            + channelId
        ).hexdigest()

    def compute_sendAsset_callback(self, log):
        args = log["args"]
        toAccount = args["toAccount"]
        U = args["units"]
        amount = args["escrowAmount"]
        fromAsset = args["escrowToken"]
        blockNumberMod = args["blockNumberMod"]
        poolAddress = log["address"]
        channelId = self.get_channel_pair(args["channelId"])
        return sha256(
            toAccount
            + str(U).encode()
            + str(amount).encode()
            + convert_64_bytes_address(fromAsset)
            + str(blockNumberMod).encode()
            + poolAddress.encode()
            + channelId
        ).hexdigest()

    def compute_sendLiquidity_callback(self, log):
        args = log["args"]
        toAccount = args["toAccount"]
        U = args["units"]
        amount = args["escrowAmount"]
        blockNumberMod = args["blockNumberMod"]
        poolAddress = log["address"]
        channelId = self.get_channel_pair(args["channelId"])
        return sha256(
            toAccount
            + str(U).encode()
            + str(amount).encode()
            + str(blockNumberMod).encode()
            + poolAddress.encode()
            + channelId
        ).hexdigest()

    def compute_swap_identifier(self, log):
        if log["event"] == "SendAsset":
            return self.compute_sendAsset_identifier(log)
        elif log["event"] == "ReceiveAsset":
            return self.compute_receiveAsset_identifier(log)
        elif log["event"] == "SendLiquidity":
            return self.compute_sendLiquidity_identifier(log)
        elif log["event"] == "ReceiveLiquidity":
            return self.compute_receiveLiquidity_identifier(log)
        elif log["event"] in ["SendAssetFailure", "SendAssetSuccess"]:
            return self.compute_sendAsset_callback(log)
        elif log["event"] in ["SendLiquidityFailure", "SendLiquiditySuccess"]:
            return self.compute_sendLiquidity_callback(log)
        else:
            raise NotImplementedError()

    def get_sendAssets(self, w3, fromBlock, ibc_endpoint):
        # get a log of all
        filter = w3.eth.filter(
            {
                "topics": [
                    "0xe1c4c822c15df23f17ad636820a990981caf1d4e40f2f46cf3bb7ad003deaec8"
                ],
                "fromBlock": fromBlock,
            }
        )
        entries = filter.get_all_entries()
        swap_vault = w3.eth.contract(abi=vault_abi)
        processed_logs = [
            swap_vault.events.SendAsset().processLog(entry) for entry in entries
        ]

        # For each log, check that our emulator also emitted a message.
        validated_logs = []
        for log in processed_logs:
            receipt = w3.eth.getTransactionReceipt(log["transactionHash"])
            log_index = log.logIndex
            for r_log in receipt.logs:
                if (r_log.logIndex in [log_index - 1, log_index - 2, log_index - 3, log_index - 4]):
                    if (
                        r_log.topics[0].hex()
                        == "0x2f7e9b1a1fac10099a9988fcda077b67b3809ede82c92a7662b82f0f96861604"
                    ):
                        if r_log.address == ibc_endpoint:
                            validated_logs.append(log)
                            break

        # We now have an array of Catalyst swaps.
        swap_hashes = [
            (log["transactionHash"].hex(), self.compute_swap_identifier(log))
            for log in validated_logs
        ]

        return swap_hashes

    def get_sendLiquidity(self, w3, fromBlock, ibc_endpoint):
        # get a log of all
        filter = w3.eth.filter(
            {
                "topics": [
                    "0x8c9503be4db35b4e3d31565a9616d1dc3f1b3024e5e9e9d65052de46a5149f1c"
                ],
                "fromBlock": fromBlock,
            }
        )
        entries = filter.get_all_entries()
        swap_vault = w3.eth.contract(abi=vault_abi)
        processed_logs = [
            swap_vault.events.SendLiquidity().processLog(entry) for entry in entries
        ]

        # For each log, check that our emulator also emitted a message.
        validated_logs = []
        for log in processed_logs:
            receipt = w3.eth.getTransactionReceipt(log["transactionHash"])
            log_index = log.logIndex
            for r_log in receipt.logs:
                if (r_log.logIndex in [log_index - 1, log_index - 2, log_index - 3, log_index - 4]):
                    if (
                        r_log.topics[0].hex()
                        == "0x2f7e9b1a1fac10099a9988fcda077b67b3809ede82c92a7662b82f0f96861604"
                    ):
                        if r_log.address == ibc_endpoint:
                            validated_logs.append(log)
                            break

        # We now have an array of Catalyst swaps.
        swap_hashes = [
            (log["transactionHash"].hex(), self.compute_swap_identifier(log))
            for log in validated_logs
        ]

        return swap_hashes

    def get_receiveAsset(self, w3, fromBlock, ibc_endpoint):
        filter = w3.eth.filter(
            {
                "topics": [
                    "0x6b7977bd09a2e845fb431e372aac95edfb358014e167149b4f4d09021c87a79d"
                ],
                "fromBlock": fromBlock,
            }
        )
        entries = filter.get_all_entries()
        swap_vault = w3.eth.contract(abi=vault_abi)
        processed_logs = [
            swap_vault.events.ReceiveAsset().processLog(entry) for entry in entries
        ]

        # For each log, check that the emulator was called.
        validated_logs = []
        for log in processed_logs:
            receipt = w3.eth.getTransactionReceipt(log["transactionHash"])
            if receipt["to"] == ibc_endpoint:
                validated_logs.append(log)

        # We now have an array of Catalyst swaps.
        swap_hashes = [
            (log["transactionHash"].hex(), self.compute_swap_identifier(log))
            for log in validated_logs
        ]

        return swap_hashes

    def get_receiveLiquidity(self, w3, fromBlock, ibc_endpoint):
        filter = w3.eth.filter(
            {
                "topics": [
                    "0x7af4b988c9949d39dbe6398b8332fa201574208c2656602a23f1624c428bfe91"
                ],
                "fromBlock": fromBlock,
            }
        )
        entries = filter.get_all_entries()
        swap_vault = w3.eth.contract(abi=vault_abi)
        processed_logs = [
            swap_vault.events.ReceiveLiquidity().processLog(entry) for entry in entries
        ]

        # For each log, check that our emulator also emitted a message.
        validated_logs = []
        for log in processed_logs:
            receipt = w3.eth.getTransactionReceipt(log["transactionHash"])
            if receipt["to"] == ibc_endpoint:
                validated_logs.append(log)

        # We now have an array of Catalyst swaps.
        swap_hashes = [
            (log["transactionHash"].hex(), self.compute_swap_identifier(log))
            for log in validated_logs
        ]

        return swap_hashes

    def get_sendAsset_callback(self, w3, fromBlock, ibc_endpoint):
        filter_ack = w3.eth.filter(
            {
                "topics": [
                    "0xe6db00361b6a35af0ded81ba5696c1633e945a81008cd7da44fb8a78422a7d42"
                ],
                "fromBlock": fromBlock,
            }
        )
        entries_ack = filter_ack.get_all_entries()
        filter_timeout = w3.eth.filter(
            {
                "topics": [
                    "0xcab6c1a18a9c89efaab5ea5a8c665ffe2c5aac9ddd9301ccad01fd4fed7c7e3d"
                ],
                "fromBlock": fromBlock,
            }
        )
        entries_timeout = filter_timeout.get_all_entries()

        swap_vault = w3.eth.contract(abi=vault_abi)
        processed_acks = [
            swap_vault.events.SendAssetSuccess().processLog(entry)
            for entry in entries_ack
        ]
        processed_timeouts = [
            swap_vault.events.SendAssetFailure().processLog(entry)
            for entry in entries_timeout
        ]

        # For each log, check that the emulator was called.
        validated_acks = []
        for log in processed_acks:
            receipt = w3.eth.getTransactionReceipt(log["transactionHash"])
            if receipt["to"] == ibc_endpoint:
                validated_acks.append(log)

        validated_timeouts = []
        for log in processed_timeouts:
            receipt = w3.eth.getTransactionReceipt(log["transactionHash"])
            if receipt["to"] == ibc_endpoint:
                validated_timeouts.append(log)

        # We now have an array of Catalyst swaps.
        ack_hashes = [
            (log["transactionHash"].hex(), self.compute_swap_identifier(log), "ack")
            for log in validated_acks
        ]
        timeout_hashes = [
            (log["transactionHash"].hex(), self.compute_swap_identifier(log), "timeout")
            for log in validated_timeouts
        ]

        return ack_hashes + timeout_hashes

    def get_sendLiquidity_callback(self, w3, fromBlock, ibc_endpoint):
        filter_ack = w3.eth.filter(
            {
                "topics": [
                    "0x8a49f1dbb0b988d0421183f74b9866ce7c88256f1b88cf865bf7f3a74706fe68"
                ],
                "fromBlock": fromBlock,
            }
        )
        entries_ack = filter_ack.get_all_entries()
        filter_timeout = w3.eth.filter(
            {
                "topics": [
                    "0x97cc161fb90f5cdec9c65ba7aac2279e32df11368946590b82fd6fe8e76b39e0"
                ],
                "fromBlock": fromBlock,
            }
        )
        entries_timeout = filter_timeout.get_all_entries()

        swap_vault = w3.eth.contract(abi=vault_abi)
        processed_acks = [
            swap_vault.events.SendLiquiditySuccess().processLog(entry)
            for entry in entries_ack
        ]
        processed_timeouts = [
            swap_vault.events.SendLiquidityFailure().processLog(entry)
            for entry in entries_timeout
        ]

        # For each log, check that the emulator was called.
        validated_acks = []
        for log in processed_acks:
            receipt = w3.eth.getTransactionReceipt(log["transactionHash"])
            if receipt["to"] == ibc_endpoint:
                validated_acks.append(log)

        validated_timeouts = []
        for log in processed_timeouts:
            receipt = w3.eth.getTransactionReceipt(log["transactionHash"])
            if receipt["to"] == ibc_endpoint:
                validated_timeouts.append(log)

        # We now have an array of Catalyst swaps.
        ack_hashes = [
            (log["transactionHash"].hex(), self.compute_swap_identifier(log), "ack")
            for log in validated_acks
        ]
        timeout_hashes = [
            (log["transactionHash"].hex(), self.compute_swap_identifier(log), "timeout")
            for log in validated_timeouts
        ]

        return ack_hashes + timeout_hashes

    def get_expected_log(self, w3, tx_hash, swap_hash):
        receipt = w3.eth.get_transaction_receipt(tx_hash)
        logs = receipt.logs
        
        swap_vault = w3.eth.contract(abi=vault_abi)
        ibc_emulator = w3.eth.contract(abi=e_abi)
        swaps = []
        packets = []
        for log in logs:
            if log.topics[0].hex() == "0xe1c4c822c15df23f17ad636820a990981caf1d4e40f2f46cf3bb7ad003deaec8":
                log_index = log.logIndex
                for r_log in logs:
                    if (r_log.logIndex in [log_index - 1, log_index - 2, log_index - 3, log_index - 4]):
                        if (
                            r_log.topics[0].hex()
                            == "0x2f7e9b1a1fac10099a9988fcda077b67b3809ede82c92a7662b82f0f96861604"
                        ):  
                            swaps.append(swap_vault.events.SendAsset().processLog(log))
                            packets.append(ibc_emulator.events.Packet().processLog(r_log))
                            break
            elif log.topics[0].hex() == "0x8c9503be4db35b4e3d31565a9616d1dc3f1b3024e5e9e9d65052de46a5149f1c":
                log_index = log.logIndex
                for r_log in logs:
                    if (r_log.logIndex in [log_index - 1, log_index - 2, log_index - 3, log_index - 4]):
                        if (
                            r_log.topics[0].hex()
                            == "0x2f7e9b1a1fac10099a9988fcda077b67b3809ede82c92a7662b82f0f96861604"
                        ):
                            swaps.append(swap_vault.events.SendLiquidity().processLog(log))
                            packets.append(ibc_emulator.events.Packet().processLog(r_log))
                            break
        
        swap_hashes = [
            (packet, self.compute_swap_identifier(swap_log))
            for packet, swap_log in zip(packets, swaps)
        ]
        
        for packet, hash in swap_hashes:
            if hash == swap_hash:
                return packet
    
    def get_all_hashes(self):
        chains = self.chains.keys()

        uniqueSwapHashes = {}
        for chain in chains:
            fromBlock = 0
            logging.info(f"Checking: {chain} from {fromBlock}")
            w3 = self.chains[chain]["w3"]
            ibc_emulator = self.chains[chain]["ibcinterface"]
            sendAsssets = self.get_sendAssets(w3, fromBlock, ibc_emulator.address)
            sendLiquidity = self.get_sendLiquidity(w3, fromBlock, ibc_emulator.address)
            receiveAssets = self.get_receiveAsset(w3, fromBlock, ibc_emulator.address)
            receiveLiquidity = self.get_receiveLiquidity(
                w3, fromBlock, ibc_emulator.address
            )
            sendAssetCallbacks = self.get_sendAsset_callback(
                w3, fromBlock, ibc_emulator.address
            )
            sendLiquidityCallbacks = self.get_sendLiquidity_callback(
                w3, fromBlock, ibc_emulator.address
            )
            logging.info(
                f"Found {len(sendAsssets) + len(receiveAssets) + len(sendAssetCallbacks)} events on {chain}"
            )

            # Get all swap hashes from sendAssets
            for hashes in sendAsssets:
                if uniqueSwapHashes.get(hashes[1]) is None:
                    uniqueSwapHashes[hashes[1]] = {"sendAssetTx": [chain, hashes[0]]}
                else:
                    uniqueSwapHashes[hashes[1]]["sendAssetTx"] = [chain, hashes[0]]

            for hashes in sendLiquidity:
                if uniqueSwapHashes.get(hashes[1]) is None:
                    uniqueSwapHashes[hashes[1]] = {
                        "sendLiquidityTx": [chain, hashes[0]]
                    }
                else:
                    uniqueSwapHashes[hashes[1]]["sendLiquidityTx"] = [chain, hashes[0]]

            for hashes in receiveAssets:
                if uniqueSwapHashes.get(hashes[1]) is None:
                    uniqueSwapHashes[hashes[1]] = {"receiveAssetTx": [chain, hashes[0]]}
                else:
                    uniqueSwapHashes[hashes[1]]["receiveAssetTx"] = [chain, hashes[0]]

            # Get all swap hashes from sendLiquidity
            for hashes in receiveLiquidity:
                if uniqueSwapHashes.get(hashes[1]) is None:
                    uniqueSwapHashes[hashes[1]] = {
                        "receiveLiqudityTx": [chain, hashes[0]]
                    }
                else:
                    uniqueSwapHashes[hashes[1]]["receiveLiqudityTx"] = [
                        chain,
                        hashes[0],
                    ]

            for hashes in sendAssetCallbacks:
                if uniqueSwapHashes.get(hashes[1]) is None:
                    uniqueSwapHashes[hashes[1]] = {
                        f"{hashes[2]}SendAssetTx": [chain, hashes[0]]
                    }
                else:
                    uniqueSwapHashes[hashes[1]][f"{hashes[2]}SendAssetTx"] = [
                        chain,
                        hashes[0],
                    ]

            for hashes in sendLiquidityCallbacks:
                if uniqueSwapHashes.get(hashes[1]) is None:
                    uniqueSwapHashes[hashes[1]] = {
                        f"{hashes[2]}SendLiquidityTx": [chain, hashes[0]]
                    }
                else:
                    uniqueSwapHashes[hashes[1]][f"{hashes[2]}SendLiquidityTx"] = [
                        chain,
                        hashes[0],
                    ]

        return uniqueSwapHashes

    def backcheck(self):
        uniqueSwapHashes = self.get_all_hashes()
        
        for hash in uniqueSwapHashes.keys():
            swap_set = uniqueSwapHashes[hash]
            
            # Check if the swap contains an ack or timeout. If it does, we can skip
            if swap_set.get("ackSendAssetTx") or swap_set.get("ackSendLiquidityTx"):
                continue
            if swap_set.get("timeoutSendAssetTx") or swap_set.get("timeoutSendLiquidityTx"):
                continue
            
            # Then lets check if the swap contains a receive
            receive = False
            if swap_set.get("receiveAssetTx") or swap_set.get("receiveLiqudityTx"):
                receive = True
            
            # Finally, check if the swaphash has a sendAsset or send liquidity.
            send = False
            if swap_set.get("sendAssetTx") or swap_set.get("sendLiquidityTx"):
                send = True
            
            inital_swap = swap_set.get("sendAssetTx") if swap_set.get("sendAssetTx") is not None else swap_set.get("sendLiquidityTx")
            chain = inital_swap[0]
            w3 = self.chains[chain]["w3"]
            packet_log = self.get_expected_log(w3, inital_swap[1], hash)
            # We now have 2 different cases:
            if receive is True:
                # Then we need to submit an ack.
                tx_hash = swap_set.get("receiveAssetTx")[1] if swap_set.get("receiveAssetTx") is not None else swap_set.get("receiveLiquidityTx")[1]
                callback = self.callback(chain, packet_log, tx_hash)
            elif receive is False and send is True:
                # Then we need to execute the swap then ack, or timeout.
                tx_hash = self.execute(chain, packet_log)
                if tx_hash is not None:
                    self.callback(chain, packet_log, tx_hash)
            else:
                raise NotImplementedError
                    
    def run(self, wait=5):
        chains = self.chains.keys()
        blocknumbers = {}

        for chain in chains:
            blocknumber = self.chains[chain]["w3"].eth.blockNumber
            logging.info(
                f"Loaded {chain} at block: {blocknumber} with relayer {self.chains[chain]['acct'].address}"
            )
            blocknumbers[chain] = blocknumber

        while True:
            for chain in chains:
                w3 = self.chains[chain]["w3"]
                fromBlock = blocknumbers[chain]
                toBlock = w3.eth.blockNumber - 100  # Don't look at the last 100 blocks. Roughtly equal to 20 minutes if block times are 12 seconds.

                if fromBlock <= toBlock:
                    blocknumbers[chain] = toBlock + 1
                    logs = self.fetch_logs(chain, fromBlock, toBlock)
                    logging.info(
                        f"{chain}: {len(logs)} logs between block {fromBlock}-{toBlock}"
                    )

                    executes = []
                    for log in logs:
                        executes.append((log, self.execute(chain, log)))

                    for exec in executes:
                        log = exec[0]
                        tx_hash = exec[1]
                        if tx_hash is not None:
                            self.callback(chain, log, tx_hash)

            sleep(wait)


def main():
    parser = argparse.ArgumentParser("proxy relayer")
    parser.add_argument(
        "config_location", nargs="?", help="The path to the config location", type=str
    )
    parser.add_argument(
        "log_location", nargs="?", help="The log location. If not set, print to std-out.", type=str
    )
    args = parser.parse_args()
    config_location = "./scripts/deploy_config.json"
    if args.config_location:
        config_location = args.config_location
    if args.log_location:
        # setup log
        logging.basicConfig(level=logging.INFO, filename=args.log_location, filemode="a")
    else:
        logging.basicConfig(level=logging.INFO)

    relayer = PoARelayer(config_name=config_location)
    relayer.run()


if __name__ == "__main__":
    main()
