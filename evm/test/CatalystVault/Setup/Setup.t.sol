// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../../TestCommon.t.sol";
import "src/ICatalystV1Vault.sol";
import "solady/utils/FixedPointMathLib.sol";
import { Token } from "../../mocks/token.sol";
import { AVaultInterfaces } from "../AVaultInterfaces.t.sol";
import { CatalystVaultCommon } from "src/CatalystVaultCommon.sol";
import { ICatalystV1Vault } from "src/ICatalystV1Vault.sol";
import { ICatalystV1Structs } from "src/interfaces/ICatalystV1VaultState.sol";
import { ICatalystV1FactoryEvents } from "src/interfaces/ICatalystV1FactoryEvents.sol";

abstract contract TestSetup is TestCommon, AVaultInterfaces {

    uint256 constant MAX_ASSETS = 3;


    function test_Setup() external {

        address[] memory assets = getTokens(MAX_ASSETS);
        uint256[] memory initBalances = new uint256[](MAX_ASSETS);
        uint256[] memory weights = new uint256[](MAX_ASSETS);
        for (uint i = 0; i < MAX_ASSETS; i++) {
            initBalances[i] = 10**18;
            weights[i] = 1;
            Token(assets[i]).approve(address(catFactory), initBalances[i]);
        }
        
        uint64 amplification = amplified ? 10**18/2 : 10**18;
        address vaultTemplate = amplified ? address(amplifiedTemplate) : address(volatileTemplate);



        // Tested action
        vm.recordLogs();
        address vault = catFactory.deployVault(
            vaultTemplate,
            assets,
            initBalances,
            weights,
            amplification,
            0,
            DEFAULT_POOL_NAME,
            DEFAULT_POOL_SYMBOL,
            address(CCI)
        );



        // Verify the 'VaultDeployed' event
        Vm.Log[] memory logs = vm.getRecordedLogs();
        Vm.Log memory vaultDeployedLog = logs[12];

        assertEq(vaultDeployedLog.topics[1], bytes32(uint256(uint160(vaultTemplate))));
        assertEq(vaultDeployedLog.topics[2], bytes32(uint256(uint160(address(CCI)))));
        assertEq(vaultDeployedLog.topics[3], bytes32(uint256(uint160(address(this)))));

        (
            address logVaultAddress,
            address[] memory logAssets,
            uint256 logK
        ) = abi.decode(vaultDeployedLog.data, (address, address[], uint256));

        assertEq(logVaultAddress, vault);
        assertEq(logAssets[0], assets[0]);
        assertEq(logAssets[1], assets[1]);
        assertEq(logAssets[2], assets[2]);
        assertEq(logK, amplification);

        // Verify the pool tokens mint
        uint256 deployerPoolTokenBalance = Token(vault).balanceOf(address(this));
        assertEq(deployerPoolTokenBalance, 10**18);
    }


    function test_SetupNoTokens() external {

        address[] memory assets = new address[](0);
        uint256[] memory initBalances = new uint256[](0);
        uint256[] memory weights = new uint256[](0);
        
        uint64 amplification = amplified ? 10**18/2 : 10**18;



        // Tested action
        vm.expectRevert();
        deployVault(assets, initBalances, weights, amplification, 0);

    }


    function test_SetupTooManyTokens() external {

        uint256 numTokens = MAX_ASSETS + 1;

        address[] memory assets = getTokens(numTokens);
        uint256[] memory initBalances = new uint256[](numTokens);
        uint256[] memory weights = new uint256[](numTokens);
        for (uint i = 0; i < numTokens; i++) {
            initBalances[i] = 10**18;
            weights[i] = 1;
            Token(assets[i]).approve(address(catFactory), initBalances[i]);
        }
        
        uint64 amplification = amplified ? 10**18/2 : 10**18;
        address vaultTemplate = amplified ? address(amplifiedTemplate) : address(amplifiedTemplate);



        // Tested action
        vm.expectRevert();
        catFactory.deployVault(
            vaultTemplate,
            assets,
            initBalances,
            weights,
            amplification,
            0,
            DEFAULT_POOL_NAME,
            DEFAULT_POOL_SYMBOL,
            address(CCI)
        );

    }


    function test_SetupNoBalanceSet() external {

        address[] memory assets = getTokens(MAX_ASSETS);
        uint256[] memory initBalances = new uint256[](MAX_ASSETS);
        uint256[] memory weights = new uint256[](MAX_ASSETS);
        for (uint i = 0; i < MAX_ASSETS; i++) {
            initBalances[i] = 10**18;
            weights[i] = 1;
            Token(assets[i]).approve(address(catFactory), initBalances[i]);
        }
        
        uint64 amplification = amplified ? 10**18/2 : 10**18;
        address vaultTemplate = amplified ? address(amplifiedTemplate) : address(amplifiedTemplate);

        // ! Set the last balance argument to 0
        initBalances[MAX_ASSETS-1] = 0;



        // Tested action
        vm.expectRevert();
        catFactory.deployVault(
            vaultTemplate,
            assets,
            initBalances,
            weights,
            amplification,
            0,
            DEFAULT_POOL_NAME,
            DEFAULT_POOL_SYMBOL,
            address(CCI)
        );

    }


    function test_SetupNoWeightSet() external {

        address[] memory assets = getTokens(MAX_ASSETS);
        uint256[] memory initBalances = new uint256[](MAX_ASSETS);
        uint256[] memory weights = new uint256[](MAX_ASSETS);
        for (uint i = 0; i < MAX_ASSETS; i++) {
            initBalances[i] = 10**18;
            weights[i] = 1;
        }
        
        uint64 amplification = amplified ? 10**18/2 : 10**18;
        address vaultTemplate = amplified ? address(amplifiedTemplate) : address(amplifiedTemplate);

        // ! Set the last weight argument to 0
        weights[MAX_ASSETS-1] = 0;



        // Tested action
        vm.expectRevert();
        catFactory.deployVault(
            vaultTemplate,
            assets,
            initBalances,
            weights,
            amplification,
            0,
            DEFAULT_POOL_NAME,
            DEFAULT_POOL_SYMBOL,
            address(CCI)
        );

    }


    function test_SetupWithoutFunds() external {

        address[] memory assets = getTokens(MAX_ASSETS);
        uint256[] memory initBalances = new uint256[](MAX_ASSETS);
        uint256[] memory weights = new uint256[](MAX_ASSETS);
        for (uint i = 0; i < MAX_ASSETS; i++) {
            initBalances[i] = 10**18;
            weights[i] = 1;
            // ! Do not set token allowances
        }
        
        uint64 amplification = amplified ? 10**18/2 : 10**18;
        address vaultTemplate = amplified ? address(amplifiedTemplate) : address(volatileTemplate);



        // Tested action
        vm.expectRevert(abi.encodePacked(uint32(0x7939f424))); // TRANSFER_FROM_FAILED
        catFactory.deployVault(
            vaultTemplate,
            assets,
            initBalances,
            weights,
            amplification,
            0,
            DEFAULT_POOL_NAME,
            DEFAULT_POOL_SYMBOL,
            address(CCI)
        );

    }


    function test_SetupInvalidTemplate() external {

        address[] memory assets = getTokens(MAX_ASSETS);
        uint256[] memory initBalances = new uint256[](MAX_ASSETS);
        uint256[] memory weights = new uint256[](MAX_ASSETS);
        for (uint i = 0; i < MAX_ASSETS; i++) {
            initBalances[i] = 10**18;
            weights[i] = 1;
            Token(assets[i]).approve(address(catFactory), initBalances[i]);
        }
        
        uint64 amplification = amplified ? 10**18/2 : 10**18;

        // ! Set the wrong template on purpose
        address vaultTemplate = amplified ? address(volatileTemplate) : address(amplifiedTemplate);



        // Tested action
        vm.expectRevert();
        catFactory.deployVault(
            vaultTemplate,
            assets,
            initBalances,
            weights,
            amplification,
            0,
            DEFAULT_POOL_NAME,
            DEFAULT_POOL_SYMBOL,
            address(CCI)
        );

    }


    function test_CallSetupAfterDeploy() external {

        address[] memory assets = getTokens(MAX_ASSETS);
        uint256[] memory initBalances = new uint256[](MAX_ASSETS);
        uint256[] memory weights = new uint256[](MAX_ASSETS);
        for (uint i = 0; i < MAX_ASSETS; i++) {
            initBalances[i] = 10**18;
            weights[i] = 1;
            Token(assets[i]).approve(address(catFactory), initBalances[i]);
        }
        
        uint64 amplification = amplified ? 10**18/2 : 10**18;
        address vaultTemplate = amplified ? address(amplifiedTemplate) : address(volatileTemplate);

        ICatalystV1Vault vault = ICatalystV1Vault(catFactory.deployVault(
            vaultTemplate,
            assets,
            initBalances,
            weights,
            amplification,
            0,
            DEFAULT_POOL_NAME,
            DEFAULT_POOL_SYMBOL,
            address(CCI)
        ));



        // Tested action
        vm.expectRevert(0xf92ee8a9);
        vault.setup(
            DEFAULT_POOL_NAME,
            DEFAULT_POOL_SYMBOL,
            address(CCI),
            0,
            0,
            address(this),
            address(this)
        );

    }


    function test_CallInitializeSwapCurvesAfterDeploy() external {

        address[] memory assets = getTokens(MAX_ASSETS);
        uint256[] memory initBalances = new uint256[](MAX_ASSETS);
        uint256[] memory weights = new uint256[](MAX_ASSETS);
        for (uint i = 0; i < MAX_ASSETS; i++) {
            initBalances[i] = 10**18;
            weights[i] = 1;
            Token(assets[i]).approve(address(catFactory), initBalances[i]);
        }
        
        uint64 amplification = amplified ? 10**18/2 : 10**18;
        address vaultTemplate = amplified ? address(amplifiedTemplate) : address(volatileTemplate);

        ICatalystV1Vault vault = ICatalystV1Vault(catFactory.deployVault(
            vaultTemplate,
            assets,
            initBalances,
            weights,
            amplification,
            0,
            DEFAULT_POOL_NAME,
            DEFAULT_POOL_SYMBOL,
            address(CCI)
        ));



        // Tested action
        vm.expectRevert();
        vault.initializeSwapCurves(
            assets,
            weights,
            amplification,
            address(this)
        );

    }
    
}
