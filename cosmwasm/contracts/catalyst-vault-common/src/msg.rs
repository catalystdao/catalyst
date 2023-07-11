use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Binary, Uint64, Uint128, Addr};
use cw20::Expiration;
use catalyst_types::U256;



/// Vault instantiation struct
/// * `name` - The name for the vault token. 
/// * `symbol` - The symbol for the vault token. 
/// * `chain_interface` - The interface used for cross-chain swaps. It can be set to None to disable cross-chain swaps. 
/// * `vault_fee` - The vault fee (18 decimals). 
/// * `governance_fee_share` - The governance fee share (18 decimals). 
/// * `fee_administrator` - The account which has the authority to modify the vault fee. 
/// * `setup_master` - The account which has the authority to continue setting up the vault (until `finish_setup` is called).
#[cw_serde]
pub struct InstantiateMsg {
    pub name: String,
    pub symbol: String,
    pub chain_interface: Option<String>,
    pub vault_fee: Uint64,
    pub governance_fee_share: Uint64,
    pub fee_administrator: String,
    pub setup_master: String,
}


#[cw_serde]
pub enum ExecuteMsg<T> {

    /// Initialize the vault swap curves.
    /// * `assets` - The list of the assets that are to be supported by the vault.
    /// * `weights` - The weights applied to the assets.
    /// * `amp` - The amplification value applied to the vault.
    /// * `depositor` - The account that will receive the initial vault tokens.
    InitializeSwapCurves {
        assets: Vec<String>,
        weights: Vec<Uint128>,
        amp: Uint64,
        depositor: String
    },

    /// Finish the vault setup. This revokes the 'setup_master' authority.
    FinishSetup {},

    /// Set the vault fee.
    /// * `fee`: The new vault fee (18 decimals).
    SetVaultFee { fee: Uint64 },

    /// Set the governance fee share.
    /// * `fee`: The new governance fee share (18 decimals).
    SetGovernanceFeeShare { fee: Uint64 },

    /// Set the fee administrator.
    /// * `administrator`: The new administrator account.
    SetFeeAdministrator { administrator: String },

    /// Setup a vault connection.
    /// * `channel_id` - The channel id that connects with the remoute vault.
    /// * `vault` - The remote vault address to be connected to this vault.
    /// * `state` - Whether the connection is enabled.
    SetConnection {
        channel_id: String,
        to_vault: Binary,
        state: bool
    },

    /// Deposit a user-configurable balance of assets on the vault.
    /// * `deposit_amounts` - The asset amounts to be deposited.
    /// * `min_out` - The minimum output of vault tokens to get in return.
    DepositMixed {
        deposit_amounts: Vec<Uint128>,
        min_out: Uint128
    },

    /// Withdraw an even amount of assets from the vault.
    /// * `vault_tokens` - The amount of vault tokens to burn.
    /// * `min_out` - The minimum output of assets to get in return.
    WithdrawAll {
        vault_tokens: Uint128,
        min_out: Vec<Uint128>
    },


    /// Withdraw an uneven amount of assets from the vault.
    /// * `vault_tokens` - The amount of vault tokens to burn.
    /// * `withdraw_ratio` - The ratio at which to withdraw the assets.
    /// * `min_out` - The minimum output of assets to get in return.
    WithdrawMixed {
        vault_tokens: Uint128,
        withdraw_ratio: Vec<Uint64>,
        min_out: Vec<Uint128>,
    },

    /// Perform a local asset swap.
    /// * `from_asset` - The source asset.
    /// * `to_asset` - The destination asset.
    /// * `amount` - The `from_asset` amount sold to the vault.
    /// * `min_out` - The mininmum return to get of `to_asset`.
    LocalSwap {
        from_asset: String,
        to_asset: String,
        amount: Uint128,
        min_out: Uint128,
    },

