use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{Binary, Uint64, Uint128};
use cw20::{Expiration, AllowanceResponse, BalanceResponse, TokenInfoResponse};
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
    pub governance_fee: u64,
    pub fee_administrator: String,
    pub setup_master: String,
}


#[cw_serde]
pub enum ExecuteMsg {

    InitializeSwapCurves {
        assets: Vec<String>,
        assets_balances: Vec<Uint128>,
        weights: Vec<u64>,
        amp: u64,
        depositor: String
    },

    FinishSetup {},

    SetPoolFee { fee: u64 },

    SetGovernanceFee { fee: u64 },

    SetFeeAdministrator { administrator: String },

    SetConnection {
        channel_id: String,
        to_pool: String,
        state: bool
    },

    SendAssetAck {
        to_account: String,
        #[serde(with = "U256Def")]
        u: U256,
        amount: Uint128,
        asset: String,
        block_number_mod: u32
    },

    SendAssetTimeout {
        to_account: String,
        #[serde(with = "U256Def")]
        u: U256,
        amount: Uint128,
        asset: String,
        block_number_mod: u32
    },

    SendLiquidityAck {
        to_account: String,
        #[serde(with = "U256Def")]
        u: U256,
        amount: Uint128,
        block_number_mod: u32
    },

    SendLiquidityTimeout {
        to_account: String,
        #[serde(with = "U256Def")]
        u: U256,
        amount: Uint128,
        block_number_mod: u32
    },

    // Setup {
    //     assets: Vec<String>,
    //     weights: Vec<u64>,          // TODO type? (originally u256)
    //     amp: [u64; 4],
    //     governance_fee: [u64; 4],
    //     name: String,
    //     symbol: String,
    //     chain_interface: String,
    //     setup_master: String
    // },

    DepositMixed {
        deposit_amounts: Vec<Uint128>,  //TODO EVM MISMATCH
        min_out: Uint128
    },

    WithdrawAll {
        pool_tokens: Uint128,
        min_out: Vec<Uint128>
    },

    // Withdraw { pool_tokens_amount: Uint128 },

    LocalSwap {
        from_asset: String,
        to_asset: String,
        amount: Uint128,
        min_out: Uint128,
    },

    SendAsset {
        channel_id: String,
        to_pool: String,
        to_account: String,
        from_asset: String,
        to_asset_index: u8,
        amount: Uint128,
        min_out: Uint128,
        fallback_account: String,   //TODO EVM mismatch
        calldata: Vec<u8>
    },

    ReceiveAsset {
        channel_id: String,
        from_pool: String,
        to_asset_index: u8,
        to_account: String,
        #[serde(with = "U256Def")]
        u: U256,
        min_out: Uint128,
        swap_hash: String,
        calldata: Vec<u8>
    },

    SendLiquidity {
        channel_id: String,
        to_pool: String,
        to_account: String,
        amount: Uint128,            //TODO EVM mismatch
        min_out: Uint128,
        fallback_account: String,   //TODO EVM mismatch
        calldata: Vec<u8>
    },

    ReceiveLiquidity {
        channel_id: String,
        from_pool: String,
        to_account: String,
        #[serde(with = "U256Def")]
        u: U256,
        min_out: Uint128,
        swap_hash: String,
        calldata: Vec<u8>
    },


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
#[derive(QueryResponses)]
pub enum QueryMsg {

    // #[returns(ChainInterfaceResponse)]
    // ChainInterface {},

    // // TokenIndexing(tokenIndex: [u64; 4]),

    // #[returns(Balance0Response)]
    // Balance0 {
    //     token: String
    // },

    // #[returns(WeightResponse)]
    // Weight {
    //     token: String
    // },

    // #[returns(WeightResponse)]
    // TargetWeight{
    //     token: String
    // },

    // #[returns(AdjustmentTargetResponse)]
    // AdjustmentTarget {},

    // #[returns(LastModificationTimeResponse)]
    // LastModificationTime {},

