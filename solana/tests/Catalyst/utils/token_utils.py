"""This module contains utilities for the SPL Token Program."""
import asyncio
from solana.publickey import PublicKey
from solana.keypair import Keypair
from solana.transaction import Transaction
from solana.system_program import create_account, CreateAccountParams
from spl.token.constants import TOKEN_PROGRAM_ID
from spl.token.instructions import (
    initialize_mint,
    InitializeMintParams,
    initialize_account,
    InitializeAccountParams,
    mint_to as mint_to_instruction,
    MintToParams,
    burn as burn_instruction,
    BurnParams,
    approve as approve_instruction,
    ApproveParams
)
from anchorpy import Provider
from spl.token.core import AccountInfo, MintInfo
from solana.rpc.commitment import Commitment
from spl.token._layouts import ACCOUNT_LAYOUT, MINT_LAYOUT
import solana.system_program as sp
from solana.rpc.types import TxOpts

from utils.transaction_utils import DEFAULT_SKIP_PREFLIGHT, confirm_transaction, DEFAULT_TX_COMMITMENT

async def create_mint(
    provider: Provider,
    mint_authority: PublicKey,
    freeze_authority: PublicKey | None = None,
    decimals: int | None = None,
    tx_opts: TxOpts = TxOpts(skip_preflight=DEFAULT_SKIP_PREFLIGHT),
    commitment: Commitment = DEFAULT_TX_COMMITMENT,
) -> PublicKey:

    mint = Keypair()
    
    tx = Transaction()
    mint_space = 82
    create_mint_mbre_resp = (
        await provider.connection.get_minimum_balance_for_rent_exemption(mint_space)
    )
    create_mint_mbre = create_mint_mbre_resp.value
    create_mint_account_params = CreateAccountParams(
        from_pubkey=provider.wallet.public_key,
        new_account_pubkey=mint.public_key,
        space=mint_space,
        lamports=create_mint_mbre,
        program_id=TOKEN_PROGRAM_ID,
    )
    create_mint_account_instruction = create_account(
        create_mint_account_params,
    )
    init_mint_instruction = initialize_mint(
        InitializeMintParams(
            mint=mint.public_key,
            decimals=0 if decimals is None else decimals,
            mint_authority=mint_authority,
            freeze_authority=freeze_authority,
            program_id=TOKEN_PROGRAM_ID,
        ),
    )

    tx.add(
        create_mint_account_instruction,
        init_mint_instruction
    )

    tx = await provider.send(tx, [mint], tx_opts)
    await confirm_transaction(provider, tx, commitment)

    return mint.public_key


async def create_token_account(
    provider: Provider,
    mint: PublicKey,
    owner: PublicKey,
    payer: Keypair | None = None, # TODO remove
    token_program_id: PublicKey = TOKEN_PROGRAM_ID,
    tx_opts: TxOpts = TxOpts(skip_preflight=DEFAULT_SKIP_PREFLIGHT),
    commitment: Commitment = DEFAULT_TX_COMMITMENT
) -> PublicKey:

    account = Keypair()
    tx = Transaction()

    payer = payer or provider.wallet.payer

    balance_needed = (await provider.connection.get_minimum_balance_for_rent_exemption(ACCOUNT_LAYOUT.sizeof())).value

    
    tx.add(
        sp.create_account(
            sp.CreateAccountParams(
                from_pubkey=payer.public_key,
                new_account_pubkey=account.public_key,
                lamports=balance_needed,
                space=ACCOUNT_LAYOUT.sizeof(),
                program_id=token_program_id,
            )
        )
    )
    tx.add(
        initialize_account(
            InitializeAccountParams(
                account=account.public_key,
                mint=mint,
                owner=owner,
                program_id=token_program_id
            )
        )
    )

    tx_result = await provider.send(tx, [payer, account], opts=tx_opts)
    await confirm_transaction(provider, tx_result, commitment)

    return account.public_key