    /// Initiate a cross-chain asset swap.
    /// * `channel_id` - The target chain identifier.
    /// * `to_vault` - The target vault on the target chain (Catalyst encoded).
    /// * `to_account` - The recipient of the swap on the target chain (Catalyst encoded).
    /// * `from_asset` - The source asset.
    /// * `to_asset_index` - The destination asset index.
    /// * `amount` - The `from_asset` amount sold to the vault.
    /// * `min_out` - The mininum `to_asset` output amount to get on the target vault.
    /// * `fallback_account` - The recipient of the swapped amount should the swap fail.
    /// * `calldata` - Arbitrary data to be executed on the target chain upon successful execution of the swap.
    SendAsset {
        channel_id: String,
        to_vault: Binary,
        to_account: Binary,
        from_asset: String,
        to_asset_index: u8,
        amount: Uint128,
        min_out: U256,
        fallback_account: String,
        calldata: Binary
    },

    /// Receive a cross-chain asset swap.
    /// * `channel_id` - The source chain identifier.
    /// * `from_vault` - The source vault on the source chain.
    /// * `to_asset_index` - The index of the purchased asset.
    /// * `to_account` - The recipient of the swap.
    /// * `u` - The incoming units.
    /// * `min_out` - The mininum output amount.
    /// * `from_amount` - The `from_asset` amount sold to the source vault.
    /// * `from_asset` - The source asset.
    /// * `from_block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32).
    /// * `calldata_target` - The contract address to invoke upon successful execution of the swap.
    /// * `calldata` - The data to pass to `calldata_target` upon successful execution of the swap.
    ReceiveAsset {
        channel_id: String,
        from_vault: Binary,
        to_asset_index: u8,
        to_account: String,
        u: U256,
        min_out: Uint128,
        from_amount: U256,
        from_asset: Binary,
        from_block_number_mod: u32,
        calldata_target: Option<Addr>,
        calldata: Option<Binary>
    },

    /// Initiate a cross-chain liquidity swap.
    /// * `channel_id` - The target chain identifier.
    /// * `to_vault` - The target vault on the target chain (Catalyst encoded).
    /// * `to_account` - The recipient of the swap on the target chain (Catalyst encoded).
    /// * `amount` - The vault tokens amount sold to the vault.
    /// * `min_vault_tokens` - The mininum vault tokens output amount to get on the target vault.
    /// * `min_reference_asset` - The mininum reference asset value on the target vault.
    /// * `fallback_account` - The recipient of the swapped amount should the swap fail.
    /// * `calldata` - Arbitrary data to be executed on the target chain upon successful execution of the swap.
    SendLiquidity {
        channel_id: String,
        to_vault: Binary,
        to_account: Binary,
        amount: Uint128,
        min_vault_tokens: U256,
        min_reference_asset: U256,
        fallback_account: String,
        calldata: Binary
    },

    /// Receive a cross-chain liquidity swap.
    /// * `channel_id` - The source chain identifier.
    /// * `from_vault` - The source vault on the source chain.
    /// * `to_account` - The recipient of the swap.
    /// * `u` - The incoming units.
    /// * `min_vault_tokens` - The mininum vault tokens output amount.
    /// * `min_reference_asset` - The mininum reference asset value.
    /// * `from_amount` - The `from_asset` amount sold to the source vault.
    /// * `from_block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32).
    /// * `calldata_target` - The contract address to invoke upon successful execution of the swap.
    /// * `calldata` - The data to pass to `calldata_target` upon successful execution of the swap.
    ReceiveLiquidity {
        channel_id: String,
        from_vault: Binary,
        to_account: String,
        u: U256,
        min_vault_tokens: Uint128,
        min_reference_asset: Uint128,
        from_amount: U256,
        from_block_number_mod: u32,
        calldata_target: Option<Addr>,
        calldata: Option<Binary>
    },

    /// Handle the confirmation of a successful asset swap.
    /// * `channel_id` - The swap's channel id.
    /// * `to_account` - The recipient of the swap output.
    /// * `u` - The units value of the swap.
    /// * `escrow_amount` - The escrowed asset amount.
    /// * `asset` - The swap source asset.
    /// * `block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32).
    OnSendAssetSuccess {
        channel_id: String,
        to_account: Binary,
        u: U256,
        amount: Uint128,
        asset: String,
        block_number_mod: u32
    },

