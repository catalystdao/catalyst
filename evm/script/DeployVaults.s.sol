// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";
import { Strings } from "openzeppelin-contracts/contracts/utils/Strings.sol";
import { Token } from "../test/mocks/token.sol";
import { IWETH } from "./IWETH.sol";
import { ICatalystV1Vault } from "../src/ICatalystV1Vault.sol";

import { CatalystFactory } from "../src/CatalystFactory.sol";

import { BaseMultiChainDeployer} from "./BaseMultiChainDeployer.s.sol";

struct JsonContracts {
    address amplified_mathlib;
    address amplified_template;
    address describer;
    address describer_registry;
    address factory;
    address volatile_mathlib;
    address volatile_template;
}

contract DeployVaults is BaseMultiChainDeployer {
    using stdJson for string;

    string pathToVaultConfig;
    string config_vault;

    string config_token;

    string config_chain;

    JsonContracts contracts;

    function deployVault(address[] memory assets, uint256[] memory init_balances, uint256[] memory weights, uint256 amp, uint256 vaultFee, string memory name, string memory symbol, address chainInterface) internal returns(address vaultAddress) {
        address vaultTemplate;
        if (amp == 10**18) {
            vaultTemplate = contracts.volatile_template;
        } else {
            vaultTemplate = contracts.amplified_template;
        }
        for (uint256 i = 0; i < assets.length; ++i) {
            Token(assets[i]).approve(contracts.factory, init_balances[i]);
        }

        vaultAddress = CatalystFactory(contracts.factory).deployVault(vaultTemplate, assets, init_balances, weights, amp, vaultFee, name, symbol, chainInterface);
    }

    modifier load_config() {
        string memory pathRoot = vm.projectRoot();
        string memory pathToContractConfig = string.concat(pathRoot, "/script/config/config_contracts.json");
        string memory config_contract = vm.readFile(pathToContractConfig);
        contracts = abi.decode(config_contract.parseRaw(string.concat(".contracts")), (JsonContracts));

        string memory pathToTokenConfig = string.concat(pathRoot, "/script/config/config_tokens.json");
        config_token = vm.readFile(pathToTokenConfig);

        pathToVaultConfig = string.concat(pathRoot, "/script/config/config_vaults.json");
        config_vault = vm.readFile(pathToVaultConfig);

        string memory pathToChainConfig = string.concat(pathRoot, "/script/config/config_chain.json");
        config_chain = vm.readFile(pathToChainConfig);

        _;
    }

    function _deploy() internal {
        string memory chain_name = rpc[chain];

        string[] memory pools = vm.parseJsonKeys(config_vault, "$");
        for (uint256 p_i = 0; p_i < pools.length; ++p_i) {
            string memory pool = pools[p_i];

            // Check if the vault exists on this chain.
            if (!vm.keyExists(config_vault, string.concat(".", pool, ".", chain_name))) continue;

            // Check if the address has been set
            address vaultAddress = vm.parseJsonAddress(config_vault, string.concat(".", pool, ".", chain_name, ".address"));
            if (vaultAddress != address(0)) continue;

            string[] memory assets_names = vm.parseJsonKeys(config_vault, string.concat(".", pool, ".", chain_name, ".tokens"));
            address[] memory assets = new address[](assets_names.length);
            uint256[] memory init_balances = new uint256[](assets_names.length);
            for (uint256 j = 0; j < assets_names.length; ++j) {
                string memory assets_name = assets_names[j];

                init_balances[j] = vm.parseJsonUint(config_vault, string.concat(".", pool, ".", chain_name, ".tokens.", assets_name));

                if (keccak256(abi.encodePacked(assets_name)) == keccak256(abi.encodePacked("WGAS"))) {
                    assets_name = wrapped_gas[chain];
                    assets[j] = vm.parseJsonAddress(
                        config_token, string.concat(".", chain_name, ".", assets_name)
                    );
                    // check if we have enough.
                    if (IWETH(assets[j]).balanceOf(address(vm.envAddress("CATALYST_ADDRESS"))) < init_balances[j]) {
                        IWETH(assets[j]).deposit{value: init_balances[j]}();
                    }
                } else {
                    assets[j] = vm.parseJsonAddress(
                        config_token, string.concat(".", chain_name, ".", assets_name, ".address")
                    );
                }
            }
            address CCI = vm.parseJsonAddress(config_vault, string.concat(".", pool, ".", chain_name, ".cci"));
            uint256[] memory weights = vm.parseJsonUintArray(config_vault, string.concat(".", pool, ".", chain_name, ".weights"));
            uint256 amp = 10**18;
            if (vm.keyExists(config_vault, string.concat(".", pool, ".amplification"))) {
                amp = vm.parseJsonUint(config_vault, string.concat(".", pool, ".amplification"));
            }
            uint256 vaultFee = vm.parseJsonUint(config_vault, string.concat(".", pool, ".", chain_name, ".fee"));
            vaultAddress = deployVault(
                assets, init_balances, weights, amp, vaultFee, pool, pool, CCI
            );

            // Write:
            vm.writeJson(Strings.toHexString(uint160(vaultAddress), 20), pathToVaultConfig, string.concat(".", pool, ".", chain_name, ".address"));
        }
    }

    function _setConnection() internal {
        string memory chain_name = rpc[chain];

        string[] memory pools = vm.parseJsonKeys(config_vault, "$");
        for (uint256 p_i = 0; p_i < pools.length; ++p_i) {
            string memory pool = pools[p_i];
            console.log(pool);

            // Check if the vault exists on this chain.
            if (!vm.keyExists(config_vault, string.concat(".", pool, ".", chain_name))) continue;

            // Check if the address has been set
            address vaultAddress = vm.parseJsonAddress(config_vault, string.concat(".", pool, ".", chain_name, ".address"));
            console.logAddress(vaultAddress);
            if (vaultAddress == address(0)) continue;

            // check if vault has already been setup.
            console.logAddress(ICatalystV1Vault(vaultAddress)._setupMaster());
            if (ICatalystV1Vault(vaultAddress)._setupMaster() == address(0)) continue;

            string[] memory vault_chains = vm.parseJsonKeys(config_vault, string.concat(".", pool));
            for (uint256 vc_i = 0; vc_i < vault_chains.length; ++ vc_i) {
                string memory vault_chain = vault_chains[vc_i];
                console.log(vault_chain);
                if (keccak256(abi.encodePacked(chain_name)) == keccak256(abi.encodePacked(vault_chain))) continue;

                bytes32 chainIdentifier = bytes32(vm.parseJsonUint(config_chain, string.concat(".", vault_chain, ".chainIdentifier")));

                address connectedVaultAddress = vm.parseJsonAddress(config_vault, string.concat(".", pool, ".", vault_chain, ".address"));

                ICatalystV1Vault(vaultAddress).setConnection(
                    chainIdentifier, abi.encodePacked(
                        uint8(20), bytes32(0), abi.encode(connectedVaultAddress)
                    ), true
                );
            }

            ICatalystV1Vault(vaultAddress).finishSetup();
        }
    }

    function deploy() load_config iter_chains(chain_list) broadcast public {
        _deploy();
    }

    function deploy_legacy() load_config iter_chains(chain_list_legacy) broadcast public {
        _deploy();
    }

    function setConnection() load_config iter_chains(chain_list) broadcast public {
        _setConnection();
    }

    function setConnection_legacy() load_config iter_chains(chain_list_legacy) broadcast public {
        _setConnection();
    }
}

