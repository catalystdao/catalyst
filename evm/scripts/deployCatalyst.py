from mimetypes import init
from brownie import (
    CatalystSwapPool,
    CatalystSwapPoolAmplified,
    CatalystSwapPoolFactory,
    Token,
    CatalystIBCInterface,
    IBCEmulator
)
from brownie import ZERO_ADDRESS, accounts

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

        self._swapTemplates()
        self._swapFactory()
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
        self.swapTemplate = CatalystSwapPool.deploy({"from": self.deployer})
        self.ampSwapTemplate = CatalystSwapPoolAmplified.deploy({"from": self.deployer})

    def _swapFactory(self):
        self.swapFactory = CatalystSwapPoolFactory.deploy(
            self.swapTemplate, self.ampSwapTemplate, 0, {"from": self.deployer}
        )

    def _crosschaininterface(self):
        self.crosschaininterface = CatalystIBCInterface.deploy(
            self.swapFactory, self.ibcinterface, {"from": self.deployer}
        )

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
            0,
            tokens,
            init_balances,
            weights,
            amp,
            name,
            symbol,
            self.crosschaininterface,
            {"from": self.deployer},
        )
        self.tokens = tokens
        self.swappool = CatalystSwapPool.at(
            self.deploytx.events["PoolDeployed"]["pool_address"]
        )
        return self.swappool


"""
from scripts.deployCatalyst import Catalyst
acct = accounts[0]

ie = IBCEmulator.deploy({'from': acct})
ps = Catalyst(acct, ibcinterface=ie)
swappool = ps.swappool
tokens = ps.tokens
tokens[0].approve(swappool, 2**256-1, {'from': acct})
swappool.localswap(tokens[0], tokens[1], 50*10**18, 0, {'from': acct})
"""


def main():
    acct = accounts[0]
    ie = IBCEmulator.deploy({'from': acct})
    ps = Catalyst(acct, ibcinterface=ie)
    swappool = ps.swappool
    tokens = ps.tokens
    tokens[0].approve(swappool, 2**256-1, {'from': acct})
    swappool.localswap(tokens[0], tokens[1], 50*10**18, 0, {'from': acct})
