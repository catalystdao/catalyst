import json

from brownie import (WETH9, CatalystIBCInterface,
                     CatalystSwapPoolAmplified, CatalystSwapPoolFactory,
                     CatalystSwapPoolVolatile, IBCEmulator, Token,
                     convert, CatalystDescriber, CatalystDescriberRegistry, CatalystMathAmp, CatalystMathVol, CatalystRouter, p2, Contract)


"""
# one liner deployment
from scripts.deployCatalyst import Catalyst; cat = Catalyst(acct, "sepolia", "scripts/deploy_config.json", True, "wSEP"); WETH9.at(cat.config["tokens"]["sepolia"]["wSEP"]).deposit({'from': cat.deployer, 'value': 1*10**18}); cat.deploy_config()

# Then run
from scripts.deployCatalyst import Catalyst; cat = Catalyst(acct, "mumbai", "scripts/deploy_config.json", True, "wMUM"); WETH9.at(cat.config["tokens"]["mumbai"]["wMUM"]).deposit({'from': cat.deployer, 'value': 1*10**18}); cat.deploy_config(); cat.set_connections()

# Potentially use:
from brownie.network.gas.strategies import LinearScalingStrategy; from brownie.network import gas_price; gas_strategy = LinearScalingStrategy("1.6 gwei", "10 gwei", 2); gas_price(gas_strategy)
"""

MAX_UINT256: int = 2**256 - 1

def get_channel_id(chain_name: str):
        return convert.to_bytes(chain_name.encode(), "bytes32")
    
def convert_64_bytes_address(address):
    return convert.to_bytes(20, "bytes1")+convert.to_bytes(0)+convert.to_bytes(address.replace("0x", ""))

