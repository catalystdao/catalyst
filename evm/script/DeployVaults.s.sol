// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";
import { Permit2 } from "../lib/permit2/src/Permit2.sol";
import { Strings } from "@openzeppelin/contracts/utils/Strings.sol";
import { Token } from "../test/mocks/token.sol";
import { IWETH } from "./IWETH.sol";
import { ICatalystV1Vault } from "../src/ICatalystV1Vault.sol";

// Math libs
import { CatalystMathVol } from "../src/registry/CatalystMathVol.sol";
import { CatalystMathAmp } from "../src/registry/CatalystMathAmp.sol";

// Registry
import { CatalystDescriber } from "../src/registry/CatalystDescriber.sol";
import { CatalystDescriberRegistry } from "../src/registry/CatalystDescriberRegistry.sol";

// Router
import { CatalystRouter } from "../src/router/CatalystRouter.sol";
import { RouterParameters } from "../src/router/base/RouterImmutables.sol";

// Core Catalyst
import { CatalystFactory } from "../src/CatalystFactory.sol";
import { CatalystGARPInterface } from "../src/CatalystGARPInterface.sol";
/// Catalyst Templates
import { CatalystVaultVolatile } from "../src/CatalystVaultVolatile.sol";
import { CatalystVaultAmplified } from "../src/CatalystVaultAmplified.sol";


// Generalised Incentives
import { IncentivizedMockEscrow } from "GeneralisedIncentives/src/apps/mock/IncentivizedMockEscrow.sol";

struct JsonContracts {
    address amplified_mathlib;
    address amplified_template;
    address describer;
    address describer_registry;
    address factory;
    address permit2;
    address router;
    address volatile_mathlib;
    address volatile_template;
}

