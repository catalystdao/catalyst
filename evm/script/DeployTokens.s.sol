// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";
import { Token } from "../test/mocks/token.sol";
import { Strings } from "@openzeppelin/contracts/utils/Strings.sol";

contract DeployCatalyst is Script {
    using stdJson for string;

    string pathToTokenConfig;
    string config_token;

    string WGAS_NAME;

    string chain;
    bytes32 chainIdentifier;

    error NoWrappedGasTokenFound();

    function getOrDeployToken(string memory name, string memory symbol, uint8 decimals, uint256 initialSupply) internal returns(address token) {
        // Check if the token is a WGAS:
        if (keccak256(abi.encodePacked(name)) == keccak256(abi.encodePacked(WGAS_NAME))) {
            return token = abi.decode(config_token.parseRaw(string.concat(".", chain, ".", name)), (address));
        }
        // Get element in config:
        token = abi.decode(config_token.parseRaw(string.concat(".", chain, ".", name, ".", "address")), (address));
        if (token == address(0)) {
            token = address(new Token(name, symbol, decimals, initialSupply));

            vm.writeJson(Strings.toHexString(uint160(token), 20), pathToTokenConfig, string.concat(".", chain, ".", name, ".address"));
        }
    }

    function deployAllTokens() internal {
        string[] memory keys = vm.parseJsonKeys(config_token, string.concat(".", chain));

        for (uint256 i = 0; i < keys.length; ++i) {
            string memory key = keys[i];
            if (keccak256(abi.encodePacked(key)) == keccak256(abi.encodePacked(WGAS_NAME))) continue;
            uint8 decimals = abi.decode(config_token.parseRaw(string.concat(".", chain, ".", key, ".", "decimals")), (uint8));

            getOrDeployToken(key, key, decimals, 1e12);
        }
    }

    function run() external {

        string memory pathRoot = vm.projectRoot();
        pathToTokenConfig = string.concat(pathRoot, "/script/config/config_tokens.json");

        // Get the chain config
        chain = vm.envString("CHAIN_NAME");

        // Get the contracts
        config_token = vm.readFile(pathToTokenConfig);

        // Get wrapped gas
        WGAS_NAME =  vm.envString("WGAS");

        uint256 deployerPrivateKey = vm.envUint("TOKENS_KEY");
        vm.startBroadcast(deployerPrivateKey);

        deployAllTokens();

        vm.stopBroadcast();

    }
}

