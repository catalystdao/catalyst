// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "../src/CatalystFactory.sol";
import "../src/CatalystVaultVolatile.sol";
import "../src/CatalystVaultAmplified.sol";
import "../src/CatalystGARPInterface.sol";
import {Token} from "./mocks/token.sol";

import {Bytes65} from "GARP/utils/Bytes65.sol";
import "GARP/apps/mock/IncentivizedMockEscrow.sol";
import { IMessageEscrowStructs } from "GARP/interfaces/IMessageEscrowStructs.sol";

contract TestCommon is Test, Bytes65, IMessageEscrowStructs {
    
    bytes32 constant DESTINATION_IDENTIFIER = bytes32(uint256(0x123123) + uint256(2**255));

    address SIGNER;
    uint256 PRIVATEKEY;

    IncentivizedMockEscrow GARP;

    CatalystFactory catFactory;
    CatalystVaultVolatile volatileTemplate; 
    CatalystVaultAmplified amplifiedTemplate;

    CatalystGARPInterface CCI;

    function setUp() virtual public {
        (SIGNER, PRIVATEKEY) = makeAddrAndKey("signer");

        catFactory = new CatalystFactory(0);

        volatileTemplate = new CatalystVaultVolatile(address(catFactory));
        amplifiedTemplate = new CatalystVaultAmplified(address(catFactory));

        GARP = new IncentivizedMockEscrow(DESTINATION_IDENTIFIER, SIGNER);

        CCI = new CatalystGARPInterface(address(GARP));
    }


    string DEFAULT_POOL_SYMBOL;
    string DEFAULT_POOL_NAME;

    function getTokens(uint256 N) internal returns(address[] memory tokens) {
        tokens = new address[](N);
        for (uint256 i = 0; i < N; ++i) {
            tokens[i] = address(deployToken());
        }
    }

    function getTokens(uint256 N, uint256[] memory balances) internal returns(address[] memory tokens) {
        tokens = new address[](N);
        for (uint256 i = 0; i < N; ++i) {
            tokens[i] = address(deployToken(18, balances[i]));
        }
    }

    function approveTokens(address target, address[] memory tokens, uint256[] memory amounts) internal {
        for (uint256 i = 0; i < tokens.length; ++i) {
            Token(tokens[i]).approve(target, amounts[i]);
        }
    }

    function approveTokens(address target, address[] memory tokens) internal {
        uint256[] memory amounts = new uint256[](tokens.length);

        for (uint256 i = 0; i < amounts.length; ++i) {
            amounts[i] = 2**256 - 1;
        }

        approveTokens(target, tokens, amounts);
    }

    function verifyBalances(address target, address[] memory tokens, uint256[] memory amounts) internal {
        for (uint256 i = 0; i < tokens.length; ++i) {
            assertEq(
                Token(tokens[i]).balanceOf(target),
                amounts[i],
                "verifyBalances(...) failed"
            );
        }
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

    function deployToken(
        string memory name,
        string memory symbol,
        uint8 decimals_,
        uint256 initialSupply
    ) internal returns (Token token) {
        return token = new Token(name, symbol, decimals_, initialSupply);
    }

    function deployToken(
        uint8 decimals_,
        uint256 initialSupply
    ) internal returns (Token token) {
        return token = deployToken("Token", "TKN", decimals_, initialSupply);
    }

    function deployToken() internal returns(Token token) {
        return deployToken(18, 1e6);
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

