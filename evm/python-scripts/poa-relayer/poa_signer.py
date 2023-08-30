import argparse
import json
import logging
import os
from hashlib import sha256
from time import sleep

from eth_account import Account


logging.basicConfig(level=logging.INFO)


class MessageSigner:
    def __init__(
        self,
        private_key: str
    ):
        self.private_key = private_key

    def signMessage(self, messageHash: bytes):
        signed_message = Account.signHash(messageHash, self.private_key)
        
        return signed_message
    