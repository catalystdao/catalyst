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
        ibcinterface="",
        poolname="CatalystVEvent",
        poolsymbol="CVE"
    ):
        self.deployer = deployer
        self.amp = amp
        self.ibcinterface = ibcinterface
        self.poolname = poolname
        self.poolsymbol = poolsymbol

        self.crosschaininterface = self._crosschaininterface()
        self.swapFactory = self._swapFactory()
        (self.swapTemplate, self.ampSwapTemplate) = self._swapTemplates()

        if default:
            self.defaultSetup()

    def create_token(self, name="TokenName", symbol="TKN", decimal=18):
        return Token.deploy(
            name, symbol, decimal, 10000, {"from": self.deployer}
        )

    def defaultSetup(self):
        tokens = []
        tokens.append(self.create_token("TokenOne", "ONE"))
        tokens.append(self.create_token("TokenTwo", "TWO"))
        self.deploy_swappool(
            tokens, amp=self.amp, name=self.poolname, symbol=self.poolsymbol
        )

    def _swapTemplates(self):
        swapTemplate = CatalystSwapPoolVolatile.deploy(self.swapFactory, {"from": self.deployer})
        ampSwapTemplate = CatalystSwapPoolAmplified.deploy(self.swapFactory, {"from": self.deployer})
        
        return (swapTemplate, ampSwapTemplate)

    def _swapFactory(self):
        return CatalystSwapPoolFactory.deploy(0, {"from": self.deployer})

    def _crosschaininterface(self):
        return CatalystIBCInterface.deploy(self.ibcinterface, {"from": self.deployer})

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
acct = accounts.add()
from scripts.deployCatalyst import *
ie = IBCEmulator.deploy({'from': acct})
cxt = Catalyst(acct, ibcinterface=ie)

chid = convert.to_bytes(0, type_str="bytes32")

cxt.swappool.setConnection(chid, swappool, True, {'from': acct})

cxt.swappool.finishSetup({'from': acct})
"""


def main():
    acct = accounts[0]
    ie = IBCEmulator.deploy({'from': acct})
    ps = Catalyst(acct, ibcinterface=ie)
    pool = ps.swappool
    tokens = ps.tokens
    tokens[0].approve(pool, 2**256-1, {'from': acct})
    pool.localSwap(tokens[0], tokens[1], 50*10**18, 0, {'from': acct})
