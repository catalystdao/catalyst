// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import { TestCommon } from "../TestCommon.t.sol";

contract TestConnectNewChain is TestCommon {
    event RemoteImplementationSet(bytes32 chainIdentifier, bytes remoteCCI, bytes remoteGARP);
    

    function test_connect_new_chain(bytes32 chainIdentifier, address remoteCCI, bytes calldata remoteGARP) external {
        bytes memory remoteCCIBytes = abi.encodePacked(
            uint8(20),
            bytes32(0),
            uint256(uint160(remoteCCI))
        );

        assertNotEq(
            keccak256(CCI.chainIdentifierToDestinationAddress(chainIdentifier)),
            keccak256(remoteCCIBytes)
        );

        vm.expectCall(
            address(GARP),
            abi.encodeCall(
                GARP.setRemoteImplementation,
                (
                    chainIdentifier, remoteGARP
                )
            )
        );
        vm.expectEmit();
        emit RemoteImplementationSet(chainIdentifier, remoteCCIBytes, remoteGARP);

        vm.prank(CCI.owner());
        CCI.connectNewChain(chainIdentifier, remoteCCIBytes, remoteGARP);

        assertEq(
            keccak256(CCI.chainIdentifierToDestinationAddress(chainIdentifier)),
            keccak256(remoteCCIBytes)
        );
    }

    function test_error_connect_not_new_chain(bytes32 chainIdentifier, address remoteCCI, bytes calldata remoteGARP) external {
        bytes memory remoteCCIBytes = abi.encodePacked(
            uint8(20),
            bytes32(0),
            uint256(uint160(remoteCCI))
        );

        address cciOwner = CCI.owner();

        vm.prank(cciOwner);
        CCI.connectNewChain(chainIdentifier, remoteCCIBytes, remoteGARP);

        vm.expectRevert(
            abi.encodeWithSignature(
                "ChainAlreadySetup()"
            )
        );

        vm.prank(cciOwner);
        CCI.connectNewChain(chainIdentifier, remoteCCIBytes, remoteGARP);

    }
}

