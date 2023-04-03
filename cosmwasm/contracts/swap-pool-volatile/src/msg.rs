
use cosmwasm_schema::cw_serde;
pub use swap_pool_common::msg::InstantiateMsg;
pub use swap_pool_common::msg::ExecuteMsg;
pub use swap_pool_common::msg::QueryMsg;


#[cw_serde]
pub enum VolatileExecuteExtension {

    SetWeights {
        weights: Vec<u64>,      //TODO EVM mismatch (name newWeights)
        target_timestamp: u64   //TODO EVM mismatch (targetTime)
    },

}

pub type VolatileExecuteMsg = ExecuteMsg<VolatileExecuteExtension>;