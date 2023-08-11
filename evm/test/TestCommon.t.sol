// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "../src/CatalystFactory.sol";
import "../src/registry/CatalystMathVol.sol";
import "../src/CatalystVaultVolatile.sol";
import "../src/registry/CatalystMathAmp.sol";
import "../src/CatalystVaultAmplified.sol";
import "../src/CatalystGARPInterface.sol";
import {Token} from "./mocks/token.sol";
import {TestTokenFunctions} from "./CommonTokenFunctions.t.sol";

import {Bytes65} from "GeneralisedIncentives/src/utils/Bytes65.sol";
import "GeneralisedIncentives/src/apps/mock/IncentivizedMockEscrow.sol";
import { IMessageEscrowStructs } from "GeneralisedIncentives/src/interfaces/IMessageEscrowStructs.sol";

contract TestCommon is Test, Bytes65, IMessageEscrowStructs, TestTokenFunctions {
    
    bytes32 constant DESTINATION_IDENTIFIER = bytes32(uint256(0x123123) + uint256(2**255));

    address SIGNER;
    uint256 PRIVATEKEY;

    IncentivizedMockEscrow GARP;

    CatalystFactory catFactory;
    CatalystMathVol volatileMathlib; 
    CatalystVaultVolatile volatileTemplate; 
    CatalystMathAmp amplifiedMathlib; 
    CatalystVaultAmplified amplifiedTemplate;

    CatalystGARPInterface CCI;

    function setUp() virtual public {
        (SIGNER, PRIVATEKEY) = makeAddrAndKey("signer");

        catFactory = new CatalystFactory(0);

        volatileMathlib = new CatalystMathVol();
        volatileTemplate = new CatalystVaultVolatile(address(catFactory), address(volatileMathlib));

        amplifiedMathlib = new CatalystMathAmp();
        amplifiedTemplate = new CatalystVaultAmplified(address(catFactory), address(amplifiedMathlib));

        GARP = new IncentivizedMockEscrow(DESTINATION_IDENTIFIER, SIGNER);

        CCI = new CatalystGARPInterface(address(GARP));
    }

    function deployVault (
        address[] memory assets,
        uint256[] memory init_balances,
        uint256[] memory weights,
        uint256 amp,
        uint256 vaultFee
    ) internal returns(address vault) {

        for (uint256 i = 0; i < assets.length; ++i) {
            Token(assets[i]).approve(address(catFactory), init_balances[i]);
        }

        address vaultTemplate = amp == 10**18 ? address(volatileTemplate) : address(amplifiedTemplate);

        vault = catFactory.deployVault(
            vaultTemplate,
            assets, init_balances, weights, amp, vaultFee, DEFAULT_POOL_NAME, DEFAULT_POOL_SYMBOL, address(CCI));
    }

    function setConnection(address vault1, address vault2, bytes32 chainIdentifier1, bytes32 chainIdentifier2) internal {
        setUpChains(chainIdentifier1);
        if (chainIdentifier1 != chainIdentifier2) setUpChains(chainIdentifier2);

        ICatalystV1Vault(vault1).setConnection(
            chainIdentifier2,
            convertEVMTo65(vault2),
            true
        );

        ICatalystV1Vault(vault2).setConnection(
            chainIdentifier1,
            convertEVMTo65(vault1),
            true
        );
    }

    function setUpChains(bytes32 chainIdentifier) internal {
        CCI.connectNewChain(chainIdentifier, convertEVMTo65(address(CCI)), abi.encode(address(GARP)));
    }

    function signMessageForMock(bytes memory message) internal view returns(uint8 v, bytes32 r, bytes32 s) {
        (v, r, s) = vm.sign(PRIVATEKEY, keccak256(message));
    }

    function getVerifiedMessage(address emitter, bytes memory message) internal view returns(bytes memory _metadata, bytes memory newMessage) {
        newMessage = abi.encodePacked(bytes32(uint256(uint160(emitter))), message);

        (uint8 v, bytes32 r, bytes32 s) = signMessageForMock(newMessage);

        _metadata = abi.encode(v, r, s);
    }
}

