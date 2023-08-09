// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "../src/CatalystFactory.sol";
import "../src/CatalystVaultVolatile.sol";
import "../src/CatalystVaultAmplified.sol";
import "../src/CatalystGARPInterface.sol";

import "GARP/apps/mock/IncentivizedMockEscrow.sol";

contract TestCommon is Test {
    
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

    function deployVault (
        address vaultTemplate,
        address[] calldata assets,
        uint256[] calldata init_balances,
        uint256[] calldata weights,
        uint256 amp,
        uint256 vaultFee
    ) public returns(address vault) {
        vault = catFactory.deployVault(vaultTemplate, assets, init_balances, weights, amp, vaultFee, DEFAULT_POOL_NAME, DEFAULT_POOL_SYMBOL, address(CCI));
    }
}

