#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128, CosmosMsg, to_binary, StdError, SubMsg, Reply, SubMsgResult, StdResult, Uint64, Deps, Binary};
use cw0::parse_reply_instantiate_data;
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, OwnerResponse};
use crate::state::{set_default_governance_fee_share_unchecked, owner, default_governance_fee_share, save_deploy_vault_reply_args, DeployVaultReplyArgs, DEPLOY_VAULT_REPLY_ID, load_deploy_vault_reply_args};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:catalyst-vault-factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let set_default_fee_event = set_default_governance_fee_share_unchecked(
        &mut deps,
        msg.default_governance_fee
    )?;

    Ok(
        Response::new()
            .add_event(set_default_fee_event)
    )
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {

    match msg {
        ExecuteMsg::DeployVault {
            vault_template_id,
            assets,
            assets_balances,
            weights,
            amplification,
            pool_fee,
            name,
            symbol,
            chain_interface
        } => execute_deploy_vault(
            deps,
            info,
            vault_template_id,
            assets,
            assets_balances,
            weights,
            amplification,
            pool_fee,
            name,
            symbol,
            chain_interface               
        ),
    }
}

fn execute_deploy_vault(
    mut deps: DepsMut,
    info: MessageInfo,
    vault_contract_id: u64,
    assets: Vec<String>,
    assets_balances: Vec<Uint128>,
    weights: Vec<u64>,
    amplification: u64,
    pool_fee: u64,
    name: String,
    symbol: String,
    chain_interface: Option<String>
) -> Result<Response, ContractError> {

    // Verify the correctness of the arguments
    if
        assets.len() == 0
        || weights.len() != assets.len()
        || assets_balances.len() != assets.len()
    {
        return Err(StdError::generic_err("Invalid asset/balances/weights count.").into());
    }

    // Store the required args to initalize the vault curves for after the vault instantiation (used on 'reply')
    // TODO! verify the safety of passing information to the reply handler in the following way (reentrancy attacks?)
    save_deploy_vault_reply_args(
        deps.branch(),
        DeployVaultReplyArgs {
            assets,
            assets_balances,
            weights,
            amplification,
            depositor: info.sender.clone()
        }
    )?;

    // Create msg to instantiate a new vault
    let instantiate_vault_submsg = SubMsg::reply_on_success(
        cosmwasm_std::WasmMsg::Instantiate {
            admin: None,        //TODO set factory as admin?
            code_id: vault_contract_id,
            msg: to_binary(&swap_pool_common::msg::InstantiateMsg {
                name,
                symbol: symbol.clone(),
                chain_interface: chain_interface.clone(),
                pool_fee,
                governance_fee: default_governance_fee_share(deps.as_ref())?,
                fee_administrator: owner(deps.as_ref())?.ok_or(ContractError::Unauthorized {})?.to_string(),      //TODO error
                setup_master: info.sender.to_string()
            })?,
            funds: vec![],
            label: symbol.to_string()   //TODO review: what to set as 'label'?
        },
        DEPLOY_VAULT_REPLY_ID
    );

    //TODO implement IsCreatedByFactory?
    
    Ok(
        Response::new()
            .add_submessage(instantiate_vault_submsg)
            .add_attribute("vault_contract_id", Uint64::from(vault_contract_id))                //TODO EVM mismatch event name
            .add_attribute("chain_interface", chain_interface.unwrap_or("None".to_string()))    //TODO review 'chain_interface' format
            .add_attribute("deployer", info.sender)
    )
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    deps: DepsMut,
    _env: Env,
    reply: Reply
) -> Result<Response, ContractError> {
    match reply.id {
        DEPLOY_VAULT_REPLY_ID => match reply.result {
            SubMsgResult::Ok(_) => handle_deploy_vault_reply(deps, reply),
            SubMsgResult::Err(_) => Err(StdError::GenericErr { msg: "Vault deployment failed.".to_string() }.into())   // Reply is set to on_success, so the code should never reach this point
        },
        _ => Err(ContractError::UnknownReplyId { id: reply.id }),
    }
}

fn handle_deploy_vault_reply(
    deps: DepsMut,
    reply: Reply
) -> Result<Response, ContractError> {

    // Get deployed vault contract address
    let vault_address = parse_reply_instantiate_data(reply).map_err(|_| StdError::generic_err("Error deploying vault contract"))?.contract_address;

    // Load the deploy vault args
    let deploy_args = load_deploy_vault_reply_args(deps.as_ref())?;

    // Build messages to order the transfer of tokens from setup_master to the vault
    let transfer_msgs: Vec<CosmosMsg> = deploy_args.assets.iter()
        .zip(&deploy_args.assets_balances)
        .map(|(asset, balance)| {    // zip: assets_balances.len() == assets.len()
            Ok(CosmosMsg::Wasm(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: asset.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                        owner: deploy_args.depositor.to_string(),
                        recipient: vault_address.clone(),
                        amount: *balance
                    })?,
                    funds: vec![]
                }
            ))
        }).collect::<StdResult<Vec<CosmosMsg>>>()?;

    // Build message to invoke vault initialize swap curves
    let initialize_swap_curves_msg = CosmosMsg::Wasm(
        cosmwasm_std::WasmMsg::Execute {
            contract_addr: vault_address.clone(),
            msg: to_binary(&swap_pool_common::msg::ExecuteMsg::<()>::InitializeSwapCurves {
                assets: deploy_args.assets.clone(),
                weights: deploy_args.weights,
                amp: deploy_args.amplification,
                depositor: deploy_args.depositor.to_string()
            })?,
            funds: vec![]
        }
    );


    //TODO event
    Ok(
        Response::new()
            .add_messages(transfer_msgs)
            .add_message(initialize_swap_curves_msg)
            .add_attribute("vault", vault_address)                                      //TODO EVM mismatch: key name (pool_address)
            .add_attribute("assets", deploy_args.assets.join(", "))
            .add_attribute("amplification", Uint64::from(deploy_args.amplification))    //TODO EVM mismatch: key name (k)
    )

}



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Owner {} => to_binary(&query_owner(deps)?)
    }
}

fn query_owner(deps: Deps) -> StdResult<OwnerResponse> {
    Ok(
        OwnerResponse {
            owner: owner(deps).map_err(|_| StdError::generic_err("Query owner error."))?    //TODO error
        }
    )
}