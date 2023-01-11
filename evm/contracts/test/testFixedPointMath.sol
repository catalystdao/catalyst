//SPDX-License-Identifier: Unlicsened

pragma solidity ^0.8.16;

import "../FixedPointMath.sol";

contract testFixedPointMath is CatalystFixedPointMath {
    function imul(uint256 a, uint256 b) external pure returns (uint256) {
        return mulX64(a, b);
    }

    function ibigdiv(uint256 a, uint256 b) external pure returns (uint256) {
        return bigdiv64(a, b);
    }

    function ilog2X64(uint256 x) external returns (uint256) {
        return log2X64(x);
    }

    function ip2X64(uint256 x) external returns (uint256) {
        return p2X64(x);
    }

    function iinvp2X64(uint256 x) external pure returns (uint256) {
        return invp2X64(x);
    }

    function iinvp2TaylorX64(uint256 x) external pure returns (uint256) {
        return invp2TaylorX64(x);
    }

    function ifpowX64(uint256 x, uint256 p) external returns (uint256) {
        return fpowX64(x, p);
    }

    function iinvfpowX64(uint256 x, uint256 p) external pure returns (uint256) {
        return invfpowX64(x, p);
    }

    function isafe_fpowX64(
        uint256 a,
        uint256 b,
        uint256 p
    ) external pure returns (uint256) {
        return safe_fpowX64(a, b, p);
    }
}
