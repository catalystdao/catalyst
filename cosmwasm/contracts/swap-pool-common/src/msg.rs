use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Binary, Uint128, Addr};
use cw20::Expiration;
use ethnum::U256;
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};


// Implement JsonSchema for U256, see https://graham.cool/schemars/examples/5-remote_derive/
//TODO VERIFY THIS IS CORRECT AND SAFE!
//TODO move to common place
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(remote = "U256")]
pub struct U256Def([u128; 2]);


#[cw_serde]
pub struct InstantiateMsg {
    pub name: String,       // Name for the pool token
    pub symbol: String,     // Symbol for the pool token
    pub chain_interface: Option<String>,
    pub pool_fee: u64,
    pub governance_fee: u64,        // TODO rename gov_fee_share
    pub fee_administrator: String,
    pub setup_master: String,
}


#[cw_serde]
pub enum ExecuteMsg<T> {

    InitializeSwapCurves {
        assets: Vec<String>,
        weights: Vec<u64>,
        amp: u64,
        depositor: String
    },

    FinishSetup {},

    SetPoolFee { fee: u64 },

    SetGovernanceFeeShare { fee: u64 },

    SetFeeAdministrator { administrator: String },

    SetConnection {
        channel_id: String,
        to_pool: Vec<u8>,
        state: bool
    },

    OnSendAssetSuccess {
        channel_id: String,
        to_account: Vec<u8>,
        #[serde(with = "U256Def")]
        u: U256,
        amount: Uint128,
        asset: String,
        block_number_mod: u32
    },

    OnSendAssetFailure {
        channel_id: String,
        to_account: Vec<u8>,
        #[serde(with = "U256Def")]
        u: U256,
        amount: Uint128,
        asset: String,
        block_number_mod: u32
    },

    OnSendLiquiditySuccess {
        channel_id: String,
        to_account: Vec<u8>,
        #[serde(with = "U256Def")]
        u: U256,
        amount: Uint128,
        block_number_mod: u32
    },

    OnSendLiquidityFailure {
        channel_id: String,
        to_account: Vec<u8>,
        #[serde(with = "U256Def")]
        u: U256,
        amount: Uint128,
        block_number_mod: u32
    },

    DepositMixed {
        deposit_amounts: Vec<Uint128>,  //TODO EVM MISMATCH
        min_out: Uint128
    },

    WithdrawAll {
        pool_tokens: Uint128,
        min_out: Vec<Uint128>
    },

    WithdrawMixed {
        pool_tokens: Uint128,
        withdraw_ratio: Vec<u64>,   //TODO type
        min_out: Vec<Uint128>,
    },

    LocalSwap {
        from_asset: String,
        to_asset: String,
        amount: Uint128,
        min_out: Uint128,
    },

    SendAsset {
        channel_id: String,
        to_pool: Vec<u8>,
        to_account: Vec<u8>,
        from_asset: String,
        to_asset_index: u8,
        amount: Uint128,
        #[serde(with = "U256Def")]
        min_out: U256,
        fallback_account: String,   //TODO EVM mismatch
        calldata: Vec<u8>
    },

    ReceiveAsset {
        channel_id: String,
        from_pool: Vec<u8>,
        to_asset_index: u8,
        to_account: String,
        #[serde(with = "U256Def")]
        u: U256,
        min_out: Uint128,
        #[serde(with = "U256Def")]
        from_amount: U256,
        from_asset: Vec<u8>,
        from_block_number_mod: u32,
        calldata_target: Option<Addr>,
        calldata: Option<Vec<u8>>
    },

    SendLiquidity {
        channel_id: String,
        to_pool: Vec<u8>,
        to_account: Vec<u8>,
        amount: Uint128,            //TODO EVM mismatch
        #[serde(with = "U256Def")]
        min_pool_tokens: U256,      //TODO EVM mismatch
        #[serde(with = "U256Def")]
        min_reference_asset: U256,  //TODO EVM mismatch
        fallback_account: String,   //TODO EVM mismatch
        calldata: Vec<u8>
    },

    ReceiveLiquidity {
        channel_id: String,
        from_pool: Vec<u8>,
        to_account: String,
        #[serde(with = "U256Def")]
        u: U256,
        min_pool_tokens: Uint128,
        min_reference_asset: Uint128,   // ! TODO type?
        #[serde(with = "U256Def")]
        from_amount: U256,
        from_block_number_mod: u32,
        calldata_target: Option<Addr>,
        calldata: Option<Vec<u8>>
    },

    Custom (T),


    // CW20 Implementation
    Transfer { recipient: String, amount: Uint128 },
    Burn { amount: Uint128 },
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
    BurnFrom { owner: String, amount: Uint128 },
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
pub struct WeightsResponse {
    pub weights: Vec<u64>
}

#[cw_serde]
pub struct PoolFeeResponse {
    pub fee: u64
}

#[cw_serde]
pub struct GovernanceFeeShareResponse {
    pub fee: u64
}

#[cw_serde]
pub struct FeeAdministratorResponse {
    pub administrator: Addr
}

#[cw_serde]
pub struct CalcSendAssetResponse {
    #[serde(with = "U256Def")]
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
    #[serde(with = "U256Def")]
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
pub struct PoolConnectionStateResponse {
    pub state: bool
}
