//SPDX-License-Identifier: Unlicensed

pragma solidity ^0.8.16;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/draft-ERC20Permit.sol";
import "./ICatalystV1Pool.sol";
import "./interfaces/IOnCatalyst.sol";

struct SwapRoute {
    address[] pools;
    address[] assets;
}

struct CrossChainSwapContext {
    bytes32 channelId;
    bytes32 targetPool;
    bytes32 targetUser;
    uint8 toAssetIndex;
    uint256 minOut;
    address fallbackUser;
    bytes calldata_;
}

struct Permit {
    uint256 amount;
    uint256 deadline;
    uint8 v;
    bytes32 r;
    bytes32 s;
}

/**
 * @title Catalyst: Swap Router
 * @author Catalyst Labs
 */
contract CatalystSwapRouter {
    using SafeERC20 for IERC20;

    address immutable WRAPPED_GAS_TOKEN;

    constructor(address wrappedGas) {
        WRAPPED_GAS_TOKEN = wrappedGas;
    }

    /// @dev Adds erc20 permit to functions, enabling 1 tx workflow.
    function approveThroughPermit(address fromAsset, Permit calldata permit) internal {
        ERC20Permit(fromAsset).permit(msg.sender, address(this), permit.amount, permit.deadline, permit.v, permit.r, permit.s);
    }

    function localExactInput(
        address pool,
        address fromAsset,
        address toAsset,
        uint256 amount,
        uint256 minOut
    ) public returns (uint256) {
        // Transfer tokens from the user to this contract.
        IERC20(fromAsset).safeTransferFrom(msg.sender, address(this), amount);

        // approve the swap pool for the amount to be sold
        IERC20(fromAsset).approve(pool, amount);
        
        // Execute the Swap
        uint256 swapOutput = ICatalystV1Pool(pool).localswap(fromAsset, toAsset, amount, minOut);

        // Transfer the tokens to the user
        IERC20(toAsset).transfer(msg.sender, swapOutput);
        
        return swapOutput;
    }

    function localExactInputPermit(
        address pool,
        address fromAsset,
        address toAsset,
        uint256 amount,
        uint256 minOut,
        Permit calldata permit
    ) external returns (uint256) {
        // approve through permit
        approveThroughPermit(fromAsset, permit);
        // approval is set through modifyer. Do the swap
        return localExactInput(pool, fromAsset, toAsset, amount, minOut);
    }

    function localExactInputRoute(
        SwapRoute calldata route,
        uint256 inputAmount,
        uint256 minOut
    ) public returns (uint256) {
        // Transfer tokens from the user to this contract.
        IERC20(route.assets[0]).safeTransferFrom(msg.sender, address(this), inputAmount);

        uint256 lastTokenIndex = route.assets.length - 1;
        // require(lastTokenIndex == pools.length);  // Not needed since the swap will revert if that is the case.
        uint256 workAmount = inputAmount;
        for (uint256 it = 0; it < lastTokenIndex; ++it) {
            // approve the swap pool for the amount to be sold
            IERC20(route.assets[it]).approve(route.pools[it], workAmount);

            // Execute the Swap
            workAmount = ICatalystV1Pool(route.pools[it]).localswap(route.assets[it], route.assets[it + 1], workAmount, 0);
        }
        // Check the output is more than the minimum.
        require(minOut <= workAmount);

        // Transfer the tokens to the user
        IERC20(route.assets[lastTokenIndex]).transfer(msg.sender, workAmount);

        return workAmount;
    }

    function localExactInputRoutePermit(
        SwapRoute calldata route,
        uint256 inputAmount,
        uint256 minOut,
        Permit calldata permit
    ) external returns (uint256) {
        // approve through permit
        address originAsset = route.assets[0];
        approveThroughPermit(originAsset, permit);
        // Do the swap
        return localExactInputRoute(route, inputAmount, minOut);
        
    }

    function crossExactInputRoute(
        SwapRoute calldata route,
        uint256 inputAmount,
        uint256 localMinOut,
        CrossChainSwapContext calldata swapContext
    ) public {
        address crossChainSwapPool;
        address finalAsset;
        uint256 swapAmount;
        {
            uint256 crossChainSwapPoolIndex = route.pools.length;
            address[] calldata routePools = route.pools[0: crossChainSwapPoolIndex - 2];
            SwapRoute memory localRoute = SwapRoute(routePools, route.assets);
            swapAmount = this.localExactInputRoute(localRoute, inputAmount, localMinOut);
            crossChainSwapPool = route.pools[crossChainSwapPoolIndex - 1];
            finalAsset = route.assets[crossChainSwapPoolIndex];
        }

        ICatalystV1Pool(crossChainSwapPool).sendSwap(
            swapContext.channelId,
            swapContext.targetPool,
            swapContext.targetUser,
            finalAsset,
            swapContext.toAssetIndex,
            swapAmount,
            swapContext.minOut,
            swapContext.fallbackUser,
            swapContext.calldata_
        );
    }

    function crossExactInputRoutePermit(
        SwapRoute calldata route,
        uint256 inputAmount,
        uint256 localMinOut,
        CrossChainSwapContext calldata swapContext,
        Permit calldata permit
    ) external {
        // approve through permit
        address originAsset = route.assets[0];
        approveThroughPermit(originAsset, permit);
        // Do the swap
        crossExactInputRoute(
            route,
            inputAmount,
            localMinOut,
            swapContext
        );
    }
}
