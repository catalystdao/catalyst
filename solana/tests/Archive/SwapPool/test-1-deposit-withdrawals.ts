import * as chai from 'chai';
import { assert } from "chai";
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import { Keypair, PublicKey } from "@solana/web3.js";
import * as anchor from "@project-serum/anchor";
import { BN, Program } from "@project-serum/anchor";

import { approve, createAccount, createMint, createTokenAccounts, getAccountInfo } from "./utils/token-utils";
import { createAndSetupSwapPool, getSwapPoolAssetWalletsAuthority, getSwapPoolTokenMintsAuthority } from "./utils/deploy-utils";

import { SwapPool } from "../../../target/types/swap_pool";
import { TOKEN_PROGRAM_ID, u64 } from "@solana/spl-token";
import { CrossChainSwapInterface } from "../../../target/types/cross_chain_swap_interface";
import { PolymeraseEmulator } from "../../../target/types/polymerase_emulator";

describe("Test deposit and withdrawals", () => {

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
    const tokenMintKeypairs  = [tokenAMintKeypair, tokenBMintKeypair, tokenCMintKeypair];
    const tokenMintAmounts   = [
        new BN(10).mul(new BN(10).pow(new BN(8))),     // 10*10**8
        new BN(100).mul(new BN(10).pow(new BN(8))),    // 100*10**8
        new BN(1000).mul(new BN(10).pow(new BN(6))),    // 1000*10**6
    ];

    const userAKeypair       = Keypair.generate();
    const userBKeypair       = Keypair.generate();

    let userATokenAAccount: PublicKey | undefined;
    let userATokenBAccount: PublicKey | undefined;
    let userATokenCAccount: PublicKey | undefined;
    let userBTokenAAccount: PublicKey | undefined;
    let userBTokenBAccount: PublicKey | undefined;
    let userBTokenCAccount: PublicKey | undefined;

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
    
    })

    beforeEach(async () => {
        // ! Regenerate the swapPoolStateKeypair on every test to make sure the tests are isolated from each other
        swapPoolStateKeypair     = Keypair.generate();

        swapPoolWalletsAuthority = await getSwapPoolAssetWalletsAuthority(swapPoolProgram.programId, swapPoolStateKeypair.publicKey);
        swapPoolMintsAuthority   = await getSwapPoolTokenMintsAuthority(swapPoolProgram.programId, swapPoolStateKeypair.publicKey);

        // Create token wallets for userA and userB, for each of the 3 mints. Fund userA wallets.
        // Currently these get recreated for every test to make sure the base asset balance for each account is always the same for each test
        [userATokenAAccount, userBTokenAAccount] = await createTokenAccounts(
            provider,
            tokenAMintKeypair.publicKey,
            tokenMintAuthority,
            [userAKeypair.publicKey, userBKeypair.publicKey],
            [tokenMintAmounts[0], new BN(0)]
        );

        [userATokenBAccount, userBTokenBAccount] = await createTokenAccounts(
            provider,
            tokenBMintKeypair.publicKey,
            tokenMintAuthority,
            [userAKeypair.publicKey, userBKeypair.publicKey],
            [tokenMintAmounts[1], new BN(0)]
        );

        [userATokenCAccount, userBTokenCAccount] = await createTokenAccounts(
            provider,
            tokenCMintKeypair.publicKey,
            tokenMintAuthority,
            [userAKeypair.publicKey, userBKeypair.publicKey],
            [tokenMintAmounts[2], new BN(0)]
        );
    })

    it("Allows deposits", async () => {
        // Deposit tokens (3 different mints) and verify the returned pool tokens amount

        const assetMints = [tokenAMintKeypair.publicKey, tokenBMintKeypair.publicKey, tokenCMintKeypair.publicKey];

        const depositorKeypair      = userAKeypair;
        const depositorAssetWallets = [userATokenAAccount, userATokenBAccount, userATokenCAccount];

        // Create the SwapPool
        const createAndSetupSwapPoolResult = await createAndSetupSwapPool(
            swapPoolProgram,
            setupMasterKeypair,
            crossChainSwapInterfaceProgram,
            interfaceSetupMasterKeypair,
            polymeraseEndpointProgram,
            setupMasterKeypair,
            assetMints,
            swapPoolStateKeypair
        );
        const swapPoolAssetWalletAuthority = createAndSetupSwapPoolResult.addAssetToSwapPoolResults[0].swapPoolAssetWalletAuthority;

        // Deposit assets (run the 3 deposit instructions at the same time)
        await Promise.all(assetMints.map(async (assetMint, i) => {

            // Create pool token account for the depositor
            const depositorPoolTokenWallet = await createAccount(
                provider,
                createAndSetupSwapPoolResult.addAssetToSwapPoolResults[i].swapPoolTokenMint,
                depositorKeypair.publicKey
            );

            // Delegate tokens to the SwapPool token authority
            await approve(
                provider,
                depositorAssetWallets[i],
                depositorKeypair,
                swapPoolAssetWalletAuthority,
                tokenMintAmounts[i]
            );

            // Deposit
            const tx = await swapPoolProgram.methods.deposit(tokenMintAmounts[i]).accounts({
                swapPoolStateAccount: swapPoolStateKeypair.publicKey,
                depositedAssetMint: assetMint,
                depositorAssetWallet: depositorAssetWallets[i],
                depositorPoolTokenWallet: depositorPoolTokenWallet,
                swapPoolAssetWallet: createAndSetupSwapPoolResult.addAssetToSwapPoolResults[i].swapPoolAssetWallet,
                swapPoolAssetWalletAuthority: createAndSetupSwapPoolResult.addAssetToSwapPoolResults[i].swapPoolAssetWalletAuthority, 
                swapPoolTokenMint: createAndSetupSwapPoolResult.addAssetToSwapPoolResults[i].swapPoolTokenMint,
                swapPoolTokenMintAuthority: createAndSetupSwapPoolResult.addAssetToSwapPoolResults[i].swapPoolTokenMintAuthority,
                tokenProgram: TOKEN_PROGRAM_ID
            }).rpc();

            // Verify asset and pool tokens balance
            const depositorWalletInfo = await getAccountInfo(
                provider,
                assetMint,
                depositorAssetWallets[i]
            );
            assert(depositorWalletInfo.amount.eq(new BN(0)), "Not all tokens have been transferred to the SwapPool.");

            const depositorPoolTokenWalletInfo = await getAccountInfo(
                provider,
                createAndSetupSwapPoolResult.addAssetToSwapPoolResults[i].swapPoolTokenMint,
                depositorPoolTokenWallet
            )
            assert(depositorPoolTokenWalletInfo.amount.eq(tokenMintAmounts[i]), "Unexpected pool token amount transferred to the depositor.");
        }));

    });

    it("Allow deposits and withdrawals", async () => {
        // ! THIS FUNCTION CURRENTLY ONLY CHECKS THAT THE DEPOSIT AND WITHDRAWAL INSTRUCTIONS WORK,
        // ! IT DOES NOT TEST FOR THEIR CORRECTNESS, AS THESE STILL HAVE TO BE FINALISED

        // Create the SwapPool
        const assetMints = [tokenAMintKeypair.publicKey, tokenBMintKeypair.publicKey];
        const createAndSetupSwapPoolResult = await createAndSetupSwapPool(
            swapPoolProgram,
            setupMasterKeypair,
            crossChainSwapInterfaceProgram,
            interfaceSetupMasterKeypair,
            polymeraseEndpointProgram,
            setupMasterKeypair,
            assetMints,
            swapPoolStateKeypair
        );

        const depositedAssetAmount = 100;
        const depositedAssetMint   = tokenAMintKeypair.publicKey;
        const depositorKeypair     = userAKeypair;
        const depositorAssetWallet = userATokenAAccount;

        // Create a pool token wallet for the depositor
        const depositorPoolTokenWallet = await createAccount(
            provider,
            createAndSetupSwapPoolResult.addAssetToSwapPoolResults[0].swapPoolTokenMint,
            depositorKeypair.publicKey
        );

        // Delegate tokens to the SwapPool token authority before the deposit instruction call
        await approve(
            provider,
            userATokenAAccount,
            userAKeypair,
            createAndSetupSwapPoolResult.addAssetToSwapPoolResults[0].swapPoolAssetWalletAuthority,
            depositedAssetAmount
        );

        // Verify the depositor asset wallet state
        const userAAssetWalletAccountInfo = await getAccountInfo(
            provider,
            tokenAMintKeypair.publicKey,
            userATokenAAccount
        );
        assert(userAAssetWalletAccountInfo.amount >= new anchor.BN(depositedAssetAmount));
        assert(userAAssetWalletAccountInfo.delegate.equals(
            createAndSetupSwapPoolResult.addAssetToSwapPoolResults[0].swapPoolAssetWalletAuthority
        ));
        assert(userAAssetWalletAccountInfo.delegatedAmount.eq(new anchor.BN(depositedAssetAmount)));

        // Deposit assets
        const tx = await swapPoolProgram.methods.deposit(new anchor.BN(depositedAssetAmount)).accounts({
            swapPoolStateAccount: swapPoolStateKeypair.publicKey,
            depositedAssetMint,
            depositorAssetWallet,
            depositorPoolTokenWallet,
            swapPoolAssetWallet: createAndSetupSwapPoolResult.addAssetToSwapPoolResults[0].swapPoolAssetWallet,
            swapPoolAssetWalletAuthority: createAndSetupSwapPoolResult.addAssetToSwapPoolResults[0].swapPoolAssetWalletAuthority, 
            swapPoolTokenMint: createAndSetupSwapPoolResult.addAssetToSwapPoolResults[0].swapPoolTokenMint,
            swapPoolTokenMintAuthority: createAndSetupSwapPoolResult.addAssetToSwapPoolResults[0].swapPoolTokenMintAuthority,
            tokenProgram: TOKEN_PROGRAM_ID
        }).signers([
        ]).rpc();

        
        // const userAAssetWalletAccountInfo2 = await getAccountInfo(
        //     provider,
        //     tokenAMintKeypair.publicKey,
        //     userATokenAAccount
        // );

        // const swapPoolAssetWalletAccountInfo = await getAccountInfo(
        //     provider,
        //     tokenAMintKeypair.publicKey,
        //     createAndSetupSwapPoolResult.addAssetToSwapPoolResults[0].swapPoolAssetWallet
        // );

        // Withdraw assets
        const tx2 = await swapPoolProgram.methods.withdraw(new anchor.BN(1)).accounts({
            swapPoolStateAccount: swapPoolStateKeypair.publicKey,
            withdrawnAssetMint: depositedAssetMint,
            withdrawerAssetWallet: depositorAssetWallet,
            withdrawerPoolTokenWallet: depositorPoolTokenWallet,
            withdrawerPoolTokenWalletAuthority: depositorKeypair.publicKey,
            swapPoolAssetWallet: createAndSetupSwapPoolResult.addAssetToSwapPoolResults[0].swapPoolAssetWallet,
            swapPoolAssetWalletAuthority: createAndSetupSwapPoolResult.addAssetToSwapPoolResults[0].swapPoolAssetWalletAuthority, 
            swapPoolTokenMint: createAndSetupSwapPoolResult.addAssetToSwapPoolResults[0].swapPoolTokenMint,
            tokenProgram: TOKEN_PROGRAM_ID
        }).signers([
            depositorKeypair
        ]).rpc();


    });

});
