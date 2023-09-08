// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "../src/CatalystFactory.sol";
import "../src/registry/CatalystMathVol.sol";
import "../src/CatalystVaultVolatile.sol";
import "../src/registry/CatalystMathAmp.sol";
import "../src/CatalystVaultAmplified.sol";
import "../src/CatalystChainInterface.sol";
import {Token} from "./mocks/token.sol";
import {TestTokenFunctions} from "./CommonTokenFunctions.t.sol";

import {Bytes65} from "GeneralisedIncentives/src/utils/Bytes65.sol";
import "GeneralisedIncentives/src/apps/mock/IncentivizedMockEscrow.sol";
import { IMessageEscrowStructs } from "GeneralisedIncentives/src/interfaces/IMessageEscrowStructs.sol";

import { DeployCatalyst, JsonContracts } from "../script/DeployCatalyst.s.sol";

contract TestCommon is Test, Bytes65, IMessageEscrowStructs, TestTokenFunctions, DeployCatalyst {

    // add this to be excluded from coverage report
    function test() public {}

    
    bytes32 constant DESTINATION_IDENTIFIER = bytes32(uint256(0x123123) + uint256(2**255));

    address SIGNER;
    uint256 PRIVATEKEY;

    IncentiveDescription _INCENTIVE = IncentiveDescription({
        maxGasDelivery: 1199199,
        maxGasAck: 1188188,
        refundGasTo: address(0),
        priceOfDeliveryGas: 123321,
        priceOfAckGas: 321123,
        targetDelta: 30 minutes
    });

    IncentivizedMockEscrow GARP;

    CatalystFactory catFactory;
    CatalystMathVol volatileMathlib; 
    CatalystVaultVolatile volatileTemplate; 
    CatalystMathAmp amplifiedMathlib; 
    CatalystVaultAmplified amplifiedTemplate;

    CatalystChainInterface CCI;

    function setUp() virtual public {
        (SIGNER, PRIVATEKEY) = makeAddrAndKey("signer");

        _INCENTIVE.refundGasTo = makeAddr("refundGasTo");

        deployAllContracts();

        catFactory = CatalystFactory(contracts.factory);

        volatileMathlib = CatalystMathVol(contracts.volatile_mathlib);
        volatileTemplate = CatalystVaultVolatile(contracts.volatile_template);

        amplifiedMathlib = CatalystMathAmp(contracts.amplified_mathlib);
        amplifiedTemplate = CatalystVaultAmplified(contracts.amplified_template);

        GARP = new IncentivizedMockEscrow(DESTINATION_IDENTIFIER, SIGNER, 0);

        CCI = new CatalystChainInterface(address(GARP), address(this));
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
            assets, init_balances, weights, amp, vaultFee, DEFAULT_POOL_NAME, DEFAULT_POOL_SYMBOL, address(CCI)
        );
    }

    function simpleVault(uint256 numTokens) internal returns(address vault) {
        address[] memory assets = new address[](numTokens);
        uint256[] memory init_balances = new uint256[](numTokens);
        uint256[] memory weights = new uint256[](numTokens);
        
        for (uint256 i = 0; i < numTokens; i++) {
            assets[i] = address(new Token("TEST", "TEST", 18, 1e6));
            init_balances[i] = 1000 * 1e18;
            weights[i] = 1;
        }

        uint256 amp = 10**18;
        uint256 vaultFee = 0;

        return vault = deployVault(assets, init_balances, weights, amp, vaultFee);
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

    function _getTotalIncentive(IncentiveDescription memory incentive) internal pure returns(uint256) {
        return incentive.maxGasDelivery * incentive.priceOfDeliveryGas + incentive.maxGasAck * incentive.priceOfAckGas;
    }

    function getVerifiedMessage(address emitter, bytes memory message) internal view returns(bytes memory _metadata, bytes memory newMessage) {
        newMessage = abi.encodePacked(bytes32(uint256(uint160(emitter))), message);

        (uint8 v, bytes32 r, bytes32 s) = signMessageForMock(newMessage);

        _metadata = abi.encode(v, r, s);
    }

    function sliceMemory(bytes calldata a, uint256 start) pure external returns(bytes calldata) {
        return a[start:];
    }

    function sliceMemory(bytes calldata a, uint256 start, uint256 end) pure external returns(bytes calldata) {
        return a[start:end];
    }

    function getVaultTokens(address vault) internal returns(address[] memory vault_tokens) {
        uint256 numTokens;
        for (numTokens = 0; numTokens < 256; ++numTokens) {
            address token = ICatalystV1Vault(vault)._tokenIndexing(numTokens);
            if (token == address(0)) break;
        }
        vault_tokens = new address[](numTokens);
        for (uint256 i = 0; i < numTokens; ++i) {
            address token = ICatalystV1Vault(vault)._tokenIndexing(i);
            vault_tokens[i] = token;
        }
    }
}

