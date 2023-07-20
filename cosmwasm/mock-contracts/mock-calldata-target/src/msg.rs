use cosmwasm_schema::QueryResponses;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct InstantiateMsg {
}

#[cw_serde]
pub enum ExecuteMsg {

    OnCatalystCall {
        purchased_tokens: Uint128,
        data: Binary
    }

}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
}
