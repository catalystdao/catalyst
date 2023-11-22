// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.19;

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

import { DeployContracts, JsonContracts } from "../script/DeployContracts.s.sol";

contract TestCommon is Test, Bytes65, IMessageEscrowStructs, TestTokenFunctions, DeployContracts {

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

    address SEND_LOST_GAS_TO;

    function setUp() virtual public {
        (SIGNER, PRIVATEKEY) = makeAddrAndKey("signer");
        SEND_LOST_GAS_TO = makeAddr("sendLostGasTo");

        _INCENTIVE.refundGasTo = makeAddr("refundGasTo");

        deployAllContracts(address(this));

        catFactory = CatalystFactory(contracts.factory);

        volatileMathlib = CatalystMathVol(contracts.volatile_mathlib);
        volatileTemplate = CatalystVaultVolatile(contracts.volatile_template);

        amplifiedMathlib = CatalystMathAmp(contracts.amplified_mathlib);
        amplifiedTemplate = CatalystVaultAmplified(contracts.amplified_template);

        GARP = new IncentivizedMockEscrow(SEND_LOST_GAS_TO, DESTINATION_IDENTIFIER, SIGNER, 0);

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

    function constructSendAsset(address sourceVault, address destinationVault, address toAccount, uint256 units, uint8 toAssetIndex, uint256 minOut, uint256 fromAmount, address fromAsset, uint32 blocknumber, uint16 underwriteIncentiveX16, bytes memory cdata) pure internal returns(bytes memory message) {
        message = abi.encodePacked(
            CTX0_ASSET_SWAP,
            abi.encodePacked(
                uint8(20),
                bytes32(0),
                abi.encode(sourceVault)
            ),
            abi.encodePacked(
                uint8(20),
                bytes32(0),
                abi.encode(destinationVault)
            ),
            abi.encodePacked(
                uint8(20),
                bytes32(0),
                abi.encode(toAccount)
            ),
            units,
            uint8(toAssetIndex),
            minOut,
            fromAmount,
            abi.encodePacked(
                uint8(20),
                bytes32(0),
                abi.encode(fromAsset)
            ),
            uint32(blocknumber),
            uint16(underwriteIncentiveX16),
            uint16(cdata.length),
            cdata
        );
    }

    function constructSendAsset(address sourceVault, address destinationVault, address toAccount, uint256 units, uint8 toAssetIndex, uint256 minOut, uint256 fromAmount, address fromAsset) view internal returns(bytes memory message) {
        message = constructSendAsset(sourceVault, destinationVault, toAccount, units, toAssetIndex, minOut, fromAmount, fromAsset, uint32(block.number), uint16(0), hex"");
    }

    function constructSendAsset(address sourceVault, address destinationVault, address toAccount, uint256 units, uint8 toAssetIndex) view internal returns(bytes memory message) {
        message = constructSendAsset(sourceVault, destinationVault, toAccount, units, toAssetIndex, 0, 0, address(0), uint32(block.number), uint16(0), hex"");
    }

    function constructSendLiquidity(address sourceVault, address destinationVault, address toAccount, uint256 units, uint256[2] memory minOuts, uint256 fromAmount, uint32 blocknumber, uint16 underwriteIncentiveX16, bytes memory cdata) pure internal returns(bytes memory message) {
        message = abi.encodePacked(
            CTX1_LIQUIDITY_SWAP,
            abi.encodePacked(
                uint8(20),
                bytes32(0),
                abi.encode(sourceVault)
            ),
            abi.encodePacked(
                uint8(20),
                bytes32(0),
                abi.encode(destinationVault)
            ),
            abi.encodePacked(
                uint8(20),
                bytes32(0),
                abi.encode(toAccount)
            ),
            units,
            minOuts[0],
            minOuts[1],
            fromAmount,
            uint32(blocknumber),
            uint16(cdata.length),
            cdata
        );
    }

    function constructSendLiquidity(address sourceVault, address destinationVault, address toAccount, uint256 units, uint256[2] memory minOuts, uint256 fromAmount) view internal returns(bytes memory message) {
        message = constructSendLiquidity(sourceVault, destinationVault, toAccount, units, minOuts, fromAmount, uint32(block.number), uint16(0), hex"");
    }

    function constructSendLiquidity(address sourceVault, address destinationVault, address toAccount, uint256 units) view internal returns(bytes memory message) {
        uint256[2] memory minOuts = [uint256(0), uint256(0)];
        message = constructSendLiquidity(sourceVault, destinationVault, toAccount, units, minOuts, 0, uint32(block.number), uint16(0), hex"");
    }

    function addGARPContext(bytes32 messageIdentifier, address fromApplication, address destinationAddress, bytes memory message) internal returns(bytes memory package) {
        return abi.encodePacked(
            bytes1(0x00),
            messageIdentifier,
            abi.encodePacked(
                uint8(20),
                bytes32(0),
                bytes32(abi.encode(fromApplication))
            ),
            abi.encodePacked(
                uint8(20),
                bytes32(0),
                bytes32(abi.encode(destinationAddress))
            ),
            uint48(4000000),
            message
        );
    }
    
    function addMockContext(bytes memory message) internal returns(bytes memory package) {
        return abi.encodePacked(
            DESTINATION_IDENTIFIER,
            DESTINATION_IDENTIFIER,
            message
        );
    }
}

