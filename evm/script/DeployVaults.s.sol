// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";
import { Permit2 } from "../lib/permit2/src/Permit2.sol";
import { Strings } from "@openzeppelin/contracts/utils/Strings.sol";
import { Token } from "../test/mocks/token.sol";

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

    event Debug(uint256 a);
    event Debug(uint256[] a);
    event Debug(address[] a);
    event Debug(string[] a);

    string pathToVaultConfig;
    string config_vault;

    string config_token;
    string WGAS_NAME;

    CatalystFactory factory;

    bool fillDescriber = false;

    JsonContracts contracts;

    address CCI;

    string chain;
    bytes32 chainIdentifier;

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
            for (uint256 i = 0; i < assets_name.length; ++i) {
                if (vm.keyExists(config_token, string.concat(".", chain, ".", assets_name[i], ".address"))) {
                    assets[i] = vm.parseJsonAddress(
                        config_token, string.concat(".", chain, ".", assets_name[i], ".address")
                    );
                } else {
                    assets[i] = vm.parseJsonAddress(
                        config_token, string.concat(".", chain, ".", assets_name[i])
                    );
                }
                init_balances[i] = vm.parseJsonUint(config_vault, string.concat(".", vault, ".", chain, ".tokens.", assets_name[i]));
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
        string memory config_chain = vm.readFile(pathToChainConfig);
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

        string memory config_interfaces = vm.readFile(pathToInterfacesConfig);
        CCI = vm.parseJsonAddress(config_interfaces, string.concat(".", chain, ".", vm.envString("CCI_VERSION"), ".interface"));
    }


    function run() external {
        setup();

        uint256 deployerPrivateKey = vm.envUint("VAULT_KEY");
        vm.startBroadcast(deployerPrivateKey);

        deployAllVaults();

        vm.stopBroadcast();

    }

    function setConnections() external {
        setup();

        uint256 deployerPrivateKey = vm.envUint("VAULT_KEY");
        vm.startBroadcast(deployerPrivateKey);

        deployAllVaults();

        vm.stopBroadcast();
    }
}

