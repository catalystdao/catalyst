import argparse
import json
import logging
import os
from hashlib import sha256
from time import sleep

import web3
# from brownie import convert
from web3 import Web3
from web3.middleware import geth_poa


logging.basicConfig(level=logging.INFO)


class MessageSigner:
    def __init__(
        self,
        private_key: str
    ):
        self.private_key = private_key

    def signMessage(self, message: str):
        signed_message = Web3.eth.account.sign_message(message, private_key=self.private_key)
        
        return signed_message
    