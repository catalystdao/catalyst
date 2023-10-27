// SPDX-License-Identifier: UNLICENSED
pragma solidity =0.8.19;

abstract contract AVaultInterfaces  {

    bool internal amplified;

    function invariant(address[] memory vaults) view virtual internal returns(uint256 inv);

    function strong_invariant(address vault) view virtual internal returns(uint256 inv);

    function getTestConfig() virtual internal view returns(address[] memory vaults);

    function getLargestSwap(address fromVault, address toVault, address fromAsset, address toAsset) virtual internal returns(uint256 amount);

    function getLargestSwap(address fromVault, address toVault, address fromAsset, address toAsset, bool securityLimit) virtual internal returns(uint256 amount);

    function getWithdrawPercentages(address vault, uint256[] memory withdraw_weights) virtual internal returns(uint256[] memory new_weights) {
        uint256 progressiveSum = 0;
        new_weights = new uint256[](withdraw_weights.length);
        for (uint256 j = withdraw_weights.length - 1; ; ) {
            uint256 ww = withdraw_weights[j];
            progressiveSum += ww;
            new_weights[j] = ww * 10**18 / progressiveSum;
            if (j == 0) {
                break;
            }
            --j;
        }
        return new_weights;
    }

}