class Catalyst:
    def read_config(self):
        with open(self.config_name) as f:
            return json.load(f)
        
    def write_config(self):
        with open(self.config_name, 'w') as f:
            json.dump(self.config, f, indent=4)
    
    def blank_setup(self, wTKN):
        # Check if a wrapped gas token is provided.
        tkn = self.config['tokens'][self.chain][wTKN]
        if tkn == '':
            tkn = self.deployer.deploy(WETH9)
            self.config['tokens'][self.chain][wTKN] = tkn.address
        
        # Deploy mathematical libs
        volatile_mathlib = self.config['chain_config'][self.chain]["volatile_mathlib"]
        amplified_mathlib = self.config['chain_config'][self.chain]["amplified_mathlib"]
        if volatile_mathlib == '':
            volatile_mathlib = self.deployer.deploy(CatalystMathVol)
            
            self.config['chain_config'][self.chain]["volatile_mathlib"] = volatile_mathlib.address
            
        if amplified_mathlib == '':
            amplified_mathlib = self.deployer.deploy(CatalystMathAmp)
            
            self.config['chain_config'][self.chain]["amplified_mathlib"] = amplified_mathlib.address
        
        
        # Check if factory have been deployed
        factory = self.config['chain_config'][self.chain]["factory"]
        if factory == '':
            factory = self.deployer.deploy(CatalystSwapPoolFactory, 0)
            
            self.config['chain_config'][self.chain]["factory"] = factory.address
        
        
        # Deploy IBC contracts
        ibcinterface = self.config['chain_config'][self.chain]["ibcinterface"]
        crosschaininterface = self.config['chain_config'][self.chain]["crosschaininterface"]
        if ibcinterface == '':
            ibcinterface = self.deployer.deploy(IBCEmulator, convert.to_bytes(self.chain.encode()))
            relayer = self.config["relayer_address"]
            ibcinterface.transferOwnership(relayer, {'from': self.deployer})
            
            self.config['chain_config'][self.chain]["ibcinterface"] = ibcinterface.address
            
        if crosschaininterface == '':
            crosschaininterface = self.deployer.deploy(CatalystIBCInterface, ibcinterface)
            
            self.config['chain_config'][self.chain]["crosschaininterface"] = crosschaininterface.address
        
        
        # Templates
        volatile_template = self.config['chain_config'][self.chain]["volatile_template"]
        amplified_template = self.config['chain_config'][self.chain]["amplified_template"]
        if volatile_template == '':
            volatile_template = self.deployer.deploy(CatalystSwapPoolVolatile, factory, volatile_mathlib)
            
            self.config['chain_config'][self.chain]["volatile_template"] = volatile_template.address
            
        if amplified_template == '':
            amplified_template = self.deployer.deploy(CatalystSwapPoolAmplified, factory, amplified_mathlib)
            
            self.config['chain_config'][self.chain]["amplified_template"] = amplified_template.address
        
        
        # Deploy regitries
        catalyst_describer = self.config['chain_config'][self.chain]["describer"]
        describer_registry = self.config['chain_config'][self.chain]["describer_registry"]
        if catalyst_describer == '':
            catalyst_describer = self.deployer.deploy(CatalystDescriber)
            catalyst_describer.add_whitelisted_template(volatile_template, 1, {'from': self.deployer})
            catalyst_describer.add_whitelisted_template(amplified_template, 1, {'from': self.deployer})
            catalyst_describer.add_whitelisted_cii(crosschaininterface, {'from': self.deployer})
            
            self.config['chain_config'][self.chain]["describer"] = catalyst_describer.address
            
        if describer_registry == '':
            describer_registry = self.deployer.deploy(CatalystDescriberRegistry)
            describer_registry.add_describer(catalyst_describer, {'from': self.deployer})
            
            self.config['chain_config'][self.chain]["describer_registry"] = catalyst_describer.address
            
        # permit2
        permit2 = self.config['chain_config'][self.chain]["permit2"]
        if permit2 == '':
            permit2 = self.deployer.deploy(p2)
            
            self.config['chain_config'][self.chain]["permit2"] = permit2.address
        
        # Router
        router = self.config['chain_config'][self.chain]["router"]
        if router == '':
            router = self.deployer.deploy(CatalystRouter, [permit2, tkn])
            
            self.config['chain_config'][self.chain]["router"] = router.address
        
        # deploy the other tokens
        for token in self.config['tokens'][self.chain].keys():
            if token == wTKN:
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
        
        self.write_config()
        self.config = self.read_config()
    
    def __init__(
        self,
        deployer,
        chain,
        config_name="deploy_config.json",
        run_blank_setup=False,
        wTKN=""
    ):
        self.deployer = deployer
        self.config_name = config_name
        self.config = self.read_config()
        assert self.config['chain_config'].get(chain) is not None, "Chain name not found in config"
        self.chain = chain
        
        if run_blank_setup is True:
            assert self.config['tokens'][self.chain].get(wTKN) is not None, "Please provide a corrent wTKN name"
            assert type(self.config['tokens'][self.chain].get(wTKN)) is str, "Please provide a wTKN name which represents a wrapped token"
            self.blank_setup(wTKN)
    
    def deploy_config(self):
        factory = CatalystSwapPoolFactory.at(self.config['chain_config'][self.chain]["factory"])
        volatile_template = self.config['chain_config'][self.chain]["volatile_template"]
        amplified_template = self.config['chain_config'][self.chain]["amplified_template"]
        
        for pool in self.config["pools"].keys():
            if self.config["pools"][pool].get(self.chain) is None:
                continue
            if self.config["pools"][pool][self.chain]["address"] == "":
                initial_balances = []
                tokens = []
                # Approve all tokens to the factory
                for token in self.config["pools"][pool][self.chain]["tokens"].keys():
                    token_address = self.config["tokens"][self.chain][token] if type(self.config["tokens"][self.chain][token]) is str else self.config["tokens"][self.chain][token]["address"]
                    assert type(token_address) is str, f"{token}, {token_address} is not a string"
                    token_container = WETH9.at(
                        token_address
                    ) if type(self.config["tokens"][self.chain][token]) is str else Token.at(
                        token_address
                    )
                    
                    decimals = 18 if type(self.config["tokens"][self.chain][token]) is str else self.config["tokens"][self.chain][token]["decimals"]
                    
                    token_container.approve(
                        factory,
                        self.config["pools"][pool][self.chain]["tokens"][token] * 10**decimals,
                        {'from': self.deployer}
                    )
                    initial_balances.append(self.config["pools"][pool][self.chain]["tokens"][token] * 10**decimals)
                    tokens.append(token_container)
                
                deploytx = factory.deploy_swappool(
                    volatile_template if self.config["pools"][pool].get("amplification") is None else amplified_template,
                    tokens,
                    initial_balances,
                    self.config["pools"][pool][self.chain]["weights"],
                    10**18 if self.config["pools"][pool].get("amplification") is None else self.config["pools"][pool].get("amplification"),
                    0,
                    pool,
                    pool[0]+pool[3]+pool[-1],
                    self.config['chain_config'][self.chain]["crosschaininterface"],
                    {"from": self.deployer},
                )
                self.config["pools"][pool][self.chain]["address"] = deploytx.events["PoolDeployed"]["pool_address"]
        
        self.write_config()
        self.config = self.read_config()
                    
    def set_connections(self):
        volatile_template = self.config['chain_config'][self.chain]["volatile_template"]
        amplified_template = self.config['chain_config'][self.chain]["amplified_template"]
        # Check that all pools have been setup.
        for pool in self.config["pools"].keys():
            if self.chain not in self.config["pools"][pool].keys():
                continue
            for chain in self.config["pools"][pool].keys():
                if chain == "amplification":
                    continue
                assert self.config["pools"][pool][chain]["address"] != ""
            # Check that the pool hasn't been set as ready
            swapPoolContainer = CatalystSwapPoolVolatile if self.config["pools"][pool].get("amplification") is None else CatalystSwapPoolAmplified
            pool_container = swapPoolContainer.at(self.config["pools"][pool][self.chain]["address"])
            assert pool_container.ready() is False, "Pool heas already been finalised"
            
        
        for pool in self.config["pools"].keys():
            if self.chain not in self.config["pools"][pool].keys():
                continue
            swapPoolContainer = CatalystSwapPoolVolatile if self.config["pools"][pool].get("amplification") is None else CatalystSwapPoolAmplified
            pool_container = swapPoolContainer.at(self.config["pools"][pool][self.chain]["address"])
            assert pool_container.ready() is False, "Pool has already been finalised"
            
            for chain in self.config["pools"][pool].keys():
                if (chain == "amplification") or (chain == self.chain):
                    continue
                target_pool =  self.config["pools"][pool][chain]["address"]
                pool_container.setConnection(
                    get_channel_id(chain),
                    convert_64_bytes_address(target_pool),
                    True,
                    {'from': self.deployer}
                )
            pool_container.finishSetup({'from': self.deployer})
        