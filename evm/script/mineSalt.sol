// SPDX-License-Identifier: UNLICENSED
// Attribute to https://github.com/karmacoma-eth/foundry-playground/blob/main/script/MineSaltScript.sol

pragma solidity ^0.8.13;

import "forge-std/Script.sol";

import {StdAssertions} from "forge-std/StdAssertions.sol";

import {LibString} from "solady/src/utils/LibString.sol";

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
import { CatalystChainInterface } from "../src/CatalystChainInterface.sol";
/// Catalyst Templates
import { CatalystVaultVolatile } from "../src/CatalystVaultVolatile.sol";
import { CatalystVaultAmplified } from "../src/CatalystVaultAmplified.sol";

contract MineSalt is Script, StdAssertions {
    using LibString for bytes;

    function setUp() public {}

    function mineSalt(bytes32 initCodeHash, string memory startsWith)
        public
        returns (bytes32 salt, address expectedAddress)
    {
        string[] memory args = new string[](6);
        args[0] = "cast";
        args[1] = "create2";
        args[2] = "--starts-with";
        args[3] = startsWith;
        args[4] = "--init-code-hash";
        args[5] = LibString.toHexStringNoPrefix(uint256(initCodeHash), 32);
        string memory result = string(vm.ffi(args));

        uint256 addressIndex = LibString.indexOf(result, "Address: ");
        string memory addressStr = LibString.slice(result, addressIndex + 9, addressIndex + 9 + 42);
        expectedAddress = vm.parseAddress(addressStr);

        uint256 saltIndex = LibString.indexOf(result, "Salt: ");
        string memory saltStr = LibString.slice(result, saltIndex + 6, bytes(result).length);
        salt = bytes32(vm.parseUint(saltStr));
    }

    function factory() public returns(address actualAddress) {
        bytes32 initCodeHash = keccak256(abi.encodePacked(type(CatalystFactory).creationCode, abi.encode(vm.envAddress("CATALYST_ADDRESS"))));
        (bytes32 salt, address expectedAddress) = mineSalt(initCodeHash, "000000");

        // DEPLOY
        vm.startBroadcast();
        actualAddress = address(new CatalystFactory{salt: bytes32(salt)}(vm.envAddress("CATALYST_ADDRESS")));
        vm.stopBroadcast();

        assertEq(actualAddress, expectedAddress);
        console2.log("Deployed factory at", actualAddress);
        console2.log("salt", uint256(salt));
    }

    function mathvol() public returns(address actualAddress) {
        bytes32 initCodeHash = keccak256(abi.encodePacked(type(CatalystMathVol).creationCode));
        (bytes32 salt, address expectedAddress) = mineSalt(initCodeHash, "000");

        // DEPLOY
        vm.startBroadcast();
        actualAddress = address(new CatalystMathVol{salt: bytes32(salt)}());
        vm.stopBroadcast();

        assertEq(actualAddress, expectedAddress);
        console2.log("Deployed vol lib at", actualAddress);
        console2.log("salt", uint256(salt));
    }

    function mathamp() public returns(address actualAddress) {
        bytes32 initCodeHash = keccak256(abi.encodePacked(type(CatalystMathAmp).creationCode));
        (bytes32 salt, address expectedAddress) = mineSalt(initCodeHash, "000");

        // DEPLOY
        vm.startBroadcast();
        actualAddress = address(new CatalystMathAmp{salt: bytes32(salt)}());
        vm.stopBroadcast();

        assertEq(actualAddress, expectedAddress);
        console2.log("Deployed amp lib at", actualAddress);
        console2.log("salt", uint256(salt));
    }

    function templatevolatile(address factory_address, address math_lib) public returns(address actualAddress) {
        bytes32 initCodeHash = keccak256(abi.encodePacked(type(CatalystVaultVolatile).creationCode, abi.encode(factory_address, math_lib)));
        (bytes32 salt, address expectedAddress) = mineSalt(initCodeHash, "0000000");

        // DEPLOY
        vm.startBroadcast();
        actualAddress = address(new CatalystVaultVolatile{salt: bytes32(salt)}(factory_address, math_lib));
        vm.stopBroadcast();

        assertEq(actualAddress, expectedAddress);
        console2.log("Deployed vol template at", actualAddress);
        console2.log("salt", uint256(salt));
    }

    function templateamplified(address factory_address, address math_lib) public returns(address actualAddress) {
        bytes32 initCodeHash = keccak256(abi.encodePacked(type(CatalystVaultAmplified).creationCode, abi.encode(factory_address, math_lib)));
        (bytes32 salt, address expectedAddress) = mineSalt(initCodeHash, "0000000");

        // DEPLOY
        vm.startBroadcast();
        actualAddress = address(new CatalystVaultAmplified{salt: bytes32(salt)}(factory_address, math_lib));
        vm.stopBroadcast();

        assertEq(actualAddress, expectedAddress);
        console2.log("Deployed amp template at", actualAddress);
        console2.log("salt", uint256(salt));
    }

    function cci(address incentive) public returns(address actualAddress) {
        bytes32 initCodeHash = keccak256(abi.encodePacked(type(CatalystChainInterface).creationCode, abi.encode(incentive, vm.envAddress("CATALYST_ADDRESS"))));
        (bytes32 salt, address expectedAddress) = mineSalt(initCodeHash, "000000");

        // DEPLOY
        vm.startBroadcast();
        actualAddress = address(new CatalystChainInterface{salt: bytes32(salt)}(incentive, vm.envAddress("CATALYST_ADDRESS")));
        vm.stopBroadcast();

        assertEq(actualAddress, expectedAddress);
        console2.log("Deployed cci at", actualAddress);
        console2.log("salt", uint256(salt));
    }

    function run() public {
        address factory_address = address(0x0000003B2d6feA85e801d0e3cFd82562D78B7421); // factory(); // salt: 17913527253544052377148342935207663648266424961550501187556387275812090712216
        address math_lib_vol = address(0x0002EDE97B6779C1898a715a7bbABf5A3FF01B11); // mathvol(); // salt: 85598404464591085278359771032523095787540092714401110853070981275698672848766
        address math_lib_amp = address(0x000be580b95551826fFeEBD7f87A381168352EFB); // mathamp(); // salt: 112267599032000726981556348618800808014156055146433051612035005254394394945477
        address incentive = address(0x000000641AC10b4e000fe361F2149E2a531061c5);
        address(0x00000077cE61884b85c528206BbB6Da912C20069); // cci(incentive); // salt: 23662216287711495946301799928329798522602365757173561077693070255109652532690
        templatevolatile(factory_address, math_lib_vol);
        templateamplified(factory_address, math_lib_amp);
    }
}