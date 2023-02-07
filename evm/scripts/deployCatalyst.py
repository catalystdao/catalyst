from mimetypes import init
from brownie import (
    CatalystSwapPoolVolatile,
    CatalystSwapPoolAmplified,
    CatalystSwapPoolFactory,
    Token,
    CatalystIBCInterface,
    IBCEmulator
)
from brownie import ZERO_ADDRESS, accounts, convert
from tests.catalyst.utils.pool_utils import decode_payload

"""
Import code into terminal for interactive debugging with `brownie console`
from scripts.catalyst.deployCatalyst import Catalyst
acct = accounts[0]
ps = Catalyst(acct)
"""

MAX_UINT256: int = 2**256 - 1




class Catalyst:
    def __init__(
        self,
        deployer,
        default=True,
        amp=10**18,
        ibcinterface=ZERO_ADDRESS,
        poolname="poolname",
        poolsymbol="ps"
    ):
        self.deployer = deployer
        self.amp = amp
        self.ibcinterface = ibcinterface
        self.poolname = poolname
        self.poolsymbol = poolsymbol

        self._swapFactory()
        self._swapTemplates()
        self._crosschaininterface()

        if default:
            self.defaultSetup()

    def create_token(self, name="TokenName", symbol="TKN", decimal=18):
        return Token.deploy(
            name, symbol, decimal, 10000, {"from": self.deployer}
        )

    def defaultSetup(self):
        tokens = []
        tokens.append(self.create_token("one", "I"))
        tokens.append(self.create_token("two", "II"))
        tokens.append(self.create_token("three", "III"))
        self.deploy_swappool(
            tokens, amp=self.amp, name=self.poolname, symbol=self.poolsymbol
        )

    def _swapTemplates(self):
        self.swapTemplate = CatalystSwapPoolVolatile.deploy(self.swapFactory, {"from": self.deployer})
        self.ampSwapTemplate = CatalystSwapPoolAmplified.deploy(self.swapFactory, {"from": self.deployer})

    def _swapFactory(self):
        self.swapFactory = CatalystSwapPoolFactory.deploy(0, {"from": self.deployer})

    def _crosschaininterface(self):
        self.crosschaininterface = CatalystIBCInterface.deploy(self.ibcinterface, {"from": self.deployer})

    def deploy_swappool(
        self, tokens, init_balances=None, weights=None, amp=10**18, name="Name", symbol="SYM"
    ):
        
        if init_balances is None:
            init_balances = []
            for token in tokens:
                init_balances.append(int(token.balanceOf(self.deployer) / 10))
                token.approve(
                    self.swapFactory,
                    int(token.balanceOf(self.deployer) / 10),
                    {"from": self.deployer},
                )
        if weights is None:
            weights = []
            for token in tokens:
                weights.append(1)

        self.deploytx = self.swapFactory.deploy_swappool(
            self.swapTemplate,
            tokens,
            init_balances,
            weights,
            amp,
            0,
            name,
            symbol,
            self.crosschaininterface,
            {"from": self.deployer},
        )
        self.tokens = tokens
        self.swappool = CatalystSwapPoolVolatile.at(
            self.deploytx.events["PoolDeployed"]["pool_address"]
        )
        return self.swappool


"""
from scripts.deployCatalyst import Catalyst, decode_payload
from brownie import convert
acct = accounts[0]

ie = IBCEmulator.deploy({'from': acct})
ps = Catalyst(acct, ibcinterface=ie)
pool = ps.swappool
tokens = ps.tokens
tokens[0].approve(pool, 2**256-1, {'from': acct})
# pool.localswap(tokens[0], tokens[1], 50*10**18, 0, {'from': acct})

chid = convert.to_bytes(1, type_str="bytes32")

# Registor IBC ports.
ps.crosschaininterface.registerPort()
ps.crosschaininterface.registerPort()

# Create the connection between the pool and itself:
pool.setConnection(
    chid,
    convert.to_bytes(pool.address.replace("0x", "")),
    True,
    {"from": acct}
)

swap_amount = tokens[0].balanceOf(pool)//10
tx = pool.sendSwap(
    chid,
    convert.to_bytes(pool.address.replace("0x", "")),
    convert.to_bytes(acct.address.replace("0x", "")),
    tokens[0],
    1,
    swap_amount,
    0,
    acct,
    {"from": acct},
)

# The data package:
tx.events["IncomingPacket"]["packet"][3]
decode_payload(tx.events["IncomingPacket"]["packet"][3])

# Execute the IBC package:
txe = ie.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": acct})

txe.info()
"""


def main():
    acct = accounts[0]
    ie = IBCEmulator.deploy({'from': acct})
    ps = Catalyst(acct, ibcinterface=ie)
    pool = ps.swappool
    tokens = ps.tokens
    tokens[0].approve(pool, 2**256-1, {'from': acct})
    pool.localswap(tokens[0], tokens[1], 50*10**18, 0, {'from': acct})
