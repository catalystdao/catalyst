
from functools import cache
from typing import Tuple
from solana.publickey import PublicKey

POOL_ASSET_WALLET_SEED      : str = "poolAsset"
POOL_TOKEN_MINT_SEED        : str = "poolMint"
POOL_AUTHORITY              : str = "poolAuth"
INTERFACE_SWAP_AUTHORITY    : str = "intSwapAuth"
POOL_ESCROW_SEED            : str = "poolEscrow"


@cache
def get_swap_pool_authority(
    swap_pool_program_id: PublicKey,
    swap_pool_state_pubkey: PublicKey
) -> Tuple[PublicKey, int]:
    return PublicKey.find_program_address(
        [
            swap_pool_state_pubkey.__bytes__(),
            POOL_AUTHORITY.encode('utf-8')
        ],
        swap_pool_program_id
    )


@cache
def get_swap_pool_asset_wallet(
    swap_pool_program_id: PublicKey,
    swap_pool_state_pubkey: PublicKey,
    asset_mint: PublicKey
) -> Tuple[PublicKey, int]:
    return PublicKey.find_program_address(
        [
            swap_pool_state_pubkey.__bytes__(),
            asset_mint.__bytes__(),
            POOL_ASSET_WALLET_SEED.encode('utf-8')
        ],
        swap_pool_program_id
    )


@cache
def get_swap_pool_token_mint(
    swap_pool_program_id: PublicKey,
    swap_pool_state_pubkey: PublicKey
) -> Tuple[PublicKey, int]:
    return PublicKey.find_program_address(
        [
            swap_pool_state_pubkey.__bytes__(),
            POOL_TOKEN_MINT_SEED.encode('utf-8')
        ],
        swap_pool_program_id
    )


@cache
def get_swap_pool_escrow_wallet(
    swap_pool_program_id: PublicKey,
    swap_pool_state_pubkey: PublicKey,
    escrow_nonce: int
) -> Tuple[PublicKey, int]:
    return PublicKey.find_program_address(
        [
            swap_pool_state_pubkey.__bytes__(),
            escrow_nonce.to_bytes(4, 'big'),
            POOL_ESCROW_SEED.encode('utf-8')
        ],
        swap_pool_program_id
    )