//SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.17;

import { ICatalystV1Structs } from "./ICatalystV1VaultState.sol";

interface ICatalystV1VaultPermissionless {
    /** 
     * @notice Setup a vault.
     * @param name_ Name of the Vault token.
     * @param symbol_ Symbol for the Vault token.
     * @param chainInterface The cross chain interface used for cross-chain swaps. (Can be address(0) to disable cross-chain swaps.)
     * @param vaultFee The vault fee.
     * @param governanceFee The governance fee share.
     * @param feeAdministrator The account that can modify the fees.
     * @param setupMaster The short-term owner of the vault (until finishSetup is called).
     */
    function setup(
        string calldata name_,
        string calldata symbol_,
        address chainInterface,
        uint256 vaultFee,
        uint256 governanceFee,
        address feeAdministrator,
        address setupMaster
    ) external;

    //--- Balance Changes ---//

    /**
     * @notice Deposits a user configurable amount of tokens.
     * @dev Requires approvals for all deposited tokens within the vault.
     * Volatile: It is advised that the deposit matches the vault's %token distribution.
     * Amplified: It is advised that the deposit is as close to 1,1,... as possible.
     *            Otherwise between 1,1,... and the vault's %token distribution.
     * @param tokenAmounts An array of the tokens amounts to be deposited.
     * @param minOut The minimum number of vault tokens to be minted.
     */
    function depositMixed(uint256[] calldata tokenAmounts, uint256 minOut)
        external returns(uint256);

    /**
     * @notice Burns baseAmount and releases the symmetrical share
     * of tokens to the burner. This doesn't change the vault price.
     * @param baseAmount The number of vault tokens to burn.
     */
    function withdrawAll(uint256 baseAmount, uint256[] calldata minOut)
        external returns(uint256[] memory);

    /**
     * @notice Burns vaultTokens and release a token distribution which can be set by the user.
     * @dev Requires approvals for all tokens within the vault.
     * Volatile: It is advised that the deposit matches the vault's %token distribution.
     * Amplified: It is advised that the deposit matches the vault's %token distribution.
     *            Otherwise it should be weighted towards the tokens the vault has more of.
     * @param vaultTokens The number of vault tokens to withdraw.
     * @param withdrawRatio The percentage of units used to withdraw. In the following special scheme: U_0 = U · withdrawRatio[0], U_1 = (U - U_0) · withdrawRatio[1], U_2 = (U - U_0 - U_1) · withdrawRatio[2], .... Is WAD.
     * @param minOut The minimum number of tokens minted.
     */
    function withdrawMixed(
        uint256 vaultTokens,
        uint256[] calldata withdrawRatio,
        uint256[] calldata minOut
    ) external returns(uint256[] memory);

    //--- Swaps ---//

    /**
     * @notice A swap between 2 assets which both are inside the vault. Is atomic.
     * @param fromAsset The asset the user wants to sell.
     * @param toAsset The asset the user wants to buy.
     * @param amount The amount of fromAsset the user wants to sell.
     * @param minOut The minimum output of _toAsset the user wants.
     */
    function localSwap(
        address fromAsset,
        address toAsset,
        uint256 amount,
        uint256 minOut
    ) external returns (uint256);

    /**
     * @notice Initiate a cross-chain swap by purchasing units and transfer them to another vault.
     * @dev Addresses are encoded in 64 + 1 bytes. To encode for EVM, encode as:
     * Solidity: abi.encodePacket(uint8(20), bytes32(0), abi.encode(<vaultAddress>))
     * @param routeDescription A cross-chain route description which contains the chainIdentifier, toAccount, toVault and relaying incentive.
     * @param fromAsset The asset the user wants to sell.
     * @param toAssetIndex The index of the asset the user wants to buy in the target vault.
     * @param amount The number of fromAsset to sell to the vault.
     * @param minOut The minimum number of returned tokens to the toAccount on the target chain.
     * @param fallbackUser If the transaction fails, send the escrowed funds to this address.
     * @param underwriteIncentiveX16 The payment for underwriting the swap (out of type(uint16.max))
     * @param calldata_ Data field if a call should be made on the target chain.
     * Encoding depends on the target chain, with evm being: abi.encodePacket(bytes20(<address>), <data>)
     * @return uint256 The number of units minted.
     */
    function sendAsset(
        ICatalystV1Structs.RouteDescription calldata routeDescription,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 amount,
        uint256 minOut,
        address fallbackUser,
        uint16 underwriteIncentiveX16,
        bytes calldata calldata_
    ) external payable returns (uint256);

