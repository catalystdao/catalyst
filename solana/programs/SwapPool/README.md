# SwapPool
The SwapPool program is used to create SwapPools that can then be used to trade assets (tokens). Each created SwapPool is nothing more than an Account on the Solana chain that holds the state of the SwapPool (SwapPoolState). The created SwapPool is then identified as the address (public key) where the pool state is stored at.
# Definitions
Given that the SwapPool holds tokens with two different purposes, the following naming convention is followed:
- **Asset**:
    - A token that is traded.
- **Pool Token**
    - A token, created by the pool, to represent an asset.

The following naming convention is followed to refer to specific Solana accounts:
- **Wallet**
    - An account that holds tokens (i.e. Token account)

Users:
- **Depositor**
    - A user that provides liquidity to the pool
- **Withdrawer**
    - A user that takes liquidity from the pool
- **Trader**
    - A user that swaps tokens using the pool's liquidity



# Create a SwapPool
The steps to create and setup a new SwapPool are:
1. Create the pool with **create_swap_pool()**
    - The transaction signer *setup_master* is set as the only authority allowed to make changes during the pool setup process.
    - Internally, the Account that holds the SwapPool state is initialised, and rent is payed by the *setup_master*.
        - **TODO** Separate the payer from the *setup_master*?
2. Add assets (tokens) to the SwapPool via **add_swap_pool_asset()**
    - The desired token mint account is added as an asset to the *SwapPoolState*.
    - Internally, a wallet to hold the asset tokens, and a token mint to create tokens that represent the asset inside the pool are created. To create these, two PDAs (Program Derived Address) have to be passed to the instruction call.
        - There are no explicit checks to prevent an asset from being added twice. HOWEVER, since the asset wallet is created at a specific PDA derived from the SwapPool state address and the asset mint address (see PDAs section), the instruction call will fail if the token wallet already exists.
        - **NOTE**: Since the derivation of the asset wallet PDA is deterministic, this implementation of the SwapPool will fail to add an asset to the pool should a third party create the asset wallet externally. This shouldn't be a problem since:
            1. The PDA is derived from the SwapPoolState public key, which can be kept private until the pool setup is complete.
            2. **TODO CHECK** The entire setup process can be executed atomically.
    - The authorities for the asset wallet and the token mint also have to be passed (PDAs too).
    - Rent for the asset wallet and token mint is payed by the *setup_master*.
    - **NOTE**: Assets are added individually, and not as group when calling *create_swap_pool()* (like with the EVM implementation), because Anchor does not (currently?) support passing arrays of accounts to instructions. Also, creation of the asset wallet and token mint is defined in the *add_swap_pool_asset* context definition (*AddSwapPoolAsset*); the way these are defined would make it incompatible with passing an array of asset accounts.
3. **TODO** Create SwapPool ICCS connection
4. Finish the setup via **finish_setup()**
    - To finish the setup, at least two assets must have been added to the pool.
    - Internally, *setup_master* is set as the *default* pubkey (all zeros), blocking any future calls to setup calls.

# PDAs
With Solana, for Programs (i.e. smart contracts) to act as user accounts (e.g. to act as an authority of a token mint), PDAs (Program Derived Address) are used. In a nutshell, it is a public key that is derived from the deployed program id, that is guaranteed NOT to have an associated private key. For a transaction that is required to be signed by a PDA to be valid, the program with the id that is used to derive the PDA must be the caller of the transaction.

To obtain different PDAs for the same program, seeds (byte arrays) are used. Given that the same program (SwapPool) is used for all the created SwapPools, the used PDAs are derived as follows:
- Asset wallet PDA:
    - Each SwapPool must have a unique PDA for the token wallets of each asset mint. The seed is composed of:
        1. The SwapPoolState account public key
        2. The asset mint public key
        3. The string "poolAsset"
- Asset wallet authority PDA:
    - Each SwapPool has a unique PDA that acts as authority for all the created asset wallets. The seed is composed of:
        1. The SwapPoolState account public key
        2. The string "poolAssetAuth"
- Token mint PDA:
    - Each SwapPool must have a unique PDA for the pool token mints corresponding to each of the pool's assets. The seed is composed of:
        1. The SwapPoolState account public key
        2. The asset mint public key that corresponds to this pool token mint
        3. The string "poolMint"
- Token mint mintAuthority PDA:
    - Each SwapPool has a unique PDA that acts as the mintAuthority of all the created pool token mints. The seed is composed of:
        1. The SwapPoolState account public key
        2. The string "poolMintsAuth"

# Instruction Contexts
## Accounts Naming Convention
Given the large count of different contexts that are used by this program (one per instruction), and the large amount of accounts that are passed to each context, the following naming convention is used for accounts that are used in different contexts and that share the same functionality:
- **input_\***
    - An account linked to an *asset* or *pool token* that is going into the pool.
        - For *assets*, it can be either when they are being provided for liquidity, or when they are being swapped for another asset.
        - For *pool tokens*, it is for when liquidity is being withdrawn.
    - > Example: ***input_asset_wallet*** is the wallet that holds the tokens that are being provided to the pool.
- **output_\***
    - An account of an *asset* or *pool token* that is going out of the pool.
        - For *assets*, it can be either when liquidity is taken out of the pool, or for assets that have been swapped for another asset.
        - For *pool tokens*, it is for when liquidity is being supplied to the pool.
    - > Example: ***output_asset_wallet*** is the wallet to which transfer the tokens that are being taken from the pool.
- **swap_pool_input_\***
    - An account that belongs to the pool that mirrors an ***input_asset_\**** account.
    - > Example: When sending assets into the pool, the account named ***swap_pool_input_asset_wallet*** is the account used by the pool to hold the tokens that are of the same type as the wallet of the tokens being provided, that is ***input_asset_wallet***.
- **swap_pool_output_\***
    - An account that belongs to the pool that mirrors an ***output_asset_\**** account.
    - > Example: When taking assets from the pool, the account named ***swap_pool_output_asset_wallet*** is the account used by the pool to hold the tokens that are of the same type as the wallet of the tokens being taken, that is ***output_asset_wallet***.
