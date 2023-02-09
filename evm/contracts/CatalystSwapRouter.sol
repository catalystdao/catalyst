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
contract CatalystSwapRouter is ICatalystReceiver {
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
        IERC20(toAsset).safeTransfer(msg.sender, swapOutput);
        
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

    function _swapViaRoute(
        SwapRoute memory route,
        uint256 inputAmount,
        uint256 minOut
    ) internal returns (uint256 swapReturn, address outputToken) {
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

        // Ensure the return values are set.
        swapReturn = workAmount;
        outputToken = route.assets[lastTokenIndex];
    }

    function localExactInputRoutePrivate(
        SwapRoute calldata route,
        uint256 inputAmount,
        uint256 minOut
    ) public returns (uint256 swapReturn, address outputToken) {
        require(msg.sender == address(this));
        (swapReturn, outputToken) = _swapViaRoute(route, inputAmount, minOut);
    }

    function localExactInputRoute(
        SwapRoute calldata route,
        uint256 inputAmount,
        uint256 minOut
    ) public returns (uint256) {
        // Transfer tokens from the user to this contract.
        IERC20(route.assets[0]).safeTransferFrom(msg.sender, address(this), inputAmount);

        (uint256 swapReturn, address outputToken) = _swapViaRoute(route, inputAmount, minOut);

        // Transfer the tokens to the user
        IERC20(outputToken).safeTransfer(msg.sender, swapReturn);

        return swapReturn;
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
        CrossChainSwapContext calldata swapContext,
        bytes memory calldata_
    ) public {
        // Transfer tokens from the user to this contract.
        IERC20(route.assets[0]).safeTransferFrom(msg.sender, address(this), inputAmount);
        
        address crossChainSwapPool;
        address finalAsset;
        uint256 swapAmount;
        {
            uint256 crossChainSwapPoolIndex = route.pools.length;
            address[] calldata routePools = route.pools[0: crossChainSwapPoolIndex - 2];

            // Define the swap route here.
            SwapRoute memory localRoute = SwapRoute(routePools, route.assets);

            // Execute the swap route
            (swapAmount, finalAsset) = _swapViaRoute(localRoute, inputAmount, localMinOut);

            crossChainSwapPool = route.pools[crossChainSwapPoolIndex - 1];
        }

        // approve the swap pool for the amount to be sold
        IERC20(finalAsset).approve(crossChainSwapPool, swapAmount);

        ICatalystV1Pool(crossChainSwapPool).sendSwap(
            swapContext.channelId,
            swapContext.targetPool,
            swapContext.targetUser,
            finalAsset,
            swapContext.toAssetIndex,
            swapAmount,
            swapContext.minOut,
            swapContext.fallbackUser,
            calldata_
        );
    }

    function crossExactInputRoutePermit(
        SwapRoute calldata route,
        uint256 inputAmount,
        uint256 localMinOut,
        CrossChainSwapContext calldata swapContext,
        Permit calldata permit,
        bytes memory calldata_
    ) external {
        // approve through permit
        address originAsset = route.assets[0];
        approveThroughPermit(originAsset, permit);
        // Do the swap
        crossExactInputRoute(
            route,
            inputAmount,
            localMinOut,
            swapContext,
            calldata_
        );
    }

    function _decodeSwapData(bytes calldata data) pure internal returns(bool allowRevert, uint256 minOut, address targetUser, SwapRoute memory route) {
       (allowRevert, minOut, targetUser, route) = abi.decode(data, (bool, uint256, address, SwapRoute));
    }

    function _encodeSwapData(bool allowRevert, uint256 minOut, address targetUser, SwapRoute calldata route) pure internal returns(bytes memory) {
        return abi.encode(
            allowRevert,
            minOut,
            targetUser,
            route
        );
    }

    function onCatalystCall(uint256 purchasedTokens, bytes calldata data) external {
        (bool allowRevert, uint256 minOut, address targetUser, SwapRoute memory route) = _decodeSwapData(data);

        address firstAsset = route.assets[0];

        // Approve 
        IERC20(firstAsset).approve(address(this), purchasedTokens);

        // use the localswap route here.
        try this.localExactInputRoutePrivate(route, purchasedTokens, minOut) returns (uint256 swapReturn, address outputToken) {
            IERC20(outputToken).safeTransfer(targetUser, swapReturn);
        } catch (bytes memory err) {
            if (!allowRevert) revert(string(err));
            IERC20(firstAsset).safeTransfer(targetUser, purchasedTokens);
        }
    }

    function crossExactInputRouteRoute(
        SwapRoute calldata localRoute,
        SwapRoute calldata remoteRoute,
        uint256 inputAmount,
        uint256 localMinOut,
        uint256 remoteMinOut,
        uint256 targetUser,
        CrossChainSwapContext calldata swapContext,
        bool allowRevert
    ) external {
        // approve through permit
        address originAsset = localRoute.assets[0];
        // Do the swap
        bytes memory calldata_ = abi.encode(
            allowRevert,
            remoteMinOut,
            targetUser,
            remoteRoute
        );
        crossExactInputRoute(
            localRoute,
            inputAmount,
            localMinOut,
            swapContext,
            calldata_
        );
    }
}