async def mint_to(
    provider: Provider,
    wallet: PublicKey,
    mint: PublicKey,
    mint_authority_keypair: Keypair,
    amount: int,
    token_program_id: PublicKey = TOKEN_PROGRAM_ID,
    tx_opts: TxOpts = TxOpts(skip_preflight=DEFAULT_SKIP_PREFLIGHT),
    commitment: Commitment = DEFAULT_TX_COMMITMENT
):
    tx = Transaction().add(
        mint_to_instruction(
            MintToParams(
                program_id=token_program_id,
                mint=mint,
                dest=wallet,
                mint_authority=mint_authority_keypair.public_key,
                amount=amount,
                signers=[mint_authority_keypair.public_key],
            )
        )
    )

    tx_result = await provider.send(tx, [mint_authority_keypair], opts=tx_opts)
    await confirm_transaction(provider, tx_result, commitment)   # ! TODO this shouldn't be required


async def burn(
    provider: Provider,
    wallet: PublicKey,
    mint: PublicKey,
    wallet_authority_keypair: Keypair,
    amount: int,
    token_program_id: PublicKey = TOKEN_PROGRAM_ID,
    tx_opts: TxOpts = TxOpts(skip_preflight=DEFAULT_SKIP_PREFLIGHT),
    commitment: Commitment = DEFAULT_TX_COMMITMENT
):
    tx = Transaction().add(
        burn_instruction(
            BurnParams(
                program_id=token_program_id,
                mint=mint,
                account=wallet,
                owner=wallet_authority_keypair.public_key,
                amount=amount,
                signers=[wallet_authority_keypair.public_key],
            )
        )
    )

    tx_result = await provider.send(tx, [wallet_authority_keypair], opts=tx_opts)
    await confirm_transaction(provider, tx_result, commitment)   # ! TODO this shouldn't be required


async def approve(
    provider: Provider,
    wallet: PublicKey,
    wallet_authority_keypair: Keypair,
    delegate_authority: PublicKey,
    amount: int,
    token_program_id: PublicKey = TOKEN_PROGRAM_ID,
    tx_opts: TxOpts = TxOpts(skip_preflight=DEFAULT_SKIP_PREFLIGHT),
    commitment: Commitment = DEFAULT_TX_COMMITMENT
):
    tx = Transaction().add(
        approve_instruction(
            ApproveParams(
                program_id=token_program_id,
                source=wallet,
                delegate=delegate_authority,
                owner=wallet_authority_keypair.public_key,
                amount=amount,
                signers=[wallet_authority_keypair.public_key]
            )
        )
    )

    tx_result = await provider.send(tx, [wallet_authority_keypair], opts=tx_opts)
    await confirm_transaction(provider, tx_result, commitment)   # ! TODO this shouldn't be required

# From AsyncToken
async def get_account_info(
    provider: Provider,
    mint: PublicKey,
    account: PublicKey,
    token_program_id: PublicKey = TOKEN_PROGRAM_ID,
    commitment: Commitment | None = DEFAULT_TX_COMMITMENT
) -> AccountInfo:
    """
        Retrieve account information.
        Function copied from spl token library
    """
    account_info = await provider.connection.get_account_info(account, commitment)

    if account_info.value is None:
        raise ValueError("Invalid account owner")

    if account_info.value.owner != token_program_id.to_solders():
        raise AttributeError("Invalid account owner")

    bytes_data = account_info.value.data
    if len(bytes_data) != ACCOUNT_LAYOUT.sizeof():
        raise ValueError("Invalid account size")

    decoded_data = ACCOUNT_LAYOUT.parse(bytes_data)

    account_mint = PublicKey(decoded_data.mint)
    owner = PublicKey(decoded_data.owner)
    amount = decoded_data.amount

    if decoded_data.delegate_option == 0:
        delegate = None
        delegated_amount = 0
    else:
        delegate = PublicKey(decoded_data.delegate)
        delegated_amount = decoded_data.delegated_amount

    is_initialized = decoded_data.state != 0
    is_frozen = decoded_data.state == 2

    if decoded_data.is_native_option == 1:
        rent_exempt_reserve = decoded_data.is_native
        is_native = True
    else:
        rent_exempt_reserve = None
        is_native = False

    if decoded_data.close_authority_option == 0:
        close_authority = None
    else:
        close_authority = PublicKey(decoded_data.owner)

    if account_mint != mint:
        raise AttributeError(f"Invalid account mint: {decoded_data.mint} != {mint}")

    return AccountInfo(
        account_mint,
        owner,
        amount,
        delegate,
        delegated_amount,
        is_initialized,
        is_frozen,
        is_native,
        rent_exempt_reserve,
        close_authority,
    )

