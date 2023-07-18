#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, to_binary};
use cw2::set_contract_version;
use cw20_base::contract::query_token_info;
use catalyst_vault_common::ContractError;
use catalyst_vault_common::msg::ExecuteMsg;
use catalyst_vault_common::state::{setup, query_assets, query_weight, query_vault_fee, query_governance_fee_share, query_fee_administrator, query_chain_interface, query_setup_master, query_factory, query_factory_owner};

use crate::msg::{InstantiateMsg, QueryMsg};
use crate::state::initialize_swap_curves;

// version info for migration info
const CONTRACT_NAME: &str = "catalyst-mock-vault";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg
) -> Result<Response, ContractError> {

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    setup(
        &mut deps,
        &env,
        info,
        msg.name,
        msg.symbol,
        msg.chain_interface,
        msg.vault_fee,
        msg.governance_fee_share,
        msg.fee_administrator,
        msg.setup_master
    )

}



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<()>,
) -> Result<Response, ContractError> {
    match msg {

        ExecuteMsg::<()>::InitializeSwapCurves {
            assets,
            weights,
            amp,
            depositor
        } => initialize_swap_curves(
            &mut deps,
            env,
            info,
            assets,
            weights,
            amp,
            depositor
        ),

        _ => unimplemented!()
    }
}



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {

        // Only define the queries needed to verify that initialization of the vault has been successful
        QueryMsg::ChainInterface {} => to_binary(&query_chain_interface(deps)?),
        QueryMsg::SetupMaster {} => to_binary(&query_setup_master(deps)?),
        QueryMsg::Factory {} => to_binary(&query_factory(deps)?),
        QueryMsg::FactoryOwner {} => to_binary(&query_factory_owner(deps)?),
        QueryMsg::Assets {} => to_binary(&query_assets(deps)?),
        QueryMsg::Weight {
            asset
        } => to_binary(&query_weight(deps, asset)?),
        QueryMsg::VaultFee {} => to_binary(&query_vault_fee(deps)?),
        QueryMsg::GovernanceFeeShare {} => to_binary(&query_governance_fee_share(deps)?),
        QueryMsg::FeeAdministrator {} => to_binary(&query_fee_administrator(deps)?),
        QueryMsg::TokenInfo {} => to_binary(&query_token_info(deps)?),
        _ => unimplemented!()
    }
}
