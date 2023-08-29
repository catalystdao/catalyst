// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";

contract FundAddresses is Script {

    function run() external {
        uint256 CATALYST_SHARE = 110;
        address CATALYST_ADDRESS = vm.envAddress("CATALYST_ADDRESS");

        uint256 CATALYST_ROUTER_SHARE = 5;
        address CATALYST_ROUTER_ADDRESS =vm.envAddress("CATALYST_ROUTER_ADDRESS");

        uint256 CATALYST_INTERFACES_SHARE = 10;
        address CATALYST_INTERFACES_ADDRESS = vm.envAddress("CATALYST_INTERFACES_ADDRESS");

        uint256 MOCK_SHARE = 10;
        address MOCK_ADDRESS = vm.envAddress("MOCK_ADDRESS");

        uint256 TOKENS_SHARE = 10;
        address TOKENS_ADDRESS = vm.envAddress("TOKENS_ADDRESS");

        uint256 WGAS_SHARE = 5;
        address WGAS_ADDRESS = vm.envAddress("WGAS_DEPLOYER_ADDRESS");


        uint256 total = 1 ether;
        uint256 sum = CATALYST_SHARE + CATALYST_ROUTER_SHARE + CATALYST_INTERFACES_SHARE + MOCK_SHARE + TOKENS_SHARE + WGAS_SHARE;

        vm.startBroadcast(vm.envUint("BASE_DEPLOYER_KEY"));

        if (CATALYST_SHARE > 0 )            payable(CATALYST_ADDRESS)           .transfer(CATALYST_SHARE * total / sum);
        if (CATALYST_ROUTER_SHARE > 0 )     payable(CATALYST_ROUTER_ADDRESS)    .transfer(CATALYST_ROUTER_SHARE * total / sum);
        if (CATALYST_INTERFACES_SHARE > 0 ) payable(CATALYST_INTERFACES_ADDRESS).transfer(CATALYST_INTERFACES_SHARE * total / sum);
        if (MOCK_SHARE > 0 )                payable(MOCK_ADDRESS)               .transfer(MOCK_SHARE * total / sum);
        if (TOKENS_SHARE > 0 )              payable(TOKENS_ADDRESS)             .transfer(TOKENS_SHARE * total / sum);
        if (WGAS_SHARE > 0 )                payable(WGAS_ADDRESS)               .transfer(WGAS_SHARE * total / sum);


        uint256 VAULT_AMOUNT = 1 ether;
        address VAULT_ADDRESS=vm.envAddress("VAULT_ADDRESS");
        if (VAULT_AMOUNT > 0 )               payable(VAULT_ADDRESS)             .transfer(VAULT_AMOUNT);

        vm.stopBroadcast();
    }
}

