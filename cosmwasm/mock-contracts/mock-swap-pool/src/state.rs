use cosmwasm_std::{Addr, Uint128, DepsMut, Env, MessageInfo, Response, StdResult, Deps, StdError, Uint64};
use cw20::{Cw20QueryMsg, BalanceResponse};
use cw20_base::contract::execute_mint;
use swap_pool_common::{
    state::{ASSETS, MAX_ASSETS, WEIGHTS, INITIAL_MINT_AMOUNT, FACTORY, factory_owner, CHAIN_INTERFACE, SETUP_MASTER}, ContractError, msg::{ChainInterfaceResponse, SetupMasterResponse, FactoryResponse, FactoryOwnerResponse},
};


pub fn initialize_swap_curves(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    assets: Vec<String>,
    weights: Vec<Uint64>,
    _amp: Uint64,
    depositor: String
) -> Result<Response, ContractError> {

    // Check the caller is the Factory    //TODO does this make sense? Unlike on EVM, the 'factory' is not set as 'immutable', but rather it is set as the caller of 'instantiate'
    if info.sender != FACTORY.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    // Make sure this function may only be invoked once (check whether assets have already been saved)
    if ASSETS.may_load(deps.storage) != Ok(None) {
        return Err(ContractError::Unauthorized {});
    }

    // Check the provided assets, assets balances and weights count
    if
        assets.len() == 0 || assets.len() > MAX_ASSETS ||
        weights.len() != assets.len()
    {
        return Err(ContractError::GenericError {}); //TODO error
    }

    // Validate the depositor address
    deps.api.addr_validate(&depositor)?;

    // Validate and save assets
    ASSETS.save(
        deps.storage,
        &assets
            .iter()
            .map(|asset_addr| deps.api.addr_validate(&asset_addr))
            .collect::<StdResult<Vec<Addr>>>()
            .map_err(|_| ContractError::InvalidAssets {})?
    )?;

    // Query and validate the vault asset balances
    let assets_balances = assets.iter()
        .map(|asset| {
            deps.querier.query_wasm_smart::<BalanceResponse>(
                asset,
                &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
            ).map(|response| response.balance)
        })
        .collect::<StdResult<Vec<Uint128>>>()?;
    
    if assets_balances.iter().any(|balance| balance.is_zero()) {
        return Err(ContractError::GenericError {}); //TODO error
    }

    // Validate and save weights
    if weights.iter().any(|weight| *weight == Uint64::zero()) {
        return Err(ContractError::GenericError {}); //TODO error
    }
    WEIGHTS.save(deps.storage, &weights)?;

    // Mint pool tokens for the depositor
    // Make up a 'MessageInfo' with the sender set to this contract itself => this is to allow the use of the 'execute_mint'
    // function as provided by cw20-base, which will match the 'sender' of 'MessageInfo' with the allowed minter that
    // was set when initializing the cw20 token (this contract itself).
    let execute_mint_info = MessageInfo {
        sender: env.contract.address.clone(),
        funds: vec![],
    };
    let minted_amount = INITIAL_MINT_AMOUNT;
    execute_mint(
        deps.branch(),
        env.clone(),
        execute_mint_info,
        depositor.clone(),
        minted_amount
    )?;

    //TODO include attributes of the execute_mint response in this response?
    Ok(
        Response::new()
            .add_attribute("to_account", depositor)
            .add_attribute("mint", minted_amount)
            .add_attribute("assets", format_vec_for_event(assets_balances))
    )
}





// Query helpers ****************************************************************************************************************

pub fn query_chain_interface(deps: Deps) -> StdResult<ChainInterfaceResponse> {
    Ok(
        ChainInterfaceResponse {
            chain_interface: CHAIN_INTERFACE.load(deps.storage)?
        }
    )
}

pub fn query_setup_master(deps: Deps) -> StdResult<SetupMasterResponse> {
    Ok(
        SetupMasterResponse {
            setup_master: SETUP_MASTER.load(deps.storage)?
        }
    )
}

pub fn query_factory(deps: Deps) -> StdResult<FactoryResponse> {
    Ok(
        FactoryResponse {
            factory: FACTORY.load(deps.storage)?
        }
    )
}

pub fn query_factory_owner(deps: Deps) -> StdResult<FactoryOwnerResponse> {
    Ok(
        FactoryOwnerResponse {
            factory_owner: factory_owner(&deps).map_err(|_| StdError::generic_err("Unable to get factory_owner."))?
        }
    )
}




// Misc helpers *****************************************************************************************************************
//TODO move helper somewhere else? (To reuse across implementations)
pub fn format_vec_for_event<T: ToString>(vec: Vec<T>) -> String {
    //TODO review output format
    vec
        .iter()
        .map(T::to_string)
        .collect::<Vec<String>>().join(", ")
}