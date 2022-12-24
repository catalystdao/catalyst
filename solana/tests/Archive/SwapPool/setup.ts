import * as anchor from "@project-serum/anchor"; // ! Required for 'Buffer'

// Global variables
export const SOLANA_CHAIN_ID = new anchor.BN(99); //TODO! Hardcode Solana chain id to 99
export const SOLANA_CHAIN_ID_BUFFER = Buffer.alloc(8);
SOLANA_CHAIN_ID_BUFFER.writeBigUInt64LE(BigInt(SOLANA_CHAIN_ID.toNumber()));