    /// Handle the confirmation of an unsuccessful asset swap.
    /// * `channel_id` - The swap's channel id.
    /// * `to_account` - The recipient of the swap output.
    /// * `u` - The units value of the swap.
    /// * `escrow_amount` - The escrowed asset amount.
    /// * `asset` - The swap source asset.
    /// * `block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32).
    OnSendAssetFailure {
        channel_id: String,
        to_account: Binary,
        u: U256,
        amount: Uint128,
        asset: String,
        block_number_mod: u32
    },

    /// Handle the confirmation of a successful liquidity swap.
    /// * `channel_id` - The swap's channel id.
    /// * `to_account` - The recipient of the swap output.
    /// * `u` - The units value of the swap.
    /// * `escrow_amount` - The escrowed liquidity amount.
    /// * `block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32).
    OnSendLiquiditySuccess {
        channel_id: String,
        to_account: Binary,
        u: U256,
        amount: Uint128,
        block_number_mod: u32
    },

    /// Handle the confirmation of an unsuccessful liquidity swap.
    /// * `channel_id` - The swap's channel id.
    /// * `to_account` - The recipient of the swap output.
    /// * `u` - The units value of the swap.
    /// * `escrow_amount` - The escrowed liquidity amount.
    /// * `block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32).
    OnSendLiquidityFailure {
        channel_id: String,
        to_account: Binary,
        u: U256,
        amount: Uint128,
        block_number_mod: u32
    },

    /// Field to allow vault implementations to extend the ExecuteMsg with custom execute calls.
    Custom (T),


    // CW20 Implementation (base messages + 'approval' extension)
    // Refer to the cw20 package for a description on the following message definitions.
    Transfer {
        recipient: String,
        amount: Uint128
    },
    Burn {
        amount: Uint128
    },
    Send {
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
    IncreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    DecreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    TransferFrom {
        owner: String,
        recipient: String,
        amount: Uint128,
    },
    SendFrom {
        owner: String,
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
    BurnFrom {
        owner: String,
        amount: Uint128
    },
}


#[cw_serde]
pub struct ChainInterfaceResponse {
    pub chain_interface: Option<Addr>
}

#[cw_serde]
pub struct SetupMasterResponse {
    pub setup_master: Option<Addr>
}

#[cw_serde]
pub struct FactoryResponse {
    pub factory: Addr
}

#[cw_serde]
pub struct FactoryOwnerResponse {
    pub factory_owner: Addr
}

#[cw_serde]
pub struct ReadyResponse {
    pub ready: bool
}

#[cw_serde]
pub struct OnlyLocalResponse {
    pub only_local: bool
}

#[cw_serde]
pub struct AssetsResponse {
    pub assets: Vec<Addr>
}

#[cw_serde]
pub struct WeightResponse {
    pub weight: Uint128
}

#[cw_serde]
pub struct VaultFeeResponse {
    pub fee: Uint64
}

#[cw_serde]
pub struct GovernanceFeeShareResponse {
    pub fee: Uint64
}

#[cw_serde]
pub struct FeeAdministratorResponse {
    pub administrator: Addr
}

#[cw_serde]
pub struct CalcSendAssetResponse {
    pub u: U256
}

#[cw_serde]
pub struct CalcReceiveAssetResponse {
    pub to_amount: Uint128
}

#[cw_serde]
pub struct CalcLocalSwapResponse {
    pub to_amount: Uint128
}

#[cw_serde]
pub struct GetLimitCapacityResponse {
    pub capacity: U256
}

#[cw_serde]
pub struct TotalEscrowedAssetResponse {
    pub amount: Uint128
}

#[cw_serde]
pub struct TotalEscrowedLiquidityResponse {
    pub amount: Uint128
}

#[cw_serde]
pub struct AssetEscrowResponse {
    pub fallback_account: Option<Addr>
}

#[cw_serde]
pub struct LiquidityEscrowResponse {
    pub fallback_account: Option<Addr>
}

#[cw_serde]
pub struct VaultConnectionStateResponse {
    pub state: bool
}