# From AsyncToken
async def get_mint_info(
    provider: Provider,
    mint: PublicKey,
    token_program_id: PublicKey = TOKEN_PROGRAM_ID,
    commitment: Commitment | None = DEFAULT_TX_COMMITMENT
) -> MintInfo:
    info = await provider.connection.get_account_info(mint, commitment)

    if info.value is None:
        raise ValueError("Failed to find mint account")

    owner = info.value.owner
    if owner != token_program_id.to_solders():
        raise AttributeError(f"Invalid mint owner: {owner}")

    bytes_data = info.value.data
    if len(bytes_data) != MINT_LAYOUT.sizeof():
        raise ValueError("Invalid mint size")

    decoded_data = MINT_LAYOUT.parse(bytes_data)
    decimals = decoded_data.decimals

    if decoded_data.mint_authority_option == 0:
        mint_authority = None
    else:
        mint_authority = PublicKey(decoded_data.mint_authority)

    supply = decoded_data.supply
    is_initialized = decoded_data.is_initialized != 0

    if decoded_data.freeze_authority_option == 0:
        freeze_authority = None
    else:
        freeze_authority = PublicKey(decoded_data.freeze_authority)

    return MintInfo(mint_authority, supply, decimals, is_initialized, freeze_authority)

"""
    Fund a single token account per provided mint. Optionally, set allowance for a third party.
"""
async def fund_accounts(
    provider: Provider,
    mints: list[PublicKey],
    mints_authority: Keypair | list[Keypair],
    token_accounts: list[PublicKey],
    balance: int | list[int],
    delegate_authority: PublicKey | None = None,
    delegate_balance: int | list[int] | None = None,
    token_accounts_authority: Keypair | list[Keypair] | None = None,
) -> None:

    assert len(mints) == len(token_accounts)

    async def fund_depositor_wallet(mint_index):
        # Mint tokens for the depositor
        await mint_to(
            provider,
            token_accounts[mint_index],
            mints[mint_index],
            mints_authority[mint_index] if isinstance(mints_authority, list) else mints_authority,
            balance[mint_index] if isinstance(balance, list) else balance,
        )
            
        # Delegate tokens
        if delegate_authority is not None:
            if token_accounts_authority is None:
                raise Exception("Cannot delegate tokens: token account authority not provided.")

            def get_allowance_balance():
                if delegate_balance is None:
                    return balance[mint_index] if isinstance(balance, list) else balance
                else:
                    return delegate_balance[mint_index] if isinstance(delegate_balance, list) else delegate_balance

            await approve(
                provider,
                token_accounts[mint_index],
                token_accounts_authority[mint_index] if isinstance(token_accounts_authority, list) else token_accounts_authority,
                delegate_authority,
                get_allowance_balance()
            )
    
    # Run in parallel to improve the  execution time
    await asyncio.gather(*[
        fund_depositor_wallet(mint_index) for mint_index in range(len(mints))
    ])
    
# Verification functions ********************************************************************************************************
async def verify_token_wallet(
    provider: Provider,
    wallet: PublicKey,
    mint: PublicKey,
    authority: PublicKey
):
    wallet_info = await get_account_info(provider, mint, wallet)

    assert wallet_info.is_initialized, "Token wallet not initialized"
    assert wallet_info.owner == authority, "Expected token wallet authority mismatch"

    #TODO! ADD MORE CHECKS: amount, delegate, delegatedAmount, isFrozen, isNative, rentExemptReserve, closeAuthority


async def verify_token_mint(
    provider: Provider,
    mint: PublicKey,
    mint_authority: PublicKey | None,
    freeze_authority: PublicKey | None = None,
):
    token_mint_info = await get_mint_info(provider, mint)

    assert token_mint_info.is_initialized, 'Token mint not initialized.'
    assert token_mint_info.mint_authority == mint_authority, 'Expected token mint mint_authority mismatch.'
    assert token_mint_info.freeze_authority == freeze_authority, 'Expected token mint freeze_authority mismatch.'

    #TODO! ADD MORE CHECKS: supply, decimals