// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

abstract contract AVaultInterfaces  {

    function invariant(address[] memory vaults) view virtual internal returns(uint256 inv);

    function getTestConfig() virtual internal returns(address[] memory vaults);

    function getLargestSwap(address fromVault, address toVault, address fromAsset, address toAsset) virtual internal returns(uint256 amount);

    function getLargestSwap(address fromVault, address toVault, address fromAsset, address toAsset, bool securityLimit) virtual internal returns(uint256 amount);

}

