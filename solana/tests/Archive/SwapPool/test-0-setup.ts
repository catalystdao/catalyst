import * as chai from 'chai';
import { expect } from "chai";
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import { Keypair, PublicKey } from "@solana/web3.js";
import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";

import { createMint, createTokenAccounts } from "./utils/token-utils";
import {
    addAssetToSwapPool,
    createSwapPool,
    finishSwapPoolSetup,
    getSwapPoolTokenMintsAuthority,
    getSwapPoolAssetWalletsAuthority,
    verifySwapPoolAsset,
    createCrossChainSwapInterface,
    createPolymeraseEndpoint
} from "./utils/deploy-utils";

import { SwapPool } from "../../../target/types/swap_pool";
import { CrossChainSwapInterface } from "../../../target/types/cross_chain_swap_interface";
import { PolymeraseEmulator } from "../../../target/types/polymerase_emulator";

describe("Test pool setup", async () => {

    // Set default commitment to 'confirmed'
    const provider = anchor.AnchorProvider.local(undefined, { commitment: 'confirmed', preflightCommitment: 'confirmed' });
    anchor.setProvider(provider);

    const swapPoolProgram                = anchor.workspace.SwapPool as Program<SwapPool>;
    const crossChainSwapInterfaceProgram = anchor.workspace.CrossChainSwapInterface as Program<CrossChainSwapInterface>;
    const polymeraseEndpointProgram      = anchor.workspace.PolymeraseEmulator as Program<PolymeraseEmulator>;

    const tokenMintAuthority = Keypair.generate();
    const tokenAMintKeypair  = Keypair.generate();
    const tokenBMintKeypair  = Keypair.generate();
    const tokenCMintKeypair  = Keypair.generate();
    const tokenDMintKeypair  = Keypair.generate();
    const tokenMintKeypairs  = [tokenAMintKeypair, tokenBMintKeypair, tokenCMintKeypair, tokenDMintKeypair];

    const userAKeypair       = Keypair.generate();
    const userBKeypair       = Keypair.generate();

    let userATokenAAccount: PublicKey | undefined;
    let userATokenBAccount: PublicKey | undefined;
    let userBTokenAAccount: PublicKey | undefined;
    let userBTokenBAccount: PublicKey | undefined;

    const setupMasterKeypair = Keypair.generate();
    const setupHackerKeypair = Keypair.generate();

    const interfaceSetupMasterKeypair = Keypair.generate();

    let swapPoolStateKeypair     : Keypair;
    let swapPoolWalletsAuthority : PublicKey;
    let swapPoolMintsAuthority   : PublicKey;


    before(async () => {
        // Fund accounts with SOL
        const providerAccountBalance = await provider.connection.getBalance(provider.wallet.publicKey)
        if (providerAccountBalance < 10000000000) {
            await provider.connection.requestAirdrop(provider.wallet.publicKey, 10000000000);
        }

        await provider.connection.requestAirdrop(setupMasterKeypair.publicKey, 10000000000);
        await provider.connection.requestAirdrop(setupHackerKeypair.publicKey, 10000000000);
        await provider.connection.requestAirdrop(interfaceSetupMasterKeypair.publicKey, 10000000000);

        // Create token mints
        for (let tokenMint of tokenMintKeypairs) {
            await createMint(
                provider,
                tokenMint,
                tokenMintAuthority.publicKey,
                null,
                0
            );
        }

        // Create token wallets
        [userATokenAAccount, userBTokenAAccount] = await createTokenAccounts(
            provider,
            tokenAMintKeypair.publicKey,
            tokenMintAuthority,
            [userAKeypair.publicKey, userBKeypair.publicKey],
            1000000
        );

        [userATokenBAccount, userBTokenBAccount] = await createTokenAccounts(
            provider,
            tokenBMintKeypair.publicKey,
            tokenMintAuthority,
            [userAKeypair.publicKey, userBKeypair.publicKey],
            1000000
        );

    })

    beforeEach(async () => {
        // ! Regenerate the swapPoolStateKeypair on every test to make sure the tests are isolated from each other
        swapPoolStateKeypair     = Keypair.generate();

        swapPoolWalletsAuthority = await getSwapPoolAssetWalletsAuthority(swapPoolProgram.programId, swapPoolStateKeypair.publicKey);
        swapPoolMintsAuthority   = await getSwapPoolTokenMintsAuthority(swapPoolProgram.programId, swapPoolStateKeypair.publicKey);
    })

    it("Can create polymerase emulator", async () => {
        await createPolymeraseEndpoint(
            polymeraseEndpointProgram,
            setupMasterKeypair,
        );
    });

    it("Can create a cross chain swap interface", async () => {
        const createPolymeraseEndpointResult = await createPolymeraseEndpoint(
            polymeraseEndpointProgram,
            setupMasterKeypair,
        );

        await createCrossChainSwapInterface(
            crossChainSwapInterfaceProgram,
            swapPoolProgram.programId,
            swapPoolStateKeypair.publicKey,
            polymeraseEndpointProgram.programId,
            createPolymeraseEndpointResult.polymeraseEndpointStateAccountKeypair.publicKey,
            interfaceSetupMasterKeypair
        );
    });

    // TODO! Make interface link compulsory
    it("Cannot create a swap pool with no assets", async () => {

        await createSwapPool(swapPoolProgram, setupMasterKeypair, swapPoolStateKeypair);

        // Make sure the setup cannot be completed
        expect(
            finishSwapPoolSetup(swapPoolProgram, setupMasterKeypair, swapPoolStateKeypair.publicKey)
        ).to.eventually.be.rejected;

    });


    it("Can create swap pool with 1 asset", async () => {

        const asset_mint = tokenAMintKeypair.publicKey;

        await createSwapPool(swapPoolProgram, setupMasterKeypair, swapPoolStateKeypair);

        await addAssetToSwapPool(
            swapPoolProgram,
            setupMasterKeypair,
            swapPoolStateKeypair.publicKey,
            asset_mint,
            undefined,
            swapPoolWalletsAuthority
        );

        await finishSwapPoolSetup(swapPoolProgram, setupMasterKeypair, swapPoolStateKeypair.publicKey);

        //TODO! check for pool tokens

        // Verify swap pool assets
        await verifySwapPoolAsset(
            swapPoolProgram,
            provider,
            swapPoolStateKeypair.publicKey,
            asset_mint
        )

    });


    it("Can create swap pool with max 3 assets", async () => {

        const asset_mints = [tokenAMintKeypair.publicKey, tokenBMintKeypair.publicKey, tokenCMintKeypair.publicKey];

        await createSwapPool(swapPoolProgram, setupMasterKeypair, swapPoolStateKeypair);

        for (let asset_mint of asset_mints) {
            await addAssetToSwapPool(
                swapPoolProgram,
                setupMasterKeypair,
                swapPoolStateKeypair.publicKey,
                asset_mint,
                undefined,
                swapPoolWalletsAuthority
            );

        }

        // Try to add an extra asset
        await expect(
            addAssetToSwapPool(
                swapPoolProgram,
                setupMasterKeypair,
                swapPoolStateKeypair.publicKey,
                tokenDMintKeypair.publicKey,
                undefined,
                swapPoolWalletsAuthority
            )
        ).to.eventually.rejected;

        await finishSwapPoolSetup(swapPoolProgram, setupMasterKeypair, swapPoolStateKeypair.publicKey);

        // Verify swap pool assets
        for (let asset_mint of asset_mints) {
            await verifySwapPoolAsset(
                swapPoolProgram,
                provider,
                swapPoolStateKeypair.publicKey,
                asset_mint
            )
        }

    });

    it("Does not allow a third party to add an asset to the pool nor finish the setup", async () => {

        const asset_mints = [tokenAMintKeypair.publicKey, tokenBMintKeypair.publicKey];

        await createSwapPool(swapPoolProgram, setupMasterKeypair, swapPoolStateKeypair);

        // Add two assets using setupMaster
        for (let asset_mint of asset_mints) {
            await addAssetToSwapPool(
                swapPoolProgram,
                setupMasterKeypair,
                swapPoolStateKeypair.publicKey,
                asset_mint,
                undefined,
                swapPoolWalletsAuthority
            );
        }

        // Try to add a third asset using a different setupMaster
        await expect(
            addAssetToSwapPool(
                swapPoolProgram,
                setupHackerKeypair,
                swapPoolStateKeypair.publicKey,
                tokenCMintKeypair.publicKey,
                undefined,
                swapPoolWalletsAuthority
            )
        ).to.eventually.be.rejected;

        // Try to finish the setup with a different setupMaster
        await expect(
            finishSwapPoolSetup(swapPoolProgram, setupHackerKeypair, swapPoolStateKeypair.publicKey)
        ).to.eventually.be.rejected;

        // Make sure the pool setup can be finished
        finishSwapPoolSetup(swapPoolProgram, setupMasterKeypair, swapPoolStateKeypair.publicKey)
    });


    it("Does not allow setup changes after finishing the setup", async () => {

        const asset_mints = [tokenAMintKeypair.publicKey, tokenBMintKeypair.publicKey];

        await createSwapPool(swapPoolProgram, setupMasterKeypair, swapPoolStateKeypair);

        for (let asset_mint of asset_mints) {
            await addAssetToSwapPool(
                swapPoolProgram,
                setupMasterKeypair,
                swapPoolStateKeypair.publicKey,
                asset_mint,
                undefined,
                swapPoolWalletsAuthority
            );

        }

        await finishSwapPoolSetup(swapPoolProgram, setupMasterKeypair, swapPoolStateKeypair.publicKey);


        // Try to add a third asset
        await expect(
            addAssetToSwapPool(
                swapPoolProgram,
                setupMasterKeypair,
                swapPoolStateKeypair.publicKey,
                tokenCMintKeypair.publicKey,
                undefined,
                swapPoolWalletsAuthority
            )
        ).to.eventually.be.rejected;

        // Try to finish the setup
        await expect(
            finishSwapPoolSetup(swapPoolProgram, setupMasterKeypair, swapPoolStateKeypair.publicKey)
        ).to.eventually.be.rejected;

    });

});
