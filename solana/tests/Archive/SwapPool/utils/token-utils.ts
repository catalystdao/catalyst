import * as anchor from "@project-serum/anchor";

import {Buffer} from 'buffer';
import { AccountInfo, AccountLayout, MintInfo, MintLayout, Token, TOKEN_PROGRAM_ID, u64 as splu64 } from "@solana/spl-token"
import { Commitment, Keypair, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import { assert } from "chai";


class u64 extends anchor.BN {
  /**
   * Convert to Buffer representation
   */
  toBuffer() {
    const a = super.toArray().reverse();
    const b = Buffer.from(a);

    if (b.length === 8) {
      return b;
    }

    if (b.length >= 8) throw new Error('u64 too large');
    const zeroPad = Buffer.alloc(8);
    b.copy(zeroPad);
    return zeroPad;
  }
  /**
   * Construct a u64 from Buffer representation
   */


  static fromBuffer(buffer: Buffer) {
    if (buffer.length !== 8) throw new Error(`Invalid buffer length: ${buffer.length}`);
    return new u64([...buffer].reverse().map(i => `00${i.toString(16)}`.slice(-2)).join(''), 16);
  }

}

// Functions based on @solana/spl-token Token class (check node_modules > @solana > spl-token > lib > index.browser.esm.js)

export async function createMint(
  provider: anchor.AnchorProvider,
  keypair: anchor.web3.Keypair,
  mintAuthority: PublicKey,
  freezeAuthority: PublicKey | null,
  decimals: number,
  tokenProgramId = TOKEN_PROGRAM_ID,
  confirmOptions?: anchor.web3.ConfirmOptions
): Promise<PublicKey> {
    // ! Function based on createMint() from @solana/spl-token Token class (check node_modules > @solana > spl-token > lib > index.browser.esm.js)

    const balanceNeeded = await Token.getMinBalanceRentForExemptMint(provider.connection);

    const transaction = new Transaction().add(
        SystemProgram.createAccount({
            fromPubkey: provider.wallet.publicKey,
            newAccountPubkey: keypair.publicKey,
            lamports: balanceNeeded,
            space: MintLayout.span,
            programId: tokenProgramId,
        }),
        Token.createInitMintInstruction(
            tokenProgramId,
            keypair.publicKey,
            decimals,
            mintAuthority,
            freezeAuthority
        )
    );

    await provider.sendAndConfirm(transaction, [keypair], confirmOptions);

    return keypair.publicKey;
}

export async function createAccount(
    provider: anchor.AnchorProvider,
    mintPublicKey: PublicKey,
    owner: PublicKey,
    tokenProgramId = TOKEN_PROGRAM_ID,
    confirmOptions?: anchor.web3.ConfirmOptions
): Promise<PublicKey> {
    // ! Function based on createAccount() from @solana/spl-token Token class (check node_modules > @solana > spl-token > lib > index.browser.esm.js)

    const newAccountKeypair = Keypair.generate();

    const balanceNeeded = await Token.getMinBalanceRentForExemptAccount(provider.connection);

    const transaction = new Transaction().add(
        SystemProgram.createAccount({
            fromPubkey: provider.wallet.publicKey,
            newAccountPubkey: newAccountKeypair.publicKey,
            lamports: balanceNeeded,
            space: AccountLayout.span,
            programId: tokenProgramId
        }),
        Token.createInitAccountInstruction(tokenProgramId, mintPublicKey, newAccountKeypair.publicKey, owner)
    );

    await provider.sendAndConfirm(transaction, [newAccountKeypair], confirmOptions);

    return newAccountKeypair.publicKey;
}

export async function createTokenAccounts(
    provider: anchor.AnchorProvider,
    mint: PublicKey,
    mintAuthority: Keypair,
    authorities: PublicKey[],
    fundAmount: number | number[] | anchor.BN | anchor.BN[],
    tokenProgramId = TOKEN_PROGRAM_ID,
    confirmOptions?: anchor.web3.ConfirmOptions
): Promise<PublicKey[]> {

    const tokenAccounts = await Promise.all(authorities.map((auth, i) => {
        return createAccount(provider, mint, auth, tokenProgramId, confirmOptions)
    }));

    await Promise.all(tokenAccounts.map((tokenAccount, i) => mintTo(
        provider,
        mint,
        tokenAccount,
        mintAuthority.publicKey,
        [mintAuthority],
        Array.isArray(fundAmount) ? fundAmount[i] : fundAmount,
        tokenProgramId,
        confirmOptions
    )));

    return tokenAccounts;
}

export async function mintTo(
    provider: anchor.AnchorProvider,
    mintPubkey: PublicKey,
    destPubkey: PublicKey,
    authorityPubkey: PublicKey,
    signers: Keypair[],
    amount: number | anchor.BN,
    tokenProgramId = TOKEN_PROGRAM_ID,
    confirmOptions?: anchor.web3.ConfirmOptions
): Promise<void> {
    // ! Function based on mintTo() from @solana/spl-token Token class (check node_modules > @solana > spl-token > lib > index.browser.esm.js)

    await provider.sendAndConfirm(
        new Transaction().add(
            Token.createMintToInstruction(
                tokenProgramId,
                mintPubkey,
                destPubkey,
                authorityPubkey,
                signers,
                splu64.fromBuffer(new anchor.BN(amount).toBuffer('le', 8))
            )
        ),
        signers,
        confirmOptions
    )
}

export async function approve(
    provider: anchor.AnchorProvider,
    walletPubkey: PublicKey,
    walletAuthorityKeypair: Keypair,
    delegateAuthorityPubkey: PublicKey,
    amount: number | anchor.BN,
    tokenProgramId = TOKEN_PROGRAM_ID,
    confirmOptions?: anchor.web3.ConfirmOptions
): Promise<void> {
    // ! Function based on approve() from @solana/spl-token Token class (check node_modules > @solana > spl-token > lib > index.browser.esm.js)

    await provider.sendAndConfirm(
        new Transaction().add(
            Token.createApproveInstruction(
                tokenProgramId,
                walletPubkey,
                delegateAuthorityPubkey,
                walletAuthorityKeypair.publicKey,
                [],
                splu64.fromBuffer(new anchor.BN(amount).toBuffer('le', 8))
            )
        ),
        [walletAuthorityKeypair],
        confirmOptions
    )
}

export async function getAccountInfo(
  provider: anchor.AnchorProvider,
  mintPubkey: PublicKey,
  account: PublicKey,
  tokenProgramId = TOKEN_PROGRAM_ID,
  commitment?: Commitment
): Promise<AccountInfo> {
    // ! Function based on getAccountInfo() from @solana/spl-token Token class (check node_modules > @solana > spl-token > lib > index.browser.esm.js)

    const info = await provider.connection.getAccountInfo(account, commitment);

    if (info === null) {
        throw new Error('Failed to find account');
    }

    if (!info.owner.equals(tokenProgramId)) {
        throw new Error('Invalid account owner');
    }

    if (info.data.length != AccountLayout.span) {
        throw new Error(`Invalid account size`);
    }

    const data = Buffer.from(info.data);
    const accountInfo = AccountLayout.decode(data);
    accountInfo.address = account;
    accountInfo.mint = new PublicKey(accountInfo.mint);
    accountInfo.owner = new PublicKey(accountInfo.owner);
    accountInfo.amount = u64.fromBuffer(accountInfo.amount);

    if (accountInfo.delegateOption === 0) {
        accountInfo.delegate = null;
        accountInfo.delegatedAmount = new u64(0);
    } else {
        accountInfo.delegate = new PublicKey(accountInfo.delegate);
        accountInfo.delegatedAmount = u64.fromBuffer(accountInfo.delegatedAmount);
    }

    accountInfo.isInitialized = accountInfo.state !== 0;
    accountInfo.isFrozen = accountInfo.state === 2;

    if (accountInfo.isNativeOption === 1) {
        accountInfo.rentExemptReserve = u64.fromBuffer(accountInfo.isNative);
        accountInfo.isNative = true;
    } else {
        accountInfo.rentExemptReserve = null;
        accountInfo.isNative = false;
    }

    if (accountInfo.closeAuthorityOption === 0) {
        accountInfo.closeAuthority = null;
    } else {
        accountInfo.closeAuthority = new PublicKey(accountInfo.closeAuthority);
    }

    if (!accountInfo.mint.equals(mintPubkey)) {
        throw new Error(`Invalid account mint: ${JSON.stringify(accountInfo.mint)} !== ${JSON.stringify(mintPubkey)}`);
    }

    return accountInfo;
}

export async function getMintInfo(
    provider: anchor.AnchorProvider,
    mintPubkey: PublicKey,
    tokenProgramId = TOKEN_PROGRAM_ID
): Promise<MintInfo> {
    // ! Function based on getMintInfo() from @solana/spl-token Token class (check node_modules > @solana > spl-token > lib > index.browser.esm.js)

    const info = await provider.connection.getAccountInfo(mintPubkey);

    if (info === null) {
      throw new Error('Failed to find mint account');
    }

    if (!info.owner.equals(tokenProgramId)) {
      throw new Error(`Invalid mint owner: ${JSON.stringify(info.owner)}`);
    }

    if (info.data.length != MintLayout.span) {
      throw new Error(`Invalid mint size`);
    }

    const data = Buffer.from(info.data);
    const mintInfo = MintLayout.decode(data);

    if (mintInfo.mintAuthorityOption === 0) {
      mintInfo.mintAuthority = null;
    } else {
      mintInfo.mintAuthority = new PublicKey(mintInfo.mintAuthority);
    }

    mintInfo.supply = u64.fromBuffer(mintInfo.supply);
    mintInfo.isInitialized = mintInfo.isInitialized != 0;

    if (mintInfo.freezeAuthorityOption === 0) {
      mintInfo.freezeAuthority = null;
    } else {
      mintInfo.freezeAuthority = new PublicKey(mintInfo.freezeAuthority);
    }

    return mintInfo;
}

export async function verifyTokenWallet(
    provider: anchor.AnchorProvider,
    wallet: PublicKey,
    mint: PublicKey,
    authority: PublicKey,
    tokenProgramId = TOKEN_PROGRAM_ID
): Promise<void> {
    const walletAccountInfo = await getAccountInfo(provider, mint, wallet, tokenProgramId);

    // ! Mint verified by getAccountInfo()
    assert(walletAccountInfo.isInitialized, 'Token wallet not initialized.');
    assert(walletAccountInfo.owner === authority, 'Expected token wallet authority mismatch.');

    //TODO add more checks?: amount, delegate, delegatedAmount, isFrozen, isNative, rentExemptReserve, closeAuthority
}

export async function verifyTokenMint(
    provider: anchor.AnchorProvider,
    mint: PublicKey,
    mintAuthority: PublicKey | null,
    freezeAuthority: PublicKey | null = null,
    tokenProgramId = TOKEN_PROGRAM_ID
): Promise<void> {
    const tokenMintAccountInfo = await getMintInfo(provider, mint, tokenProgramId);

    assert(tokenMintAccountInfo.isInitialized, 'Token mint not initialized.');
    assert(tokenMintAccountInfo.mintAuthority === mintAuthority, 'Expected token mint mintAuthority mismatch.');
    assert(tokenMintAccountInfo.freezeAuthority === freezeAuthority, 'Expected token mint freezeAuthority mismatch.');

    //TODO add more checks?: supply, decimals
}