import { Keypair, PublicKey } from "@solana/web3.js";
import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import * as chai from 'chai';
import chaiAsPromised from 'chai-as-promised';
import {Buffer} from 'buffer';

import { PolymeraseEmulator } from "../../target/types/polymerase_emulator";
import { PolymeraseEmulatorTester } from "../../target/types/polymerase_emulator_tester";
import { assert } from "chai";

chai.use(chaiAsPromised);

// Hardcoded solana chain id
const SOLANA_CHAIN_ID = new anchor.BN(99);

describe("Polymerase Emulator", () => {

    // Set default commitment to 'confirmed'
    const provider = anchor.AnchorProvider.local(undefined, { commitment: 'confirmed', preflightCommitment: 'confirmed' });
    anchor.setProvider(provider);

    const polymeraseEmulator       = anchor.workspace.PolymeraseEmulator as Program<PolymeraseEmulator>;
    const polymeraseEmulatorTester = anchor.workspace.PolymeraseEmulatorTester as Program<PolymeraseEmulatorTester>;

    const setupMasterKeypair         = Keypair.generate();

    const emulatorStateKeypair       = Keypair.generate();
    const emulatorTesterStateKeypair = Keypair.generate();

    const originSignerKeypair        = Keypair.generate();

    before(async () => {
        // Fund accounts with SOL
        const providerAccountBalance = await provider.connection.getBalance(provider.wallet.publicKey)
        if (providerAccountBalance < 10000000000) {
            await provider.connection.requestAirdrop(provider.wallet.publicKey, 10000000000);
        }

        const tx = await provider.connection.requestAirdrop(setupMasterKeypair.publicKey, 10000000000);
        await provider.connection.confirmTransaction(tx, 'confirmed');
    })


    it("Can initialize the emulator", async () => {
        await polymeraseEmulator.methods.initialize().accounts({
            payer: setupMasterKeypair.publicKey,
            emulatorStateAccount: emulatorStateKeypair.publicKey
        }).signers([
            setupMasterKeypair,
            emulatorStateKeypair
        ]).rpc();
    });


    it("Can initialize the emulator tester", async() => {
        await polymeraseEmulatorTester.methods.createPolymeraseConnection(
            polymeraseEmulator.programId,
            emulatorStateKeypair.publicKey
        ).accounts({
            payer: setupMasterKeypair.publicKey,
            polymeraseConnectionAccount: emulatorTesterStateKeypair.publicKey
        }).signers([
            setupMasterKeypair,
            emulatorTesterStateKeypair
        ]).rpc();
    });


    it("Can emulate receive + send", async() => {

        const data           = new anchor.BN(8);
        const target_program = polymeraseEmulatorTester.programId;

        let emulatorInstructionIndex = 0;
        let next_index_seed = Buffer.alloc(8);
        next_index_seed.writeBigUInt64LE(BigInt(emulatorInstructionIndex));

        const polymeraseInstructionAccount = await PublicKey.findProgramAddress(
            [
                emulatorStateKeypair.publicKey.toBytes(),
                next_index_seed
            ],
            polymeraseEmulator.programId
        ).then(([pubkey]) => pubkey)

        // Start listening for Cross Chain event. Max timeout 1s
        const crossChainTxEventPromise = listenToEvent(polymeraseEmulator, 'CrossChainTxEvent', 0);

        // Perform multichain call (inside polymeraseEmulatorTester)
        const send_tx = await polymeraseEmulatorTester.methods.sendData(
            data,
            SOLANA_CHAIN_ID,
            target_program
        ).accounts({
            polymeraseConnectionAccount: emulatorTesterStateKeypair.publicKey,
            polymeraseEndpointState: emulatorStateKeypair.publicKey,
            polymeraseInstructionAccountPayer: setupMasterKeypair.publicKey,
            polymeraseInstructionAccount,
            polymeraseEndpointProgram: polymeraseEmulator.programId,
            polymeraseInstructionSigner: originSignerKeypair.publicKey
        }).signers([
            setupMasterKeypair,
            originSignerKeypair
        ]).rpc();

        // Wait for Tx event
        const crossChainTxEvent = await crossChainTxEventPromise;
        assert(SOLANA_CHAIN_ID.eq(crossChainTxEvent.targetChain),              'CrossChainTxEvent target_chain mismatch.');
        assert(target_program.equals(crossChainTxEvent.targetProgram),         'CrossChainTxEvent target_program mismatch.');
        assert(originSignerKeypair.publicKey.equals(crossChainTxEvent.sender), 'CrossChainTxEvent sender mismatch.');
        // Payload checked at the receiving end

        // console.log(await provider.connection.getTransaction(send_tx));

        //TODO! BUG After listening to crossChainTxEventPromise, crossChainRxEventPromise cannot be catched.
        //! Caused by removeEventListener
        // // Start listening for Rx Cross Chain event. Max timeout 1s
        // const crossChainRxEventPromise = listenToEvent(polymeraseEmulator, 'CrossChainRxEvent', 1000);
    
        // Start listening for the 'MessageReceived' event of the polymeraseEmulatorTester
        const eventPromise = listenToEvent(polymeraseEmulatorTester, 'MessageReceived', 1000);

        // Execute cross chain transaction
        
        const polymeraseAuthority = await PublicKey.findProgramAddress(
            [emulatorStateKeypair.publicKey.toBytes()],
            polymeraseEmulator.programId
        ).then(([pda]) => pda);

        const receive_tx = await polymeraseEmulator.methods.execute(
            new anchor.BN(emulatorInstructionIndex)
        ).accounts({
            instructionAccount: polymeraseInstructionAccount,
            emulatorStateAccount: emulatorStateKeypair.publicKey,
            targetProgram: polymeraseEmulatorTester.programId,
            rentReceiver: setupMasterKeypair.publicKey,
            polymeraseAuthority
        }).remainingAccounts([
            // Here go any accounts required by the target program
            {
                pubkey: emulatorTesterStateKeypair.publicKey,
                isSigner: false,
                isWritable: false
            }
        ]).signers([
        ]).rpc();

        // console.log(await provider.connection.getTransaction(receive_tx));

        // // Wait for Rx event
        // const crossChainRxEvent = await crossChainRxEventPromise;
        // assert(SOLANA_CHAIN_ID.eq(crossChainRxEvent.sourceChain),              'CrossChainTxEvent target_chain mismatch.');
        // assert(target_program.equals(crossChainRxEvent.targetProgram),         'CrossChainTxEvent target_program mismatch.');
        // assert(originSignerKeypair.publicKey.equals(crossChainRxEvent.sender), 'CrossChainTxEvent sender mismatch.');
        // // Payload checked at the receiving end
    
        // Check the received data matches the sent data
        const event = await eventPromise;
        // console.log(event);
        assert(event.data.eq(data));
        
    });

});

function listenToEvent<IDL extends anchor.Idl, T = any>(program: Program<IDL>, eventName: string, timeout = 0): Promise<T> {
    return new Promise<T>((resolve, reject) => {
        let timeoutId: any;
        let listener = program.addEventListener(eventName, (event, slot) => {
            program.removeEventListener(listener);
            clearTimeout(timeoutId);
            resolve(event);
        });
        if (timeout != 0) {
            timeoutId = setTimeout(() => {
                program.removeEventListener(listener);
                reject(eventName + ' Timeout');
            }, timeout);
        }
    })}
