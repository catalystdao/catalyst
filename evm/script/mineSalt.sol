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

    function factory() public {
        uint256 baseGovFee = 0;

        bytes32 initCodeHash = keccak256(abi.encodePacked(type(CatalystFactory).creationCode, abi.encode(baseGovFee)));
        (bytes32 salt, address expectedAddress) = mineSalt(initCodeHash, "000000");

        // DEPLOY
        vm.startBroadcast();
        address actualAddress = address(new CatalystFactory{salt: bytes32(salt)}(baseGovFee));
        vm.stopBroadcast();

        assertEq(actualAddress, expectedAddress);
        console2.log("Deployed factory at", actualAddress);
        console2.log("salt", uint256(salt));
    }

    function mathvol() public {
        bytes32 initCodeHash = keccak256(abi.encodePacked(type(CatalystMathVol).creationCode));
        (bytes32 salt, address expectedAddress) = mineSalt(initCodeHash, "0ca70");

        // DEPLOY
        vm.startBroadcast();
        address actualAddress = address(new CatalystMathVol{salt: bytes32(salt)}());
        vm.stopBroadcast();

        assertEq(actualAddress, expectedAddress);
        console2.log("Deployed vol lib at", actualAddress);
        console2.log("salt", uint256(salt));
    }

    function mathamp() public {
        bytes32 initCodeHash = keccak256(abi.encodePacked(type(CatalystMathAmp).creationCode));
        (bytes32 salt, address expectedAddress) = mineSalt(initCodeHash, "0ca70");

        // DEPLOY
        vm.startBroadcast();
        address actualAddress = address(new CatalystMathAmp{salt: bytes32(salt)}());
        vm.stopBroadcast();

        assertEq(actualAddress, expectedAddress);
        console2.log("Deployed amp lib at", actualAddress);
        console2.log("salt", uint256(salt));
    }

    function templatevolatile() public {
        bytes32 initCodeHash = keccak256(abi.encodePacked(type(CatalystVaultVolatile).creationCode, abi.encode(address(0x000000acd176D987fCf664f43491684760B90858), address(0x0ca7066880cD2F03d886461812fBf7544df56B19))));
        (bytes32 salt, address expectedAddress) = mineSalt(initCodeHash, "000000");

        // DEPLOY
        vm.startBroadcast();
        address actualAddress = address(new CatalystVaultVolatile{salt: bytes32(salt)}(address(0x000000acd176D987fCf664f43491684760B90858), address(0x0ca7066880cD2F03d886461812fBf7544df56B19)));
        vm.stopBroadcast();

        assertEq(actualAddress, expectedAddress);
        console2.log("Deployed vol template at", actualAddress);
        console2.log("salt", uint256(salt));
    }

    function templateamplified() public {
        bytes32 initCodeHash = keccak256(abi.encodePacked(type(CatalystVaultAmplified).creationCode, abi.encode(address(0x000000acd176D987fCf664f43491684760B90858), address(0x0ca70b3718EF58e3be946f5D09a7E8F44CB934ac))));
        (bytes32 salt, address expectedAddress) = mineSalt(initCodeHash, "000000");

        // DEPLOY
        vm.startBroadcast();
        address actualAddress = address(new CatalystVaultAmplified{salt: bytes32(salt)}(address(0x000000acd176D987fCf664f43491684760B90858), address(0x0ca70b3718EF58e3be946f5D09a7E8F44CB934ac)));
        vm.stopBroadcast();

        assertEq(actualAddress, expectedAddress);
        console2.log("Deployed amp template at", actualAddress);
        console2.log("salt", uint256(salt));
    }

    function run() public {
        // factory(); is 0x000000acd176D987fCf664f43491684760B90858 with salt 114709421156958415186027217025547827737296622131151675381220287808246552851098
        // mathvol(); is 0x0ca7066880cD2F03d886461812fBf7544df56B19 with salt 97319872752460098323199248802066309034939976127668417001544311163263852915521
        // mathamp(); is 0x0ca70b3718EF58e3be946f5D09a7E8F44CB934ac with salt 63722654287578236768624707543115016453003725919344306794241604926482092720364
        // templatevolatile(); is 0x0000004f4ac7F19dD840C6f032A1F2bD49FCD253 with salt 60662232750751397629041053606497919570818143080516418923077142053432447907648
        // templateamplified(); is 0x0000007802DdBEA102743a705306D9377303259a with salt 62066348516751541895847183559269671012091941168857148659880307387008083129007
    }
}