    // #[returns(TargetMaxUnitInflowResponse)]
    // TargetMaxUnitInflow {},

    // #[returns(PoolFeeX64Response)]
    // PoolFeeX64 {},

    // #[returns(GovernanceFeeResponse)]
    // GovernanceFee {},

    // #[returns(FeeAdministratorResponse)]
    // FeeAdministrator {},

    // #[returns(SetupMasterResponse)]
    // SetupMaster {},

    // #[returns(MaxUnitInflowResponse)]
    // MaxUnitInflow {},

    // #[returns(EscrowedTokensResponse)]
    // EscrowedTokens { token: String },

    // #[returns(EscrowedPoolTokensResponse)]
    // EscrowedPoolTokens {},

    // // #[returns(FactoryOwnerResponse)]
    // // FactoryOwner {},

    #[returns(ReadyResponse)]
    Ready {},
    #[returns(OnlyLocalResponse)]
    OnlyLocal {},
    #[returns(GetUnitCapacityResponse)]
    GetUnitCapacity {},

    #[returns(CalcSendAssetResponse)]
    CalcSendAsset {
        from_asset: String,
        amount: Uint128
    },
    #[returns(CalcReceiveAssetResponse)]
    CalcReceiveAsset {
        to_asset: String,
        #[serde(with = "U256Def")]
        u: U256
    },
    #[returns(CalcLocalSwapResponse)]
    CalcLocalSwap {
        from_asset: String,
        to_asset: String,
        amount: Uint128
    },


    // CW20 Implementation
    #[returns(BalanceResponse)]
    Balance { address: String },
    #[returns(TokenInfoResponse)]
    TokenInfo {},
    #[returns(AllowanceResponse)]
    Allowance { owner: String, spender: String },

}


#[cw_serde]
pub struct UnitCapacityResponse {
    #[serde(with = "U256Def")]
    pub amount: U256,
}

#[cw_serde]
pub struct LiquidityUnitCapacityResponse {
    #[serde(with = "U256Def")]
    pub amount: U256,
}

#[cw_serde]
pub struct ChainInterfaceResponse {
    pub contract: String,
}

#[cw_serde]
pub struct Balance0Response {
    #[serde(with = "U256Def")]
    pub balance: U256,
}

#[cw_serde]
pub struct WeightResponse {
    pub weight: Uint64,     //TODO TYPE
}

#[cw_serde]
pub struct TargetWeightResponse {
    pub weight: Uint64,     //TODO TYPE
}

#[cw_serde]
pub struct AdjustmentTargetResponse {
    // TODO
}

#[cw_serde]
pub struct LastModificationTimeResponse {
    // TODO
}

#[cw_serde]
pub struct TargetMaxUnitInflowResponse {
    #[serde(with = "U256Def")]
    pub amount: U256
}

#[cw_serde]
pub struct PoolFeeX64Response {
    #[serde(with = "U256Def")]
    pub fee: U256    //TODO use u64?
}

#[cw_serde]
pub struct GovernanceFeeResponse {
    #[serde(with = "U256Def")]
    pub fee: U256    //TODO use u64?
}

#[cw_serde]
pub struct FeeAdministratorResponse {
    pub admin: String
}

#[cw_serde]
pub struct SetupMasterResponse {
    pub setup_master: String
}

#[cw_serde]
pub struct MaxUnitInflowResponse {
    #[serde(with = "U256Def")]
    pub amount: U256
}

#[cw_serde]
pub struct EscrowedTokensResponse {
    pub amount: Uint128
}

#[cw_serde]
pub struct EscrowedPoolTokensResponse {
    pub amount: Uint128
}

// #[cw_serde]
// pub struct FactoryOwnerResponse {

// }

#[cw_serde]
pub struct ReadyResponse {
    pub ready: Binary
}

#[cw_serde]
pub struct OnlyLocalResponse {
    pub only_local: Binary
}

#[cw_serde]
pub struct GetUnitCapacityResponse {
    #[serde(with = "U256Def")]
    pub capacity: U256
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

