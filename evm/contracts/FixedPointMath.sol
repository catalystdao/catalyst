//SPDX-License-Identifier: Unlicsened

pragma solidity >=0.8.17 <0.9.0;

/// @title Catalyst Fixed Point Mathematics Library
/// @author
///     Alexander @ Catalyst
///     Copyright reserved by Catalyst
/// @notice
///     Fixed point mathematics used by Polymer.
///     If a fixed point number is stored inside uint256, the variable
///     should be clearly marked. For example, if 64 bits are reserved
///     for the decimal, the variable should have X64 appened.
///
///     Contains the following mathematical descriptions in Solidity:
///         mulX64
///         a : X64, b : X64
///         a · b => y : X64, as long as y does not overflow X64.
///
///         bigdiv64
///         a : uint256, b : uint256
///         (a << 64)/b => y, as long as y does not overflow uint256.
///
///         log2X64
///         x : uint256
///         log2(x) => y : X64, as long as y >= 0
///
///         p2X64
///         x : X64, x < 192
///         2**x => y : X64
///
///         fpowX64
///         x : X64, p : X64
///         x**p => y : X64, depends on p2 and log2
///@dev All power functions have inverse companions, which may have other limitations.
contract CatalystFixedPointMath {
    // Natural logarithm of 2.
    uint256 constant LN2 = 12786308645202655660;

    // The number of decimal points in the mathematical library.
    // The number 1 will then be equal to 2**pXX.
    uint256 constant pXX = 64;
    uint256 constant ONE = 2**pXX; // Instead of repeating 2**pXX everywhere, it is simpler to write ONE
    uint256 constant ONEONE = 2**(pXX * 2); // Shortcut for 2**pXX * 2**pXX = 2**(2*pXX)

    constructor() {}

    /// @notice Safely calculates a : X64 times b : X64 while returning X64.
    /// @dev
    ///     Reverts if a · b > 2**(256-64)-1
    ///     Credit: https://medium.com/wicketh/mathemagic-full-multiply-27650fec525d
    /// @param a uint256, X64 Factor 1
    /// @param b uint256, X64 Factor 2
    /// @return uint256, X64:  a times b
    function mulX64(uint256 a, uint256 b) internal pure returns (uint256) {
        uint256 r0;
        uint256 r1;
        unchecked {
            r0 = a * b; // Overflow is desired
        }
        r1 = mulmod(a, b, type(uint256).max);
        r1 = (r1 - r0) - (r1 < r0 ? 1 : 0);

        // !CRITICAL! Check for overflow into the third resolution.
        // We know r1 expand with shift(r1, 256-pXX)
        // r1 · 2^(256-64) = r1 · 2^192 < 2^256
        // r1 < 2^64
        // r1 < 2**64 - 1. Minus because we are indexing from 0.
        require(r1 < 2**64 - 1);

        // The overflow part is now stored in r1 while the remainder is in r0.
        // The true number is thus r1 · 2^256 + r0
        // This could be improved to
        uint256 points = r0 >> pXX;

        return (r1 << (256 - pXX)) + points;
    }

    /// @notice Safely calculates (a << 64)/b
    /// @dev
    ///     Reverts normally if result overflows.
    ///     To get (a << p)/b replace (2**64 - 1) by (2**p - 1)
    ///     While one could theoretically make these dynamically, hardcoding
    ///     the shift is cheaper.
    ///     Credit: https://github.com/vyperlang/vyper/issues/1086
    /// @param a uint256 numerator
    /// @param b uint256 denominator
    /// @return uint256 (a << 64)/b
    function bigdiv64(uint256 a, uint256 b) internal pure returns (uint256) {
        uint256 m = (ONE - 1) % b;
        uint256 r = (ONE - 1) / b;
        return r * a + ((m + 1) * a) / b;
    }

    /// @notice
    ///     Fixed point number can be written as
    ///     x = m · 2^(-pXX)
    ///     log2(x) = log2(m · 2^(-pXX)) = log2(m) + log2(2^(-pXX))
    ///     log2(x) = log2(m) - pXX
    ///     This finds the integer part
    ///
    ///     Let a be the integer part of log2, then
    ///     log2(x) - a is the decimal part.
    ///     log2(x) - log2(2^a) = log2(x/2^a)
    ///     x/2^a is definitely in [1, 2) and only 1 if number could be expressed as 2^a.
    /// @dev
    ///      for i in range(V) goes through the remaining bits.
    ///      Set v to the smaller bit one wants included
    /// @param x uint256, X64
    /// @return uint256, X64 as log2(x/2**64)*2**64
    function log2X64(uint256 x) internal pure returns (uint256) {
        require(x >= 2**64);
        uint256 x_i = x;
        uint256 log2_intermediate = 0;

        unchecked {
            if (x_i >= 2**128) {
                x_i = x_i >> 128;
                log2_intermediate += 128;
            }
            if (x_i >= 2**64) {
                x_i = x_i >> 64;
                log2_intermediate += 64;
            }
            if (x_i >= 2**32) {
                x_i = x_i >> 32;
                log2_intermediate += 32;
            }
            if (x_i >= 2**16) {
                x_i = x_i >> 16;
                log2_intermediate += 16;
            }
            if (x_i >= 2**8) {
                x_i = x_i >> 8;
                log2_intermediate += 8;
            }
            if (x_i >= 2**4) {
                x_i = x_i >> 4;
                log2_intermediate += 4;
            }
            if (x_i >= 2**2) {
                x_i = x_i >> 2;
                log2_intermediate += 2;
            }
            if (x_i >= 2**1) {
                // x_i = x_i >> 1;
                log2_intermediate += 1;
            }
            log2_intermediate -= pXX;

            // Secure the decimal point
            x_i = x / (1 << log2_intermediate);
            log2_intermediate = log2_intermediate << pXX;
            // for (uint256 i = 0; i < 24; i++) {
            //     // 24 is Supposedly: 1/2**24 => .0.0000059605% deviation, but I am getting more like 1/2**20 deviation => .0000953674% deviation
            //     if (x_i >= 2 << pXX) {
            //         log2_intermediate += 1 << (pXX - i);
            //         x_i = x_i >> 1;
            //     }
            //     x_i = (x_i * x_i) >> pXX;
            // }
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 1);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 2);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 3);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 4);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 5);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 6);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 7);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 8);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 9);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 10);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 11);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 12);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 13);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 14);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 15);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 16);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 17);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 18);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 19);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 20);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 21);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 22);
                x_i = x_i >> 1;
            }
            x_i = (x_i * x_i) >> pXX;
            if (x_i >= 2 << pXX) {
                log2_intermediate += 1 << (pXX - 23);
            }
            return log2_intermediate;
        }
    }

    /// @notice
    ///     We can write x as
    ///     x = 2^y = 2^v + 2^-1 + 2^-2 + ...
    ///
    ///     2^x = 2^(2^v + 2^-1 + 2^-2 + ...) = 2^major · 2^(2^-1) · 2^(2^-2) · ...
    ///     2^(2^-i) is precomputed.
    /// @dev for i in range(1, 20-1) surfs over the 63 to 0 bits.
    /// @param x uint256, X64
    /// @return uint256, X64 as 2**(x/2**64)*2**64
    function p2X64(uint256 x) internal pure returns (uint256) {
        // Get major of x
        uint256 major_x = x >> 64;
        require(major_x < 192);

        // 2^(2^(-i)) * 2^64, i = 1..
        // uint72[23] memory TWOTWOMINUSI = [
        // 26087635650665564425, 21936999301089678047, 20116317054877281742, 19263451207323153962, 18850675170876015534, 18647615946650685159, 18546908069882975960, 18496758270674070881, 18471734244850835106, 18459234930309000272, 18452988445124272033, 18449865995240371898, 18448304968436414829, 18447524504564044946, 18447134285009651015, 18446939178327825412, 18446841625760745902, 18446792849670663277, 18446768461673986097 , 18446756267687738522, 18446750170697637486, 18446747122203342655, 18446745597956384162]; // 18446744835832952145,
        // 18446744454771247945, 18446744264240398796, 18446744168974974960, 18446744121342263227,
        // 18446744097525907406, 18446744085617729507, 18446744079663640561, 18446744076686596088,
        // 18446744075198073852, 18446744074453812734, 18446744074081682175, 18446744073895616895,
        // 18446744073802584256, 18446744073756067936, 18446744073732809776, 18446744073721180696,
        // 18446744073715366156, 18446744073712458886, 18446744073711005251, 18446744073710278433,
        // 18446744073709915024, 18446744073709733320, 18446744073709642468, 18446744073709597042,
        // 18446744073709574329, 18446744073709562973, 18446744073709557294, 18446744073709554455,
        // 18446744073709553036, 18446744073709552326, 18446744073709551971, 18446744073709551793,
        // 18446744073709551705, 18446744073709551660, 18446744073709551638, 18446744073709551627,
        // 18446744073709551622, 18446744073709551619, 18446744073709551617, 18446744073709551617 ]

        uint256 intermediate = 2**64;

        unchecked {
            if ((x & (1 << (64 - 1))) > 0)
                intermediate = (intermediate * 26087635650665564425) >> 64;
            if ((x & (1 << (64 - 2))) > 0)
                intermediate = (intermediate * 21936999301089678047) >> 64;
            if ((x & (1 << (64 - 3))) > 0)
                intermediate = (intermediate * 20116317054877281742) >> 64;
            if ((x & (1 << (64 - 4))) > 0)
                intermediate = (intermediate * 19263451207323153962) >> 64;
            if ((x & (1 << (64 - 5))) > 0)
                intermediate = (intermediate * 18850675170876015534) >> 64;
            if ((x & (1 << (64 - 6))) > 0)
                intermediate = (intermediate * 18647615946650685159) >> 64;
            if ((x & (1 << (64 - 7))) > 0)
                intermediate = (intermediate * 18546908069882975960) >> 64;
            if ((x & (1 << (64 - 8))) > 0)
                intermediate = (intermediate * 18496758270674070881) >> 64;
            if ((x & (1 << (64 - 9))) > 0)
                intermediate = (intermediate * 18471734244850835106) >> 64;
            if ((x & (1 << (64 - 10))) > 0)
                intermediate = (intermediate * 18459234930309000272) >> 64;
            if ((x & (1 << (64 - 11))) > 0)
                intermediate = (intermediate * 18452988445124272033) >> 64;
            if ((x & (1 << (64 - 12))) > 0)
                intermediate = (intermediate * 18449865995240371898) >> 64;
            if ((x & (1 << (64 - 13))) > 0)
                intermediate = (intermediate * 18448304968436414829) >> 64;
            if ((x & (1 << (64 - 14))) > 0)
                intermediate = (intermediate * 18447524504564044946) >> 64;
            if ((x & (1 << (64 - 15))) > 0)
                intermediate = (intermediate * 18447134285009651015) >> 64;
            if ((x & (1 << (64 - 16))) > 0)
                intermediate = (intermediate * 18446939178327825412) >> 64;
            if ((x & (1 << (64 - 17))) > 0)
                intermediate = (intermediate * 18446841625760745902) >> 64;
            if ((x & (1 << (64 - 18))) > 0)
                intermediate = (intermediate * 18446792849670663277) >> 64;
            if ((x & (1 << (64 - 19))) > 0)
                intermediate = (intermediate * 18446768461673986097) >> 64;
            if ((x & (1 << (64 - 20))) > 0)
                intermediate = (intermediate * 18446756267687738522) >> 64;
            if ((x & (1 << (64 - 21))) > 0)
                intermediate = (intermediate * 18446750170697637486) >> 64;
            if ((x & (1 << (64 - 22))) > 0)
                intermediate = (intermediate * 18446747122203342655) >> 64;
            if ((x & (1 << (64 - 23))) > 0)
                intermediate = (intermediate * 18446745597956384162) >> 64;
        }

        // The major part is added here to increase the size of the number we can compute.
        return intermediate << major_x;
    }

    /// @notice
    ///     We can write x as
    ///     x = 2^y = 2^v + 2^-1 + 2^-2 + ...
    ///
    ///     2^-x = 2^(-(2^v + 2^-1 + 2^-2 + ...)) = 2^-major · 2^(-2^-1) · 2^(-2^-2) · ...
    ///     2^(-2^-i) is precomputed.
    /// @dev for i in range(1, 20-1) surfs over the 63 to 0 bits.
    /// @param x uint256, X64
    /// @return uint256, X64 as 2**(-x/2**64)*2**64
    function invp2X64(uint256 x) internal pure returns (uint256) {
        // Get major of x
        uint256 major_x = x >> 64;
        require(major_x < 41); // dev: Major larger then fixed points. Reserve a few (64-41=23) bits for accuracy

        uint256 intermediate = ONE;
        if (major_x == 0) {
            // Taylor works really well for x < 1.
            // While I should be able to use the identity:
            // 2^(-x) = 2^(-(x_up + x_down)) = 2^(-x_up - x_down) = 2^(-x_up) * 2^(-x_down)
            // it is not as good as the alternative implementation.
            uint256 xp = x;
            unchecked {
                intermediate -= (12786308645202655659 * xp) >> 64;
                xp = (xp * x) >> 64; // Max value is 41**2 * 2**128 < 2**256-1
                intermediate += (4431396893595737425 * xp) >> 64;
                xp = (xp * x) >> 64; // Max value is 41**3 * 2**128 < 2**256-1
                intermediate -= (1023870087579328453 * xp) >> 64;
                xp = (xp * x) >> 64; // Max value is 41**4 * 2**128 < 2**256-1
                intermediate += (177423166116318950 * xp) >> 64;
                xp = (xp * x) >> 64; // Max value is 41**5 * 2**128 < 2**256-1
                intermediate -= (24596073471909060 * xp) >> 64;
                xp = (xp * x) >> 64; // Max value is 41**6 * 2**128 < 2**256-1
                intermediate += (2841449829983172 * xp) >> 64;
                xp = (xp * x) >> 64; // Max value is 41**7 * 2**128 < 2**256-1
                intermediate -= (281363276907910 * xp) >> 64;
                xp = (xp * x) >> 64; // Max value is 41**8 * 2**128 < 2**256-1
                intermediate += (24378270262729 * xp) >> 64;
                xp = (xp * x) >> 64; // Max value is 41**9 * 2**128 < 2**256-1
                // intermediate -= (1877525477726 * xp) >> 64;
                // xp = (xp*x) >> 64; // Max value is 41**10 * 2**128 < 2**256-1
                // intermediate += (130140149132 * xp) >> 64;
                // xp = (xp*x) >> 64; // Max value is 41**11 * 2**128 < 2**256-1
                // intermediate -= (8200570677 * xp) >> 64;
                // xp = (xp*x) >> 64; // Max value is 41**12 * 2**128 < 2**256-1
                // intermediate += (473683537 * xp) >> 64;
                // xp = (xp*x) >> 64; // Max value is 41**13 * 2**128 < 2**256-1
                // intermediate -= (25256339 * xp) >> 64;
                // xp = (xp*x) >> 64; // Max value is 41**14 * 2**128 < 2**256-1
                // intermediate += (1250455 * xp) >> 64;
                // Ending on +, the number is slightly larger than it is supposed to.
                // Because of how the function is used, this is desired.
            }
            return intermediate;
        }
        // 2^(-2^(-i)) * 2^64, i = 1..
        // uint72[19] memory TWOTWOMINUSI = [13043817825332782212, 15511800964685064948, 16915738899553466670, 17664662643191237676, 18051468387014017850, 18248035989933441396, 18347121020861646923, 18396865112328554661, 18421787711448657617, 18434261669329232139, 18440501815349552981, 18443622680442407997, 18445183311048607332, 18445963675871538003, 18446353870663572145, 18446548971154807802, 18446646522174239825, 18446695297877410579, 18446719685777359790 ] //, 18446731879739425374, 18446737976723480912, 18446741025216264368, 18446742549462845018, 18446743311586182573, 18446743692647863158, 18446743883178706403, 18446743978444128763, 18446744026076840128, 18446744049893195856, 18446744061801373732, 18446744067755462673, 18446744070732507144]

        unchecked {
            if ((x & (1 << (64 - 1))) > 0)
                intermediate = (intermediate * 13043817825332782212) >> 64;
            if ((x & (1 << (64 - 2))) > 0)
                intermediate = (intermediate * 15511800964685064948) >> 64;
            if ((x & (1 << (64 - 3))) > 0)
                intermediate = (intermediate * 16915738899553466670) >> 64;
            if ((x & (1 << (64 - 4))) > 0)
                intermediate = (intermediate * 17664662643191237676) >> 64;
            if ((x & (1 << (64 - 5))) > 0)
                intermediate = (intermediate * 18051468387014017850) >> 64;
            if ((x & (1 << (64 - 6))) > 0)
                intermediate = (intermediate * 18248035989933441396) >> 64;
            if ((x & (1 << (64 - 7))) > 0)
                intermediate = (intermediate * 18347121020861646923) >> 64;
            if ((x & (1 << (64 - 8))) > 0)
                intermediate = (intermediate * 18396865112328554661) >> 64;
            if ((x & (1 << (64 - 9))) > 0)
                intermediate = (intermediate * 18421787711448657617) >> 64;
            if ((x & (1 << (64 - 10))) > 0)
                intermediate = (intermediate * 18434261669329232139) >> 64;
            if ((x & (1 << (64 - 11))) > 0)
                intermediate = (intermediate * 18440501815349552981) >> 64;
            if ((x & (1 << (64 - 12))) > 0)
                intermediate = (intermediate * 18443622680442407997) >> 64;
            if ((x & (1 << (64 - 13))) > 0)
                intermediate = (intermediate * 18445183311048607332) >> 64;
            if ((x & (1 << (64 - 14))) > 0)
                intermediate = (intermediate * 18445963675871538003) >> 64;
            if ((x & (1 << (64 - 15))) > 0)
                intermediate = (intermediate * 18446353870663572145) >> 64;
            if ((x & (1 << (64 - 16))) > 0)
                intermediate = (intermediate * 18446548971154807802) >> 64;
            if ((x & (1 << (64 - 17))) > 0)
                intermediate = (intermediate * 18446646522174239825) >> 64;
            if ((x & (1 << (64 - 18))) > 0)
                intermediate = (intermediate * 18446695297877410579) >> 64;
            if ((x & (1 << (64 - 19))) > 0)
                intermediate = (intermediate * 18446719685777359790) >> 64;
        }

        // The major part is added here to increase the size of the number we can compute.
        return intermediate >> major_x;
    }

    /// @notice Using taylor
    /// @dev The accuracy is the smallest iteration.
    ///      (Uses the lagrange error => max i at 1. Thus the max error is last fixed number)
    /// @param x uint256, X64
    /// @return uint256, X64 as 2**(-x/2**64)*2**64
    function invp2TaylorX64(uint256 x) internal pure returns (uint256) {
        // Get major of x
        uint256 major_x = x >> 64;
        require(major_x < 41); // dev: Major larger then fixed points. Reserve a few (64-41=23) bits for accuracy

        uint256 intermediate = ONE;

        uint256 xp = (x << (256 - 64)) >> (256 - 64);
        unchecked {
            intermediate -= (12786308645202655659 * xp) >> 64;
            xp = (xp * x) >> 64; // Max value is 41**2 * 2**128 < 2**256-1
            intermediate += (4431396893595737425 * xp) >> 64;
            xp = (xp * x) >> 64; // Max value is 41**3 * 2**128 < 2**256-1
            intermediate -= (1023870087579328453 * xp) >> 64;
            xp = (xp * x) >> 64; // Max value is 41**4 * 2**128 < 2**256-1
            intermediate += (177423166116318950 * xp) >> 64;
            xp = (xp * x) >> 64; // Max value is 41**5 * 2**128 < 2**256-1
            intermediate -= (24596073471909060 * xp) >> 64;
            xp = (xp * x) >> 64; // Max value is 41**6 * 2**128 < 2**256-1
            intermediate += (2841449829983172 * xp) >> 64;
            xp = (xp * x) >> 64; // Max value is 41**7 * 2**128 < 2**256-1
            intermediate -= (281363276907910 * xp) >> 64;
            xp = (xp * x) >> 64; // Max value is 41**8 * 2**128 < 2**256-1
            intermediate += (24378270262729 * xp) >> 64;
            xp = (xp * x) >> 64; // Max value is 41**9 * 2**128 < 2**256-1
            intermediate -= (1877525477726 * xp) >> 64;
            xp = (xp * x) >> 64; // Max value is 41**10 * 2**128 < 2**256-1
            intermediate += (130140149132 * xp) >> 64;
            // xp = (xp*x) >> 64; // Max value is 41**11 * 2**128 < 2**256-1
            // intermediate -= (8200570677 * xp) >> 64;
            // xp = (xp*x) >> 64; // Max value is 41**12 * 2**128 < 2**256-1
            // intermediate += (473683537 * xp) >> 64;
            // xp = (xp*x) >> 64; // Max value is 41**13 * 2**128 < 2**256-1
            // intermediate -= (25256339 * xp) >> 64;
            // xp = (xp*x) >> 64; // Max value is 41**14 * 2**128 < 2**256-1
            // intermediate += (1250455 * xp) >> 64;
            // Ending on +, the number is slightly larger than it is supposed to.
            // Because of how the function is used, this is desired.
        }

        return intermediate >> major_x;
    }

    /// @notice x^p = 2^(p · log2(x))
    /// @dev Depends heavily on log2 and p2.
    /// @param x uint256, X64 the number raised to the power of p.
    /// @param p uint256 the power which x is raised to.
    /// @return uint256, X64
    function fpowX64(uint256 x, uint256 p) internal pure returns (uint256) {
        return (p2X64(mulX64(p, log2X64(x))));
    }

    /// @notice x^p = 2^(-(p · log2(x)))
    /// @dev Depends heavily on log2 and invp2.
    /// @param x uint256, X64 the number raised to the power of -p.
    /// @param p uint256 the power which x is raised to.
    /// @return uint256, X64
    function invfpowX64(uint256 x, uint256 p) internal pure returns (uint256) {
        return (invp2X64(mulX64(p, log2X64(x))));
    }

    /// @notice
    ///     To calculate (a/b)^p using the identitiy: (a/b)^p = 2^(log2(a/b)*p)
    ///     with log2(a/b) only working for a/b > 1 and 2^x only working for x > 0,
    ///     one can use the trick: a/b < 1 => b > a => b/a > 1.
    ///     Selectivly using fpow and invfpow thus allows one to compute (a/b)^p
    ///     for any a/b.
    ///     The alternative would be wrap 2^x to handle and x and use
    ///     log2(a/b) = log2(a) - log2(b). However, this requires 1 more log2 calculation
    ///     and wrapping 2^x is still needed, since that is a based around lookups.
    /// @dev Explain to a developer any extra details
    /// @param a uint256, X64 Factor 1
    /// @param b uint256, X64 Factor 2
    /// @param p uint256, X64 power.
    /// @return uint256, X64 as (a/b)^p
    function safe_fpowX64(
        uint256 a,
        uint256 b,
        uint256 p
    ) internal pure returns (uint256) {
        if (a < b) {
            return invfpowX64(bigdiv64(b, a), p);
        }
        return fpowX64(bigdiv64(a, b), p);
    }
}
