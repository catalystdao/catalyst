//SPDX-License-Identifier: Unlicensed

pragma solidity ^0.8.16;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/draft-ERC20Permit.sol";
import "./ICatalystV1Pool.sol";

/**
 * @title Catalyst: Swap Router
 * @author Catalyst Labs
 */
abstract contract CatalystSwapRouter {
    using SafeERC20 for IERC20;

    address immutable WRAPPED_GAS_TOKEN;

    constructor(address wrappedGas) {
        WRAPPED_GAS_TOKEN = wrappedGas;
    }

    modifier approveThroughPermit(address fromAsset, uint256 amount, uint256 deadline, uint8 v, bytes32 r, bytes32 s) {
        ERC20Permit(fromAsset).permit(msg.sender, address(this), amount, deadline, v, r, s);
        _;
    }

    function localExactInput(
        address pool,
        address fromAsset,
        address toAsset,
        uint256 amount,
        uint256 minOut
    ) public {
        // Transfer tokens from the user to this contract.
        IERC20(fromAsset).safeTransferFrom(msg.sender, address(this), amount);

        // approve the swap pool for the amount to be sold
        IERC20(fromAsset).approve(pool, amount);
        
        // Execute the Swap
        uint256 swapOutput = ICatalystV1Pool(pool).localswap(fromAsset, toAsset, amount, minOut);

        // Transfer the tokens to the user
        IERC20(toAsset).transfer(msg.sender, swapOutput);
    }

    function localExactInputPermit(
        address pool,
        address fromAsset,
        address toAsset,
        uint256 amount,
        uint256 minOut,
        uint256 deadline,
        uint8 v,
        bytes32 r,
        bytes32 s
    ) approveThroughPermit(fromAsset, amount, deadline, v, r, s) external {
        // approval is set through modifyer. Do the swap
        localExactInput(pool, fromAsset, toAsset, amount, minOut);
    }
}
