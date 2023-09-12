// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

abstract contract AVaultInterfaces  {

    bool internal amplified;

    function invariant(address[] memory vaults) view virtual internal returns(uint256 inv);

    function getTestConfig() virtual internal view returns(address[] memory vaults);

    function getLargestSwap(address fromVault, address toVault, address fromAsset, address toAsset) virtual internal returns(uint256 amount);

    function getLargestSwap(address fromVault, address toVault, address fromAsset, address toAsset, bool securityLimit) virtual internal returns(uint256 amount);

}