    /**
     * @notice Initiate a cross-chain swap by purchasing units and transfer them to another vault using a fixed number of units.
     * @dev Addresses are encoded in 64 + 1 bytes. To encode for EVM, encode as:
     * Solidity: abi.encodePacket(uint8(20), bytes32(0), abi.encode(<vaultAddress>))
     * @param routeDescription A cross-chain route description which contains the chainIdentifier, toAccount, toVault and relaying incentive.
     * @param fromAsset The asset the user wants to sell.
     * @param toAssetIndex The index of the asset the user wants to buy in the target vault.
     * @param amount The number of fromAsset to sell to the vault.
     * @param minOut The minimum number of returned tokens to the toAccount on the target chain.
     * @param minU The minimum and exact number of units sent.
     * @param fallbackUser If the transaction fails, send the escrowed funds to this address.
     * @param calldata_ Data field if a call should be made on the target chain.
     * Encoding depends on the target chain, with evm being: abi.encodePacket(bytes20(<address>), <data>)
     * @param underwriteIncentiveX16 The payment for underwriting the swap (out of type(uint16.max))
     * @return uint256 The number of units minted.
     */
    function sendAssetFixedUnit(
        ICatalystV1Structs.RouteDescription calldata routeDescription,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 amount,
        uint256 minOut,
        uint256 minU,
        address fallbackUser,
        uint16 underwriteIncentiveX16,
        bytes calldata calldata_
    ) external payable returns (uint256);

    /**
     * @notice Completes a cross-chain swap by converting units to the desired token (toAsset)
     * @dev Can only be called by the chainInterface.
     * @param channelId The incoming connection identifier.
     * @param fromVault The source vault.
     * @param toAssetIndex Index of the asset to be purchased with Units.
     * @param toAccount The recipient.
     * @param U Number of units to convert into toAsset.
     * @param minOut Minimum number of tokens bought. Reverts if less.
     * @param fromAmount Used to match cross-chain swap events. The input amount on the sending chain.
     * @param fromAsset Used to match cross-chain swap events. The input asset on the sending chain.
     * @param blockNumberMod Used to match cross-chain swap events. The block number from the host side.
     */
    function receiveAsset(
        bytes32 channelId,
        bytes calldata fromVault,
        uint256 toAssetIndex,
        address toAccount,
        uint256 U,
        uint256 minOut,
        uint256 fromAmount,
        bytes calldata fromAsset,
        uint32 blockNumberMod
    ) external;

    function receiveAsset(
        bytes32 channelId,
        bytes calldata fromVault,
        uint256 toAssetIndex,
        address toAccount,
        uint256 U,
        uint256 minOut,
        uint256 fromAmount,
        bytes calldata fromAsset,
        uint32 blockNumberMod,
        address dataTarget,
        bytes calldata data
    ) external;

    /**
     * @notice Initiate a cross-chain liquidity swap by withdrawing tokens and converting them to units.
     * @dev While the description says tokens are withdrawn and then converted to units, vault tokens are converted
     * directly into units through the following equation:
     *      U = ln(PT/(PT-pt)) * \sum W_i
     * @param routeDescription A cross-chain route description which contains the chainIdentifier, toAccount, toVault and relaying incentive.
     * @param vaultTokens The number of vault tokens to exchange.
     * @param minOut An array of minout describing: [the minimum number of vault tokens, the minimum number of reference assets].
     * @param fallbackUser If the transaction fails, send the escrowed funds to this address.
     * @param calldata_ Data field if a call should be made on the target chain.
     * Encoding depends on the target chain, with evm being: abi.encodePacket(bytes20(<address>), <data>)
     * @return uint256 The number of units minted.
     */
    function sendLiquidity(
        ICatalystV1Structs.RouteDescription calldata routeDescription,
        uint256 vaultTokens,
        uint256[2] calldata minOut,
        address fallbackUser,
        bytes calldata calldata_
    ) external payable returns (uint256);

    /**
     * @notice Completes a cross-chain liquidity swap by converting units to tokens and depositing.
     * @dev Called exclusively by the chainInterface.
     * @param fromVault The source vault
     * @param toAccount The recipient of the vault tokens
     * @param U Number of units to convert into vault tokens.
     * @param minVaultTokens The minimum number of vault tokens to mint on target vault. Otherwise: Reject
     * @param minReferenceAsset The minimum number of reference asset the vaults tokens are worth. Otherwise: Reject
     * @param fromAmount Used to match cross-chain swap events. The input amount on the sending chain.
     * @param blockNumberMod Used to match cross-chain swap events. The block number from the host side.
     */
    function receiveLiquidity(
        bytes32 channelId,
        bytes calldata fromVault,
        address toAccount,
        uint256 U,
        uint256 minVaultTokens,
        uint256 minReferenceAsset,
        uint256 fromAmount,
        uint32 blockNumberMod
    ) external;

    function receiveLiquidity(
        bytes32 channelId,
        bytes calldata fromVault,
        address toAccount,
        uint256 U,
        uint256 minVaultTokens,
        uint256 minReferenceAsset,
        uint256 fromAmount,
        uint32 blockNumberMod,
        address dataTarget,
        bytes calldata data
    ) external;
}
