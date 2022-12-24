import * as anchor from "@project-serum/anchor";
import { Program, Provider } from "@project-serum/anchor";
import { Keypair, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { assert } from "chai";
import { approve, createAccount, verifyTokenMint, verifyTokenWallet } from "./token-utils";

import { SwapPool } from "../../../../target/types/swap_pool";
import { CrossChainSwapInterface } from "../../../../target/types/cross_chain_swap_interface";
import { PolymeraseEmulator } from "../../../../target/types/polymerase_emulator";

const POOL_ASSET_WALLET_SEED            : string = "poolAsset";
const POOL_ASSET_WALLETS_AUTHORITY_SEED : string = "poolAssetAuth";
const POOL_TOKEN_MINT_SEED              : string = "poolMint";
const POOL_TOKEN_MINTS_AUTHORITY_SEED   : string = "poolMintsAuth";
const POOL_SWAP_AUTHORITY               : string = "poolSwapAuth";
const INTERFACE_SWAP_AUTHORITY          : string = "intSwapAuth";

export async function createSwapPool(
    program: Program<SwapPool>,
    setupMasterKeypair: Keypair,
    swapPoolStateAccountKeypair?: Keypair
): Promise<CreateSwapPoolResult> {

    swapPoolStateAccountKeypair = swapPoolStateAccountKeypair ?? Keypair.generate();
    
    const tx = await program.methods.createSwapPool(new anchor.BN(1)).accounts({ //TODO set amplification as parameter
        setupMaster: setupMasterKeypair.publicKey,
        swapPoolStateAccount: swapPoolStateAccountKeypair.publicKey,
        systemProgram: SystemProgram.programId
    }).signers([
        swapPoolStateAccountKeypair,
        setupMasterKeypair
    ]).rpc();

    return {tx, swapPoolStateAccountKeypair};
}

export async function createPolymeraseEndpoint(
    program: Program<PolymeraseEmulator>,
    payerKeypair: Keypair,
    polymeraseEndpointStateAccountKeypair?: Keypair
): Promise<CreatePolymeraseEndpointResult> {

    polymeraseEndpointStateAccountKeypair = polymeraseEndpointStateAccountKeypair ?? Keypair.generate();
    
    const tx = await program.methods.initialize().accounts({
        payer: payerKeypair.publicKey,
        emulatorStateAccount: polymeraseEndpointStateAccountKeypair.publicKey,
        systemProgram: SystemProgram.programId
    }).signers([
        polymeraseEndpointStateAccountKeypair,
        payerKeypair
    ]).rpc();

    return {tx, polymeraseEndpointStateAccountKeypair};
}

export async function createCrossChainSwapInterface(
    program: Program<CrossChainSwapInterface>,
    swapPoolProgramId: PublicKey,
    swapPoolStateAccountPubkey: PublicKey,
    polymeraseEndpointProgramId: PublicKey,
    polymeraseEndpointStateAccountPubkey: PublicKey,
    setupMasterKeypair: Keypair,
    swapPoolSwapAuthority?: PublicKey
): Promise<CreateCrossChainSwapInterfaceResult> {

    swapPoolSwapAuthority = swapPoolSwapAuthority ?? await getSwapPoolSwapAuthority(
        swapPoolProgramId,
        swapPoolStateAccountPubkey
    );

    let interfaceStateAccountPDA = await PublicKey.findProgramAddress(
        [swapPoolStateAccountPubkey.toBytes()],
        program.programId
    ).then(([pubkey]) => pubkey);
    
    const tx = await program.methods.initialize(
        swapPoolStateAccountPubkey,
        swapPoolSwapAuthority,
        polymeraseEndpointProgramId,
        polymeraseEndpointStateAccountPubkey
    ).accounts({ //TODO set amplification as parameter
        configurator: setupMasterKeypair.publicKey,
        interfaceStateAccount: interfaceStateAccountPDA,
        systemProgram: SystemProgram.programId
    }).signers([
        setupMasterKeypair
    ]).rpc();

    return {tx, swapPoolSwapAuthority, interfaceStateAccountPDA};
}

export async function addAssetToSwapPool(
    program: Program<SwapPool>,
    setupMasterKeypair: Keypair,
    swapPoolStateAccountPubkey: PublicKey,
    assetMint: PublicKey,
    swapPoolAssetWallet?: PublicKey,
    swapPoolAssetWalletAuthority?: PublicKey,
    swapPoolTokenMint?: PublicKey,
    swapPoolTokenMintAuthority?: PublicKey
): Promise<AddAssetToSwapPoolResult> {

    swapPoolAssetWallet = swapPoolAssetWallet ?? await getSwapPoolAssetWallet(
        program.programId,
        swapPoolStateAccountPubkey,
        assetMint
    );

    swapPoolAssetWalletAuthority = swapPoolAssetWalletAuthority ?? await getSwapPoolAssetWalletsAuthority(
        program.programId,
        swapPoolStateAccountPubkey
    )

    swapPoolTokenMint = swapPoolTokenMint ?? await getSwapPoolTokenMint(
        program.programId,
        swapPoolStateAccountPubkey,
        assetMint
    );

    swapPoolTokenMintAuthority = swapPoolTokenMintAuthority ?? await getSwapPoolTokenMintsAuthority(
        program.programId,
        swapPoolStateAccountPubkey
    )

    const tx = await program.methods.addSwapPoolAsset().accounts({
        swapPoolStateAccount: swapPoolStateAccountPubkey,
        setupMaster: setupMasterKeypair.publicKey,
        assetMint,
        swapPoolAssetWallet,
        swapPoolAssetWalletAuthority,
        swapPoolTokenMint,
        swapPoolTokenMintAuthority,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY
    }).signers([
        setupMasterKeypair
    ]).rpc();

    return {
        tx,
        swapPoolAssetWallet,
        swapPoolAssetWalletAuthority,
        swapPoolTokenMint,
        swapPoolTokenMintAuthority
    };
}

export async function linkCrossChainSwapInterface(
    swapPoolProgram: Program<SwapPool>,
    swapPoolSetupMasterKeypair: Keypair,
    swapPoolStateAccountPubkey: PublicKey,
    crossChainSwapInterfaceState: PublicKey
): Promise<LinkCCSIToSwapPoolResult> {

    const tx = await swapPoolProgram.methods.linkCrossChainSwapInterface(
        crossChainSwapInterfaceState
    ).accounts({
        swapPoolStateAccount: swapPoolStateAccountPubkey,
        setupMaster: swapPoolSetupMasterKeypair.publicKey
    }).signers([
        swapPoolSetupMasterKeypair
    ]).rpc();

    return {tx};
}

export async function finishSwapPoolSetup(
    program: Program<SwapPool>,
    setupMasterKeypair: Keypair,
    swapPoolStateAccountPubkey: PublicKey
): Promise<FinishSwapPoolSetupResult> {

    const tx = await program.methods.finishSetup().accounts({
        swapPoolStateAccount: swapPoolStateAccountPubkey,
        setupMaster: setupMasterKeypair.publicKey
    }).signers([
        setupMasterKeypair
    ]).rpc();

    return {tx};
}

export async function createAndSetupSwapPool(
    swapPoolProgram: Program<SwapPool>,
    setupMasterKeypair: Keypair,
    crossChainSwapInterfaceProgram: Program<CrossChainSwapInterface>,
    interfaceSetupMasterKeypair: Keypair,
    polymeraseEndpointProgram: Program<PolymeraseEmulator>,
    polymeraseEndpointSetupMasterKeypair: Keypair,
    assetMints: PublicKey[],
    swapPoolStateAccountKeypair?: Keypair
): Promise<CreateAndSetupSwapPoolResult> {

    const createSwapPoolResult = await createSwapPool(swapPoolProgram, setupMasterKeypair, swapPoolStateAccountKeypair);
    const swapPoolStateAccountPublicKey = createSwapPoolResult.swapPoolStateAccountKeypair.publicKey;

    
    const createPolymeraseEndpointResult = await createPolymeraseEndpoint(
        polymeraseEndpointProgram,
        polymeraseEndpointSetupMasterKeypair,
    );

    const createInterfaceResult = await createCrossChainSwapInterface(
        crossChainSwapInterfaceProgram,
        swapPoolProgram.programId,
        swapPoolStateAccountKeypair.publicKey,
        polymeraseEndpointProgram.programId,
        createPolymeraseEndpointResult.polymeraseEndpointStateAccountKeypair.publicKey,
        interfaceSetupMasterKeypair,
        undefined
    );

    const addAssetToSwapPoolResults = await Promise.all(assetMints.map(assetMint => addAssetToSwapPool(
        swapPoolProgram,
        setupMasterKeypair,
        swapPoolStateAccountPublicKey,
        assetMint
    )));

    const linkCCSIToSwapPoolResult = await linkCrossChainSwapInterface(
        swapPoolProgram,
        setupMasterKeypair,
        swapPoolStateAccountPublicKey,
        createInterfaceResult.interfaceStateAccountPDA
    )

    const finishSwapPoolSetupResult = await finishSwapPoolSetup(swapPoolProgram, setupMasterKeypair, swapPoolStateAccountPublicKey);

    return {
        createSwapPoolResult,
        createPolymeraseEndpointResult,
        createInterfaceResult,
        addAssetToSwapPoolResults,
        linkCCSIToSwapPoolResult,
        finishSwapPoolSetupResult
    }
}

export async function addLiquidityToSwapPool(
    program: Program<SwapPool>,
    amount: number,
    depositedAssetMint: PublicKey,
    depositorAssetWallet: PublicKey,
    depositorKeypair: Keypair,
    swapPoolAssetWallet: PublicKey,
    swapPoolAssetWalletAuthority: PublicKey,
    swapPoolTokenMint: PublicKey,
    swapPoolTokenMintAuthority: PublicKey,
    swapPoolStateAccount: PublicKey,
    depositorPoolTokenWallet?: PublicKey
): Promise<AddLiquidityToSwapPoolResult> {
    // Create a pool token wallet for the depositor
    depositorPoolTokenWallet = depositorPoolTokenWallet ?? await createAccount(
        program.provider as anchor.AnchorProvider,
        swapPoolTokenMint,
        depositorKeypair.publicKey
    );

    // Delegate tokens to the SwapPool token authority before the deposit instruction call
    await approve(
        program.provider as anchor.AnchorProvider,
        depositorAssetWallet,
        depositorKeypair,
        swapPoolAssetWalletAuthority,
        amount
    );

    // Deposit assets
    const tx = await program.methods.deposit(new anchor.BN(amount)).accounts({
        swapPoolStateAccount,
        depositedAssetMint,
        depositorAssetWallet,
        depositorPoolTokenWallet,
        swapPoolAssetWallet,
        swapPoolAssetWalletAuthority, 
        swapPoolTokenMint,
        swapPoolTokenMintAuthority,
        tokenProgram: TOKEN_PROGRAM_ID
    }).signers([
    ]).rpc();

    return {
        tx,
        depositorPoolTokenWallet
    }
}

export async function getSwapPoolAssetWalletsAuthority(
    programId: PublicKey,
    swapPoolStateAccountPubkey: PublicKey,
): Promise<PublicKey> {
    return PublicKey.findProgramAddress(
        [
            swapPoolStateAccountPubkey.toBytes(),
            Buffer.from(anchor.utils.bytes.utf8.encode(POOL_ASSET_WALLETS_AUTHORITY_SEED))
        ],
        programId
    ).then(([pubkey]) => pubkey);
}

export async function getSwapPoolTokenMintsAuthority(
    programId: PublicKey,
    swapPoolStateAccountPubkey: PublicKey,
): Promise<PublicKey> {
    return PublicKey.findProgramAddress(
        [
            swapPoolStateAccountPubkey.toBytes(),
            Buffer.from(anchor.utils.bytes.utf8.encode(POOL_TOKEN_MINTS_AUTHORITY_SEED))
        ],
        programId
    ).then(([pubkey]) => pubkey);
}

export async function getSwapPoolSwapAuthority(
    programId: PublicKey,
    swapPoolStateAccountPubkey: PublicKey,
): Promise<PublicKey> {
    return PublicKey.findProgramAddress(
        [
            swapPoolStateAccountPubkey.toBytes(),
            Buffer.from(anchor.utils.bytes.utf8.encode(POOL_SWAP_AUTHORITY))
        ],
        programId
    ).then(([pubkey]) => pubkey);
}

export async function getSwapPoolAssetWallet(
    programId: PublicKey,
    swapPoolStateAccountPubkey: PublicKey,
    assetMint: PublicKey
): Promise<PublicKey> {
    return PublicKey.findProgramAddress(
        [
            swapPoolStateAccountPubkey.toBytes(),
            assetMint.toBytes(),
            Buffer.from(anchor.utils.bytes.utf8.encode(POOL_ASSET_WALLET_SEED))
        ],
        programId
    ).then(([pubkey]) => pubkey);
}

export async function getSwapPoolTokenMint(
    programId: PublicKey,
    swapPoolStateAccountPubkey: PublicKey,
    assetMint: PublicKey
): Promise<PublicKey> {
    return PublicKey.findProgramAddress(
        [
            swapPoolStateAccountPubkey.toBytes(),
            assetMint.toBytes(),
            Buffer.from(anchor.utils.bytes.utf8.encode(POOL_TOKEN_MINT_SEED))
        ],
        programId
    ).then(([pubkey]) => pubkey);
}



// Verification Helpers

export async function verifySwapPoolAsset(
    program: Program<SwapPool>,
    provider: Provider,
    swapPoolStateAccountPubkey: PublicKey,
    expectedAssetMint: PublicKey
): Promise<void> {
    
    const swap_pool_state = await program.account.swapPoolState.fetch(swapPoolStateAccountPubkey, 'recent');

    // Check asset is registered
    const asset_index = swap_pool_state.poolAssetsMints.findIndex(
        savedMint => savedMint.toString() === expectedAssetMint.toString()
    );
    assert(asset_index !== -1, "Token mint asset not found in SwapPool.");

    // Check asset wallet
    const expectedAssetWallet          = await getSwapPoolAssetWallet(program.programId, swapPoolStateAccountPubkey, expectedAssetMint);
    const expectedAssetWalletAuthority = await getSwapPoolAssetWalletsAuthority(program.programId, swapPoolStateAccountPubkey);
    
    verifyTokenWallet(
        provider as anchor.AnchorProvider,
        expectedAssetWallet,
        expectedAssetMint,
        expectedAssetWalletAuthority
    )

    // Check the swap pool token mint corresponding to the added asset
    const expectedTokenMint          = await getSwapPoolTokenMint(program.programId, swapPoolStateAccountPubkey, expectedAssetMint);
    const expectedTokenMintAuthority = await getSwapPoolTokenMintsAuthority(program.programId, swapPoolStateAccountPubkey);

    verifyTokenMint(
        provider as anchor.AnchorProvider,
        expectedTokenMint,
        expectedTokenMintAuthority,
        null
    )
}



// Helper Interfaces

export interface CreateSwapPoolResult {
    tx: string,
    swapPoolStateAccountKeypair: Keypair
}

export interface CreatePolymeraseEndpointResult {
    tx: string,
    polymeraseEndpointStateAccountKeypair: Keypair
}

export interface CreateCrossChainSwapInterfaceResult {
    tx: string,
    swapPoolSwapAuthority: PublicKey
    interfaceStateAccountPDA: PublicKey
}

export interface AddAssetToSwapPoolResult {
    tx: string,
    swapPoolAssetWallet: PublicKey,
    swapPoolAssetWalletAuthority: PublicKey,
    swapPoolTokenMint: PublicKey,
    swapPoolTokenMintAuthority: PublicKey,
}

export interface LinkCCSIToSwapPoolResult {
    tx: string
}

export interface FinishSwapPoolSetupResult {
    tx: string
}

export interface CreateAndSetupSwapPoolResult {
    createSwapPoolResult: CreateSwapPoolResult,
    createPolymeraseEndpointResult: CreatePolymeraseEndpointResult,
    createInterfaceResult: CreateCrossChainSwapInterfaceResult,
    addAssetToSwapPoolResults: AddAssetToSwapPoolResult[],
    linkCCSIToSwapPoolResult: LinkCCSIToSwapPoolResult,
    finishSwapPoolSetupResult: FinishSwapPoolSetupResult,
}

export interface AddLiquidityToSwapPoolResult {
    tx: string,
    depositorPoolTokenWallet: PublicKey
}