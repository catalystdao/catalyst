import * as chai from 'chai';
import { assert, expect } from "chai";
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import { Keypair, PublicKey } from "@solana/web3.js";
import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";

import { approve, createMint, createTokenAccounts, getAccountInfo } from "./utils/token-utils";
import {
    createAndSetupSwapPool,
    addLiquidityToSwapPool,
    getSwapPoolAssetWalletsAuthority,
    getSwapPoolTokenMintsAuthority
} from "./utils/deploy-utils";

import { SwapPool } from "../../../target/types/swap_pool";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { CrossChainSwapInterface } from "../../../target/types/cross_chain_swap_interface";
import { PolymeraseEmulator } from "../../../target/types/polymerase_emulator";
import { SOLANA_CHAIN_ID, SOLANA_CHAIN_ID_BUFFER } from "./setup";

describe("Test swaps", () => {

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

    it("Local Swap", async () => {
        // ! THIS FUNCTION CURRENTLY ONLY CHECKS THAT THE LOCAL SWAP INSTRUCTION WORKS,
        // ! IT DOES NOT TEST FOR ITS CORRECTNESS

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

        // Define parameters
        const swapperKeypair           = userAKeypair;
        const swapperInputAssetBalance = 500;
        const swapperInputAssetWallet  = userATokenAAccount;
        const swapperOutputAssetWallet = userATokenBAccount;
        
        const inputAssetMint           = tokenAMintKeypair.publicKey;
        const outputAssetMint          = tokenBMintKeypair.publicKey;

        const swapPoolInputAssetWallet     = createAndSetupSwapPoolResult.addAssetToSwapPoolResults[0].swapPoolAssetWallet;
        const swapPoolOutputAssetWallet    = createAndSetupSwapPoolResult.addAssetToSwapPoolResults[1].swapPoolAssetWallet
        const swapPoolAssetWalletAuthority = createAndSetupSwapPoolResult.addAssetToSwapPoolResults[0].swapPoolAssetWalletAuthority;

        // Add liquidity to the pool for swap to be possible
        const depositedAssetAmount = 10000;

        await addLiquidityToSwapPool(
            swapPoolProgram,
            depositedAssetAmount,
            inputAssetMint,
            userBTokenAAccount,
            userBKeypair,
            swapPoolInputAssetWallet,
            swapPoolAssetWalletAuthority,
            createAndSetupSwapPoolResult.addAssetToSwapPoolResults[0].swapPoolTokenMint,
            createAndSetupSwapPoolResult.addAssetToSwapPoolResults[0].swapPoolTokenMintAuthority,
            swapPoolStateKeypair.publicKey
        );

        await addLiquidityToSwapPool(
            swapPoolProgram,
            depositedAssetAmount,
            outputAssetMint,
            userBTokenBAccount,
            userBKeypair,
            swapPoolOutputAssetWallet,
            swapPoolAssetWalletAuthority,
            createAndSetupSwapPoolResult.addAssetToSwapPoolResults[1].swapPoolTokenMint,
            createAndSetupSwapPoolResult.addAssetToSwapPoolResults[1].swapPoolTokenMintAuthority,
            swapPoolStateKeypair.publicKey
        );


        // Swap A tokens for B tokens

        // Delegate tokens to the SwapPool token authority before the deposit instruction call
        await approve(
            provider,
            swapperInputAssetWallet,
            swapperKeypair,
            swapPoolAssetWalletAuthority,
            swapperInputAssetBalance
        );

        // Verify the depositor asset wallet state
        const swapperInputAssetWalletAccountInfo = await getAccountInfo(
            provider,
            inputAssetMint,
            swapperInputAssetWallet
        );

        assert(swapperInputAssetWalletAccountInfo.amount.toNumber() >= swapperInputAssetBalance);
        assert(swapperInputAssetWalletAccountInfo.delegate.equals(swapPoolAssetWalletAuthority));
        assert(swapperInputAssetWalletAccountInfo.delegatedAmount.eq(new anchor.BN(swapperInputAssetBalance)));

        // Note the current state of the swapper output wallet
        const swapperAssetWalletAccountInfo = await getAccountInfo(
            provider,
            outputAssetMint,
            swapperOutputAssetWallet
        );

        // Swap
        const tx = await swapPoolProgram.methods.localSwap(new anchor.BN(swapperInputAssetBalance), new anchor.BN(1)).accounts({
            swapPoolStateAccount: swapPoolStateKeypair.publicKey,
            inputAssetMint,
            inputAssetWallet: swapperInputAssetWallet,
            swapPoolInputAssetWallet,
            outputAssetMint: tokenBMintKeypair.publicKey,
            outputAssetWallet: swapperOutputAssetWallet,
            swapPoolOutputAssetWallet,
            swapPoolAssetWalletAuthority,
            tokenProgram: TOKEN_PROGRAM_ID
        }).signers([
        ]).rpc();

        // Check that some B tokens are received
        const swapperAssetWalletAccountInfoAfterSwap = await getAccountInfo(
            provider,
            outputAssetMint,
            swapperOutputAssetWallet
        );

        assert(swapperAssetWalletAccountInfoAfterSwap.amount > swapperAssetWalletAccountInfo.amount);

    });



    it("Crude cross chain swap", async () => {
        // ! THIS FUNCTION CURRENTLY ONLY CHECKS THAT A CROSS CHAIN SWAP WORKS,
        // ! IT DOES NOT TEST FOR ITS CORRECTNESS


        // Create the SwapPool
        const assetMints = [tokenAMintKeypair.publicKey];
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

        // Define parameters
        const swapperKeypair           = userAKeypair;
        const swapperInputAssetBalance = 500;
        const swapperInputAssetWallet  = userATokenAAccount;
        
        const inputAssetMint           = tokenAMintKeypair.publicKey;

        const swapPoolInputAssetWallet     = createAndSetupSwapPoolResult.addAssetToSwapPoolResults[0].swapPoolAssetWallet;
        const swapPoolAssetWalletAuthority = createAndSetupSwapPoolResult.addAssetToSwapPoolResults[0].swapPoolAssetWalletAuthority;

        // Add liquidity to the pool for swap to be possible
        const depositedAssetAmount = 10000;

        await addLiquidityToSwapPool(
            swapPoolProgram,
            depositedAssetAmount,
            inputAssetMint,
            userBTokenAAccount,
            userBKeypair,
            swapPoolInputAssetWallet,
            swapPoolAssetWalletAuthority,
            createAndSetupSwapPoolResult.addAssetToSwapPoolResults[0].swapPoolTokenMint,
            createAndSetupSwapPoolResult.addAssetToSwapPoolResults[0].swapPoolTokenMintAuthority,
            swapPoolStateKeypair.publicKey
        );


        // Create pool connection
        const target_program = swapPoolProgram.programId; // The interface of the target pool
        const target_pool_id = createAndSetupSwapPoolResult.createInterfaceResult.interfaceStateAccountPDA;
        
        const connectionStateAccount = await PublicKey.findProgramAddress(
            [
                createAndSetupSwapPoolResult.createInterfaceResult.interfaceStateAccountPDA.toBytes(),
                SOLANA_CHAIN_ID_BUFFER,
                target_pool_id.toBytes()
            ],
            crossChainSwapInterfaceProgram.programId
        ).then(([pubkey]) => pubkey);

        const createConnectionTx = await crossChainSwapInterfaceProgram.methods.createConnection(
            SOLANA_CHAIN_ID,
            target_pool_id,
            crossChainSwapInterfaceProgram.programId
        ).accounts({
            configurator: interfaceSetupMasterKeypair.publicKey,
            interfaceStateAccount: createAndSetupSwapPoolResult.createInterfaceResult.interfaceStateAccountPDA,
            connectionStateAccount
        }).signers([
            interfaceSetupMasterKeypair
        ]).rpc();


        // Swap A tokens

        // Delegate tokens to the SwapPool token authority before the deposit instruction call
        await approve(
            provider,
            swapperInputAssetWallet,
            swapperKeypair,
            swapPoolAssetWalletAuthority,
            swapperInputAssetBalance
        );

        // Verify the depositor asset wallet state
        let swapperInputAssetWalletAccountInfo = await getAccountInfo(
            provider,
            inputAssetMint,
            swapperInputAssetWallet
        );

        assert(swapperInputAssetWalletAccountInfo.amount.toNumber() >= swapperInputAssetBalance);
        assert(swapperInputAssetWalletAccountInfo.delegate.equals(swapPoolAssetWalletAuthority));
        assert(swapperInputAssetWalletAccountInfo.delegatedAmount.eq(new anchor.BN(swapperInputAssetBalance)));

        const swapperInputAssetInitialBalance = swapperInputAssetWalletAccountInfo.amount

        // Outswap => CrossChainSwap => CallMultichain

        let emulatorInstructionIndex = 0;
        let next_index_seed = Buffer.alloc(8);
        next_index_seed.writeBigUInt64LE(BigInt(emulatorInstructionIndex));

        const polymeraseInstructionAccount = await PublicKey.findProgramAddress(
            [
                createAndSetupSwapPoolResult.createPolymeraseEndpointResult.polymeraseEndpointStateAccountKeypair.publicKey.toBytes(),
                next_index_seed
            ],
            polymeraseEndpointProgram.programId
        ).then(([pubkey]) => pubkey)


        const out_tx = await swapPoolProgram.methods.outSwap(
            SOLANA_CHAIN_ID,
            target_pool_id,
            0,
            swapperInputAssetWallet,
            new anchor.BN(swapperInputAssetBalance)
        ).accounts({
            swapPoolStateAccount: swapPoolStateKeypair.publicKey,
            inputAssetMint,
            inputAssetWallet: swapperInputAssetWallet,
            swapPoolInputAssetWallet,
            swapPoolAssetWalletAuthority,
            tokenProgram: TOKEN_PROGRAM_ID,
            crossChainSwapInterfaceProgram: crossChainSwapInterfaceProgram.programId,
            swapPoolAuthority: createAndSetupSwapPoolResult.createInterfaceResult.swapPoolSwapAuthority,
            interfaceStateAccount: createAndSetupSwapPoolResult.createInterfaceResult.interfaceStateAccountPDA,
            connectionStateAccount,
            polymeraseEndpointState: createAndSetupSwapPoolResult.createPolymeraseEndpointResult.polymeraseEndpointStateAccountKeypair.publicKey,
            polymeraseInstructionAccountPayer: setupMasterKeypair.publicKey,
            polymeraseInstructionAccount: polymeraseInstructionAccount,
            polymeraseEndpointProgram: polymeraseEndpointProgram.programId
        }).signers([
            setupMasterKeypair
        ]).rpc();


        // Verify the depositor asset wallet state
        swapperInputAssetWalletAccountInfo = await getAccountInfo(
            provider,
            inputAssetMint,
            swapperInputAssetWallet
        );

        assert(swapperInputAssetWalletAccountInfo.amount.eq(swapperInputAssetInitialBalance.sub(new anchor.BN(swapperInputAssetBalance)))); // The swapper input asset balance must have decreased by swapperInputAssetBalance
        assert(swapperInputAssetWalletAccountInfo.delegatedAmount.eq(new anchor.BN(0)));


        const emulatorStateAccount = createAndSetupSwapPoolResult.createPolymeraseEndpointResult.polymeraseEndpointStateAccountKeypair.publicKey;
        const polymeraseAuthority = await PublicKey.findProgramAddress(
            [emulatorStateAccount.toBytes()],
            polymeraseEndpointProgram.programId
        ).then(([pda]) => pda);

        // Execute cross chain transaction
        // Execute => Receive => InSwap
        const receive_tx = await polymeraseEndpointProgram.methods.execute(
            new anchor.BN(emulatorInstructionIndex)
        ).accounts({
            instructionAccount: polymeraseInstructionAccount,
            emulatorStateAccount,
            targetProgram: crossChainSwapInterfaceProgram.programId,
            rentReceiver: setupMasterKeypair.publicKey,
            polymeraseAuthority
        }).remainingAccounts([
            // Here go any accounts required by the target program
            {   // interface_state_account
                pubkey: createAndSetupSwapPoolResult.createInterfaceResult.interfaceStateAccountPDA,
                isSigner: false,
                isWritable: false
            },
            {   // swap_pool
                pubkey: swapPoolStateKeypair.publicKey,
                isSigner: false,
                isWritable: true
            },
            {   // connection_state_account
                pubkey: connectionStateAccount,
                isSigner: false,
                isWritable: false
            },
            {   // swap_pool_program
                pubkey: swapPoolProgram.programId,
                isSigner: false,
                isWritable: false
            },
            {   // output_asset_mint
                pubkey: inputAssetMint,
                isSigner: false,
                isWritable: false
            },
            {   // output_asset_wallet
                pubkey: swapperInputAssetWallet,
                isSigner: false,
                isWritable: true
            },
            {   // swap_pool_output_asset_wallet
                pubkey: swapPoolInputAssetWallet,
                isSigner: false,
                isWritable: true
            },
            {   // swap_pool_asset_wallet_authority
                pubkey: swapPoolAssetWalletAuthority,
                isSigner: false,
                isWritable: false
            },
            {
                pubkey: TOKEN_PROGRAM_ID,
                isSigner: false,
                isWritable: false
            }
        ]).signers([
        ]).rpc();


        // Verify the depositor asset wallet state
        swapperInputAssetWalletAccountInfo = await getAccountInfo(
            provider,
            inputAssetMint,
            swapperInputAssetWallet
        );

        assert(swapperInputAssetInitialBalance.sub(swapperInputAssetWalletAccountInfo.amount).lte(new anchor.BN(1))); // The current balance must match the initial balance (allow 1 unit of error)

    });


});
