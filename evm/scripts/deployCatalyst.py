import json
from time import sleep
import os

from brownie import (WETH9, WCANTO, WCRO, WEVMOS, CatalystIBCInterface,
                     CatalystVaultAmplified, CatalystFactory,
                     CatalystVaultVolatile, IBCEmulator, Token,
                     convert, CatalystDescriber, CatalystDescriberRegistry, CatalystMathAmp, CatalystMathVol, CatalystRouter, p2, Contract)


"""
# one liner deployment
from scripts.deployCatalyst import Catalyst; cat = Catalyst(acct, "evmos", "scripts/deploy_config.json", True, "WEVMOS",
WEVMOS); WEVMOS.at(cat.config["tokens"]["evmos"]["WEVMOS"]).deposit({'from': cat.deployer, 'value': 0.8*10**18}); cat.deploy_config()

# And run
from scripts.deployCatalyst import Catalyst; cat = Catalyst(acct, "canto", "scripts/deploy_config.json", True, "WCANTO", WCANTO); WCANTO.at(cat.config["tokens"]["canto"]["WCANTO"]).deposit({'from': cat.deployer, 'value': 0.8*10**18}); cat.deploy_config();

# And run
from scripts.deployCatalyst import Catalyst; cat = Catalyst(acct, "cronos", "scripts/deploy_config.json", True, "WCRO", WCRO); WCRO.at(cat.config["tokens"]["cronos"]["WCRO"]).deposit({'from': cat.deployer, 'value': 0.8*10**18}); cat.deploy_config()

# On all chains, run:
cat.set_connections()

# Potentially use:
from brownie.network.gas.strategies import LinearScalingStrategy; from brownie.network import gas_price; gas_strategy = LinearScalingStrategy("1.6 gwei", "20 gwei", 2); gas_price(gas_strategy)
"""

MAX_UINT256: int = 2**256 - 1

LOCK = os.path.join(os.path.dirname(__file__), "lock")

def get_channel_id(from_chain: str, to_chain: str):
    a = convert.to_bytes(from_chain.encode(), "bytes32")
    b = (int.from_bytes(to_chain.encode(), "big") << 128).to_bytes(32, "big")
    return bytes(x ^ y for x, y in zip(b, a))
    
def convert_64_bytes_address(address):
    return convert.to_bytes(20, "bytes1")+convert.to_bytes(0)+convert.to_bytes(address.replace("0x", ""))


