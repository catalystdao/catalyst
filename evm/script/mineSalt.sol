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
import { CatalystGARPInterface } from "../src/CatalystGARPInterface.sol";
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
        bytes32 initCodeHash = keccak256(abi.encodePacked(type(CatalystGARPInterface).creationCode, abi.encode(incentive, vm.envAddress("CATALYST_ADDRESS"))));
        (bytes32 salt, address expectedAddress) = mineSalt(initCodeHash, "000000");

        // DEPLOY
        vm.startBroadcast();
        actualAddress = address(new CatalystGARPInterface{salt: bytes32(salt)}(incentive, vm.envAddress("CATALYST_ADDRESS")));
        vm.stopBroadcast();

        assertEq(actualAddress, expectedAddress);
        console2.log("Deployed incentive at", actualAddress);
        console2.log("salt", uint256(salt));
    }

    function run() public {
        address factory_address = address(0x000000833bCE31E92256B0495F689C960Ab43ecF); // factory(); salt: 10939276613522903274843397318942176140916222402700924666769304012145111727455
        address math_lib_vol = address(0x000B5ea158396635B4baA3d4434cAC1b7f21a27A); // mathvol(); salt: 74033924058872986406176265824008880344335346982926356649690178493693274618638
        address math_lib_amp = address(0x00079a894bcB0E6DaBc277F42C6856d63Ab63dDd); // mathamp(); salt: 18470954962426965460301953922945832700396418726579815624870618255990400962467
        address incentive = address(0x000000641AC10b4e000fe361F2149E2a531061c5);
        address cci = address(0x000000fbDa7dAf29ea5C8A3CEd2d05D65733917b); // cci(incentive); salt: 96570294397286943990946847491866188430491972815926572221094897960520587353076
        templatevolatile(factory_address, math_lib_vol);
        templateamplified(factory_address, math_lib_amp);
    }
}