import json

from brownie import (WETH9, ZERO_ADDRESS, CatalystIBCInterface,
                     CatalystSwapPoolAmplified, CatalystSwapPoolFactory,
                     CatalystSwapPoolVolatile, IBCEmulator, Token, accounts,
                     convert)

from tests.catalyst.utils.pool_utils import decode_payload

"""
Import code into terminal for interactive debugging with `brownie console`
from scripts.catalyst.deployCatalyst import Catalyst
acct = accounts[0]
ps = Catalyst(acct)
"""

MAX_UINT256: int = 2**256 - 1


class Catalyst:
    def read_config(self):
        with open(self.config_name) as f:
            return json.load(f)
        
    def write_config(self):
        with open(self.config_name, 'w') as f:
            json.dump(self.config, f)
    
    def blank_setup(self, wTKN):
        # Check if a wrapped gas token is provided.
        tkn = self.config['tokens'][self.chain][wTKN]
        if tkn == '':
            tkn = self.deployer.deploy(WETH9)
        self.config['tokens'][self.chain][wTKN] = tkn.address
        
        # deploy the other tokens
        for token in self.config['tokens'][self.chain].keys():
            if token == "wTKN":
                continue
            if self.config['tokens'][self.chain][token]["address"] == "":
                deployed_tkn = Token.deploy(
                    token,
                    token[0]+token[3]+token[-1],
                    self.config['tokens'][self.chain][token]["decimals"],
                    self.config['tokens'][self.chain][token]["supply"],
                    {"from": self.deployer}
                )
                self.config['tokens'][self.chain][token]["address"] = deployed_tkn.address
        
        # Check if factory have been deployed
        factory = self.config['chain_config'][self.chain]["factory"]
        if factory == '':
            factory = self.deployer.deploy(CatalystSwapPoolFactory, 0)
        self.config['chain_config'][self.chain]["factory"] = factory.address
        
        # Deploy IBC contracts
        ibcinterface = self.config['chain_config'][self.chain]["ibcinterface"]
        crosschaininterface = self.config['chain_config'][self.chain]["crosschaininterface"]
        if ibcinterface == '':
            ibcinterface = self.deployer.deploy(IBCEmulator)
            relayer = self.config['chain_config'][self.chain]["relayer_address"]
            ibcinterface.transferOwnership(relayer, {'from': self.deployer})
        if crosschaininterface == '':
            crosschaininterface = self.deployer.deploy(CatalystIBCInterface, ibcinterface)
        self.config['chain_config'][self.chain]["ibcinterface"] = ibcinterface.address
        self.config['chain_config'][self.chain]["crosschaininterface"] = crosschaininterface.address
        
        # Templates
        volatile_template = self.config['chain_config'][self.chain]["volatile_template"]
        amplified_template = self.config['chain_config'][self.chain]["amplified_template"]
        if volatile_template == '':
            volatile_template = self.deployer.deploy(IBCEmulator, factory)
        if amplified_template == '':
            amplified_template = self.deployer.deploy(amplified_template, factory)
        self.config['chain_config'][self.chain]["volatile_template"] = volatile_template.address
        self.config['chain_config'][self.chain]["amplified_template"] = amplified_template.address
        
        self.write_config()
    
    def __init__(
        self,
        deployer,
        chain,
        config_name="deploy_config.json",
        run_blank_setup=False,
        wTKN=""
    ):
        self.deployer = deployer
        assert self.config['chain_config'].get(chain) is not None, "Chain name not found in config"
        self.chain = chain
        self.config_name = config_name
        self.config = self.read_config()
        
        if run_blank_setup is True:
            assert self.config['tokens'][self.chain].get(wTKN) is not None, "Please provide a corrent wTKN name"
            assert type(self.config['tokens'][self.chain].get(wTKN)) is str, "Please provide a wTKN name which represents a wrapped token"
            self.blank_setup(wTKN)
    
    def deploy_config(self):
        pass
    
    def set_connections(self):
        pass

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
# pool.localSwap(tokens[0], tokens[1], 50*10**18, 0, {'from': acct})

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
tx = pool.sendAsset(
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
    pool.localSwap(tokens[0], tokens[1], 50*10**18, 0, {'from': acct})