class Catalyst:
    def read_config(self):
        with open(self.config_name) as f:
            return json.load(f)
    
    def load_config(self):
        self.config = self.read_config()
        
    def prepare_write_config(self):
        while True:
            with open(LOCK, "r") as f:
                status = f.read()
                print("status", status)
            if status == "":
                with open(LOCK, "w") as f:
                    f.write(self.chain)
                sleep(0.5)
            elif status == self.chain:
                break
        
    def write_config(self):
        with open(self.config_name, 'w') as f:
            json.dump(self.config, f, indent=4)
        with open(LOCK, "w") as f:
            f = ""
    
    def blank_setup(self, wTKN, WTKN_CONTRACT):
        # Check if a wrapped gas token is provided.
        tkn = self.config['tokens'][self.chain][wTKN]
        if tkn == '':
            tkn = self.deployer.deploy(WTKN_CONTRACT)
            self.prepare_write_config()
            self.load_config()
            self.config['tokens'][self.chain][wTKN] = tkn.address
            self.write_config()
        
        # Deploy mathematical libs
        volatile_mathlib = self.config['chain_config'][self.chain]["volatile_mathlib"]
        amplified_mathlib = self.config['chain_config'][self.chain]["amplified_mathlib"]
        if volatile_mathlib == '':
            volatile_mathlib = self.deployer.deploy(CatalystMathVol)
            
            self.prepare_write_config()
            self.load_config()
            self.config['chain_config'][self.chain]["volatile_mathlib"] = volatile_mathlib.address
            self.write_config()
            
        if amplified_mathlib == '':
            amplified_mathlib = self.deployer.deploy(CatalystMathAmp)
            
            self.prepare_write_config()
            self.load_config()
            self.config['chain_config'][self.chain]["amplified_mathlib"] = amplified_mathlib.address
            self.write_config()
        
        # Check if factory have been deployed
        factory = self.config['chain_config'][self.chain]["factory"]
        if factory == '':
            factory = self.deployer.deploy(CatalystFactory, 0)
            
            self.prepare_write_config()
            self.load_config()
            self.config['chain_config'][self.chain]["factory"] = factory.address
            self.write_config()
        
        # Deploy IBC contracts
        ibcinterface = self.config['chain_config'][self.chain]["ibcinterface"]
        crosschaininterface = self.config['chain_config'][self.chain]["crosschaininterface"]
        if ibcinterface == '':
            ibcinterface = self.deployer.deploy(IBCEmulator, convert.to_bytes(self.chain.encode()))
            relayer = self.config["relayer_address"]
            ibcinterface.transferOwnership(relayer, {'from': self.deployer})
            
            self.prepare_write_config()
            self.load_config()
            self.config['chain_config'][self.chain]["ibcinterface"] = ibcinterface.address
            self.write_config()
            
        if crosschaininterface == '':
            crosschaininterface = self.deployer.deploy(CatalystIBCInterface, ibcinterface)
            
            self.load_config()
            self.config['chain_config'][self.chain]["crosschaininterface"] = crosschaininterface.address
            self.write_config()
        
        # Templates
        volatile_template = self.config['chain_config'][self.chain]["volatile_template"]
        amplified_template = self.config['chain_config'][self.chain]["amplified_template"]
        if volatile_template == '':
            volatile_template = self.deployer.deploy(CatalystVaultVolatile, factory, volatile_mathlib)
            
            self.prepare_write_config()
            self.load_config()
            self.config['chain_config'][self.chain]["volatile_template"] = volatile_template.address
            self.write_config()
            
        if amplified_template == '':
            amplified_template = self.deployer.deploy(CatalystVaultAmplified, factory, amplified_mathlib)
            
            self.prepare_write_config()
            self.load_config()
            self.config['chain_config'][self.chain]["amplified_template"] = amplified_template.address
            self.write_config()
        
        # Deploy regitries
        catalyst_describer = self.config['chain_config'][self.chain]["describer"]
        describer_registry = self.config['chain_config'][self.chain]["describer_registry"]
        if catalyst_describer == '':
            catalyst_describer = self.deployer.deploy(CatalystDescriber)
            catalyst_describer.add_vault_factory(factory, {'from': self.deployer})
            catalyst_describer.add_whitelisted_template(volatile_template, 1, {'from': self.deployer})
            catalyst_describer.add_whitelisted_template(amplified_template, 1, {'from': self.deployer})
            catalyst_describer.add_whitelisted_cii(crosschaininterface, {'from': self.deployer})
            
            self.prepare_write_config()
            self.load_config()
            self.config['chain_config'][self.chain]["describer"] = catalyst_describer.address
            self.write_config()
            
        if describer_registry == '':
            describer_registry = self.deployer.deploy(CatalystDescriberRegistry)
            describer_registry.add_describer(catalyst_describer, {'from': self.deployer})
            
            self.prepare_write_config()
            self.load_config()
            self.config['chain_config'][self.chain]["describer_registry"] = describer_registry.address
            self.write_config()
            
        # permit2
        permit2 = self.config['chain_config'][self.chain]["permit2"]
        if permit2 == '':
            permit2 = self.deployer.deploy(p2)
            
            self.prepare_write_config()
            self.load_config()
            self.config['chain_config'][self.chain]["permit2"] = permit2.address
            self.write_config()
        
        # Router
        router = self.config['chain_config'][self.chain]["router"]
        if router == '':
            router = self.deployer.deploy(CatalystRouter, [permit2, tkn])
            
            self.prepare_write_config()
            self.load_config()
            self.config['chain_config'][self.chain]["router"] = router.address
            self.write_config()
        
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
                
                self.prepare_write_config()
                self.load_config()
                self.config['tokens'][self.chain][token]["address"] = deployed_tkn.address
                self.write_config()
    
    def __init__(
        self,
        deployer,
        chain,
        config_name="deploy_config.json",
        run_blank_setup=False,
        wTKN="",
        WTKN_CONTRACT=WETH9
    ):
        self.deployer = deployer
        self.config_name = config_name
        self.config = self.read_config()
        assert self.config['chain_config'].get(chain) is not None, "Chain name not found in config"
        self.chain = chain
        self.WETH = WTKN_CONTRACT
        
        if run_blank_setup is True:
            assert self.config['tokens'][self.chain].get(wTKN) is not None, "Please provide a corrent wTKN name"
            assert type(self.config['tokens'][self.chain].get(wTKN)) is str, "Please provide a wTKN name which represents a wrapped token"
            self.blank_setup(wTKN, WTKN_CONTRACT)
    
    def deploy_config(self):
        factory = CatalystFactory.at(self.config['chain_config'][self.chain]["factory"])
        volatile_template = self.config['chain_config'][self.chain]["volatile_template"]
        amplified_template = self.config['chain_config'][self.chain]["amplified_template"]
        
        for vault in self.config["vaults"].keys():
            if self.config["vaults"][vault].get(self.chain) is None:
                continue
            if self.config["vaults"][vault][self.chain]["address"] == "":
                initial_balances = []
                tokens = []
                # Approve all tokens to the factory
                for token in self.config["vaults"][vault][self.chain]["tokens"].keys():
                    token_address = self.config["tokens"][self.chain][token] if type(self.config["tokens"][self.chain][token]) is str else self.config["tokens"][self.chain][token]["address"]
                    assert type(token_address) is str, f"{token}, {token_address} is not a string"
                    token_container = self.WETH.at(
                        token_address
                    ) if type(self.config["tokens"][self.chain][token]) is str else Token.at(
                        token_address
                    )
                    
                    decimals = 18 if type(self.config["tokens"][self.chain][token]) is str else self.config["tokens"][self.chain][token]["decimals"]
                    
                    token_container.approve(
                        factory,
                        self.config["vaults"][vault][self.chain]["tokens"][token] * 10**decimals,
                        {'from': self.deployer}
                    )
                    initial_balances.append(self.config["vaults"][vault][self.chain]["tokens"][token] * 10**decimals)
                    tokens.append(token_container)
                
                deploytx = factory.deployVault(
                    volatile_template if self.config["vaults"][vault].get("amplification") is None else amplified_template,
                    tokens,
                    initial_balances,
                    self.config["vaults"][vault][self.chain]["weights"],
                    10**18 if self.config["vaults"][vault].get("amplification") is None else self.config["vaults"][vault].get("amplification")*10**18,
                    self.config["vaults"][vault][self.chain]["fee"]*10**18,
                    vault,
                    vault[0]+vault[3]+vault[-1],
                    self.config['chain_config'][self.chain]["crosschaininterface"],
                    {"from": self.deployer},
                )
                
                self.prepare_write_config()
                self.load_config()
                self.config["vaults"][vault][self.chain]["address"] = deploytx.events["VaultDeployed"]["vaultAddress"]
                self.write_config()
                    
    def set_connections(self):
        self.load_config()
        volatile_template = self.config['chain_config'][self.chain]["volatile_template"]
        amplified_template = self.config['chain_config'][self.chain]["amplified_template"]
        # Check that all vaults have been setup.
        for vault in self.config["vaults"].keys():
            if self.chain not in self.config["vaults"][vault].keys():
                continue
            for chain in self.config["vaults"][vault].keys():
                if chain == "amplification":
                    continue
                assert self.config["vaults"][vault][chain]["address"] != ""
            # Check that the vault hasn't been set as ready
            vaultContainer = CatalystVaultVolatile if self.config["vaults"][vault].get("amplification") is None else CatalystVaultAmplified
            vault_container = vaultContainer.at(self.config["vaults"][vault][self.chain]["address"])
            assert vault_container.ready() is False, "Vault heas already been finalised"
        
        for vault in self.config["vaults"].keys():
            if self.chain not in self.config["vaults"][vault].keys():
                continue
            vaultContainer = CatalystVaultVolatile if self.config["vaults"][vault].get("amplification") is None else CatalystVaultAmplified
            vault_container = vaultContainer.at(self.config["vaults"][vault][self.chain]["address"])
            assert vault_container.ready() is False, "Vault has already been finalised"
            
            for chain in self.config["vaults"][vault].keys():
                if (chain == "amplification") or (chain == self.chain):
                    continue
                target_vault =  self.config["vaults"][vault][chain]["address"]
                vault_container.setConnection(
                    get_channel_id(chain, self.chain),
                    convert_64_bytes_address(target_vault),
                    True,
                    {'from': self.deployer}
                )
                vault_container.setConnection(
                    get_channel_id(self.chain, chain),
                    convert_64_bytes_address(target_vault),
                    True,
                    {'from': self.deployer}
                )
            vault_container.finishSetup({'from': self.deployer})