contract DeployVaults is Script {
    using stdJson for string;

    string pathToVaultConfig;
    string config_vault;

    string config_token;
    string WGAS_NAME;

    CatalystFactory factory;

    bool fillDescriber = false;

    JsonContracts contracts;

    address CCI;

    string chain;
    string config_chain;
    bytes32 chainIdentifier;

    string config_interfaces;

    error NoWrappedGasTokenFound();

    function getToken(string memory name, string memory symbol, uint8 decimals, uint256 initialSupply) internal returns(address token) {
        // Check if the token is a WGAS:
        if (keccak256(abi.encodePacked(name)) == keccak256(abi.encodePacked(WGAS_NAME))) {
            return token = abi.decode(config_token.parseRaw(string.concat(".", chain, ".", name)), (address));
        }
        // Get element in config:
        token = abi.decode(config_token.parseRaw(string.concat(".", chain, ".", name, ".", "address")), (address));
        require(token != address(0), "Token havn't been deployed");
    }

    function deployVault(address[] memory assets, uint256[] memory init_balances, uint256[] memory weights, uint256 amp, uint256 vaultFee, string memory name, string memory symbol, address chainInterface) internal returns(address vaultAddress) {
        address vaultTemplate;
        if (amp == 10**18) {
            vaultTemplate = contracts.volatile_template;
        } else {
            vaultTemplate = contracts.amplified_template;
        }
        for (uint256 i = 0; i < assets.length; ++i) {
            Token(assets[i]).approve(address(factory), init_balances[i]);
        }

        vaultAddress = factory.deployVault(vaultTemplate, assets, init_balances, weights, amp, vaultFee, name, symbol, chainInterface);
    }

    function deployAllVaults() internal {
        string[] memory vaults = vm.parseJsonKeys(config_vault, "$");

        for (uint256 i = 0; i < vaults.length; ++i) {
            string memory vault = vaults[i];

            // Check if the vault has a chain part.
            if (!vm.keyExists(config_vault, string.concat(".", vault, ".", chain))) continue;

            // Check if the address has been set
            address vaultAddress = vm.parseJsonAddress(config_vault, string.concat(".", vault, ".", chain, ".address"));
            if (vaultAddress != address(0)) continue;

            // Deploy

            string[] memory assets_name = vm.parseJsonKeys(config_vault, string.concat(".", vault, ".", chain, ".tokens"));
            address[] memory assets = new address[](assets_name.length);
            uint256[] memory init_balances = new uint256[](assets_name.length);
            for (uint256 j = 0; j < assets_name.length; ++j) {
                init_balances[j] = vm.parseJsonUint(config_vault, string.concat(".", vault, ".", chain, ".tokens.", assets_name[j]));
                if (vm.keyExists(config_token, string.concat(".", chain, ".", assets_name[j], ".address"))) {
                    assets[j] = vm.parseJsonAddress(
                        config_token, string.concat(".", chain, ".", assets_name[j], ".address")
                    );
                } else {
                    assets[j] = vm.parseJsonAddress(
                        config_token, string.concat(".", chain, ".", assets_name[j])
                    );
                    IWETH(assets[j]).deposit{value: init_balances[j]}();
                }
            }
            uint256[] memory weights = vm.parseJsonUintArray(config_vault, string.concat(".", vault, ".", chain, ".weights"));
            uint256 amp = 10**18;
            if (vm.keyExists(config_vault, string.concat(".", vault, ".amplification"))) {
                amp = vm.parseJsonUint(config_vault, string.concat(".", vault, ".amplification"));
            }
            uint256 vaultFee = vm.parseJsonUint(config_vault, string.concat(".", vault, ".", chain, ".fee"));
            vaultAddress = deployVault(
                assets, init_balances, weights, amp, vaultFee, vault, vault, CCI
            );

            // Write:
            vm.writeJson(Strings.toHexString(uint160(vaultAddress), 20), pathToVaultConfig, string.concat(".", vault, ".", chain, ".address"));
        }
        
    }

    function setup() internal {

        string memory pathRoot = vm.projectRoot();
        pathToVaultConfig = string.concat(pathRoot, "/script/config/config_vaults.json");
        string memory pathToChainConfig = string.concat(pathRoot, "/script/config/config_chain.json");
        string memory pathToContractConfig = string.concat(pathRoot, "/script/config/config_contracts.json");
        string memory pathToTokenConfig = string.concat(pathRoot, "/script/config/config_tokens.json");
        string memory pathToInterfacesConfig = string.concat(pathRoot, "/script/config/config_interfaces.json");

        // Get the chain config
        chain = vm.envString("CHAIN_NAME");
        config_chain = vm.readFile(pathToChainConfig);
        chainIdentifier = abi.decode(config_chain.parseRaw(string.concat(".", chain, ".chainIdentifier")), (bytes32));

        // Get the contracts
        string memory config_contract = vm.readFile(pathToContractConfig);
        contracts = abi.decode(config_contract.parseRaw(string.concat(".", chain)), (JsonContracts));
        factory = CatalystFactory(contracts.factory);

        // Get wrapped gas
        config_token = vm.readFile(pathToTokenConfig);
        WGAS_NAME = vm.envString("WGAS");

        // Get vaults
        config_vault = vm.readFile(pathToVaultConfig);

        config_interfaces = vm.readFile(pathToInterfacesConfig);
        CCI = vm.parseJsonAddress(config_interfaces, string.concat(".", chain, ".", vm.envString("CCI_VERSION"), ".interface"));
    }


    function run() external {
        setup();

        uint256 deployerPrivateKey = vm.envUint("CATALYST_DEPLOYER");
        vm.startBroadcast(deployerPrivateKey);

        deployAllVaults();

        vm.stopBroadcast();

    }

    function setConnections() external {
        setup();

        uint256 deployerPrivateKey = vm.envUint("CATALYST_DEPLOYER");
        vm.startBroadcast(deployerPrivateKey);

        string[] memory chains = vm.parseJsonKeys(config_interfaces, "$");
        string[] memory bridge_versions = vm.parseJsonKeys(config_interfaces, string.concat(".", chain));
        for (uint256 ii = 0; ii < bridge_versions.length; ++ii) {
            string memory version = bridge_versions[ii];

        address localCCI = vm.parseJsonAddress(config_interfaces, string.concat(".", chain, ".", version, ".interface"));
        require(localCCI != address(0), "CCI not deployed");

        // Set CCI connections.
            for (uint256 i = 0; i < chains.length; ++i) {
                string memory other_chain = chains[i];
                if (keccak256(abi.encodePacked(other_chain)) == keccak256(abi.encodePacked(chain))) continue;
                if (!vm.keyExists(config_interfaces, string.concat(".", other_chain, ".", version))) continue;

                bytes32 other_chainIdentifier = abi.decode(config_chain.parseRaw(string.concat(".", other_chain, ".chainIdentifier")), (bytes32));

                address remoteCCI = vm.parseJsonAddress(config_interfaces, string.concat(".", other_chain, ".", version, ".interface"));
                address remoteGI = vm.parseJsonAddress(config_interfaces, string.concat(".", other_chain, ".", version, ".incentive"));
                if (remoteCCI == address(0)) continue;
                if (remoteCCI == address(0)) continue;

                // Check if a connection has already been set.
                bytes memory read_remote_cci = CatalystGARPInterface(localCCI).chainIdentifierToDestinationAddress(other_chainIdentifier);
                if (keccak256(read_remote_cci) != keccak256(bytes(hex""))) continue;

                // set
                CatalystGARPInterface(localCCI).connectNewChain(other_chainIdentifier, abi.encodePacked(uint8(20), bytes32(0), abi.encode(remoteCCI)), abi.encode(remoteGI));

            }
        }

        // Get all pool
        string[] memory pools = vm.parseJsonKeys(config_vault, "$");
        for (uint256 i = 0; i < pools.length; ++i) {
            string memory pool_name = pools[i];
            // Check if the vault has a (this chain) component.
            if (!vm.keyExists(config_vault, string.concat(".", pool_name, ".", chain))) continue;

            address vault_address = vm.parseJsonAddress(config_vault, string.concat(".", pool_name, ".", chain, ".address"));
            require(vault_address != address(0), "local vault not deployed");

            string[] memory pool_chains = vm.parseJsonKeys(config_vault, string.concat(".", pool_name));

            for (uint256 ii = 0; ii < pool_chains.length; ++ii) {
                // get all other chains
                string memory other_chain = pool_chains[ii];

                // skip this chain
                if (keccak256(abi.encodePacked(other_chain)) == keccak256(abi.encodePacked(chain))) continue;
                if (keccak256(abi.encodePacked(other_chain)) == keccak256(abi.encodePacked("amplification"))) continue;

                address other_vault = vm.parseJsonAddress(config_vault, string.concat(".", pool_name, ".", other_chain, ".address"));
                require(other_vault != address(0), "remote vault not deployed");

                bytes32 other_chainIdentifier = abi.decode(config_chain.parseRaw(string.concat(".", other_chain, ".chainIdentifier")), (bytes32));

                // set connections
                ICatalystV1Vault(vault_address).setConnection(
                    other_chainIdentifier,
                    abi.encodePacked(uint8(20), bytes32(0), abi.encode(other_vault)),
                    true
                );
            }
        }
        // set connections

        vm.stopBroadcast();
    }
}

