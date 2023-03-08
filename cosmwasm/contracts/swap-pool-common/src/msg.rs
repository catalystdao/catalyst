use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{Binary, Uint64, Uint128};
use cw20::{Expiration, AllowanceResponse, BalanceResponse, TokenInfoResponse};


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

    SetPoolFee { fee: u64 },

    SetGovernanceFee { fee: u64 },

    SetFeeAdministrator { administrator: String },

    // CreateConnection {
    //     channel_id: String,
    //     pool: String,
    //     state: bool
    // },

    // CreateConnectionWithChain {
    //     chain_id: [u64; 4],
    //     pool: String,
    //     state: bool
    // },

    // FinishSetup {},

    // ReleaseEscrowAck {
    //     message_hash: String,
    //     units: [u64; 4],
    //     token: String,
    //     amount: Uint128
    // },

    // ReleaseEscrowTimeout {
    //     message_hash: String,
    //     units: [u64; 4],
    //     token: String,
    //     amount: Uint128
    // },

    // ReleaseLiquidityEscrowAck {
    //     message_hash: String,
    //     units: [u64; 4],
    //     amount: [u64; 4]
    // },

    // ReleaseLiquidityEscrowTimeout {
    //     message_hash: String,
    //     units: [u64; 4],
    //     amount: [u64; 4]
    // },

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

    // Deposit { pool_tokens_amount: Uint128 },

    // Withdraw { pool_tokens_amount: Uint128 },

    // Localswap {
    //     from_asset: String,
    //     to_asset: String,
    //     amount: Uint128,
    //     min_out: Uint128,
    //     approx: bool
    // },

    // SwapToUnits {
    //     chain: u32,
    //     target_pool: String,
    //     target_user: String,
    //     from_asset: String,
    //     to_asset_index: u8,
    //     amount: Uint128,
    //     min_out: [u64; 4],
    //     approx: u8,
    //     fallback_address: String,
    //     calldata: Vec<u8>
    // },

    // SwapFromUnits {
    //     to_asset_index: u8,
    //     who: String,
    //     units: [u64; 4],
    //     min_out: [u64; 4],
    //     approx: bool,
    //     message_hash: String,
    //     data_target: String,
    //     // bytes calldata data // TODO vec<>?
    // },

    // OutLiquidity {
    //     chain: [u64; 4],
    //     target_pool: String,
    //     who: String,
    //     base_amount: [u64; 4],
    //     min_out: [u64; 4],
    //     approx: u8,
    //     fallback_user: String
    // },

    // InLiquidity {
    //     who: String,
    //     units: [u64; 4],
    //     min_out: [u64; 4],
    //     approx: bool,
    //     message_hash: String
    // }


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

    // #[returns(UnitCapacityResponse)]
    // UnitCapacity {},

    // #[returns(LiquidityUnitCapacityResponse)]
    // LiquidityUnitCapacity {},
    
    // #[returns(ChainInterfaceResponse)]
    // ChainInterface {},

    // // TokenIndexing(tokenIndex: [u64; 4]),

    // #[returns(IsLocalResponse)]
    // IsLocal {},

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

    // #[returns(IsReadyResponse)]
    // IsReady {},


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
    pub amount: [u64; 4],
}

#[cw_serde]
pub struct LiquidityUnitCapacityResponse {
    pub amount: [u64; 4],
}

#[cw_serde]
pub struct ChainInterfaceResponse {
    pub contract: String,
}

#[cw_serde]
pub struct IsLocalResponse {
    pub is_local: Binary,
}

#[cw_serde]
pub struct Balance0Response {
    pub balance: [u64; 4],
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
    pub amount: [u64; 4]
}

#[cw_serde]
pub struct PoolFeeX64Response {
    pub fee: [u64; 4]    //TODO use u64?
}

#[cw_serde]
pub struct GovernanceFeeResponse {
    pub fee: [u64; 4]    //TODO use u64?
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
    pub amount: [u64; 4]
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
pub struct IsReadyResponse {
    pub ready: Binary
}



