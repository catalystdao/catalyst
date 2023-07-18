use cosmwasm_std::{Addr, Uint128, DepsMut, Env, MessageInfo, Response, Uint64};
use cw20::{Cw20QueryMsg, BalanceResponse};
use cw20_base::contract::execute_mint;
use catalyst_vault_common::{
    state::{ASSETS, MAX_ASSETS, WEIGHTS, INITIAL_MINT_AMOUNT, FACTORY}, ContractError, event::{deposit_event, cw20_response_to_standard_event},
};


pub fn initialize_swap_curves(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    assets: Vec<String>,
    weights: Vec<Uint128>,
    _amp: Uint64,
    depositor: String
) -> Result<Response, ContractError> {

    // Check the caller is the Factory
    if info.sender != FACTORY.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    // Make sure this function may only be invoked once (check whether assets have already been saved)
    if ASSETS.may_load(deps.storage) != Ok(None) {
        return Err(ContractError::Unauthorized {});
    }

    // Check the provided assets, assets balances and weights count
    if assets.len() == 0 || assets.len() > MAX_ASSETS {
        return Err(ContractError::InvalidAssets {});
    }

    if weights.len() != assets.len() {
        return Err(ContractError::InvalidParameters {
            reason: "Invalid weights count.".to_string()
        });
    }

    // Query and validate the vault asset balances
    let assets_balances = assets.iter()
        .map(|asset| {

            let balance = deps.querier.query_wasm_smart::<BalanceResponse>(
                asset,
                &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
            )?.balance;

            if balance.is_zero() {
                return Err(ContractError::InvalidZeroBalance {});
            }

            Ok(balance)
        })
        .collect::<Result<Vec<Uint128>, ContractError>>()?;

    // Save the assets
    // NOTE: there is no need to validate the assets addresses, as invalid asset addresses
    // would have caused the previous 'asset balance' check to fail.
    ASSETS.save(
        deps.storage,
        &assets
            .iter()
            .map(|asset| Addr::unchecked(asset))
            .collect::<Vec<Addr>>()
    )?;

    // Validate and save weights
    weights
        .iter()
        .zip(&assets)
        .try_for_each(|(weight, asset)| -> Result<(), ContractError> {

            if weight.is_zero() {
                return Err(ContractError::InvalidWeight {});
            }

            WEIGHTS.save(deps.storage, asset, weight)?;
            
            Ok(())
        })?;

    // Mint vault tokens for the depositor
    // Make up a 'MessageInfo' with the sender set to this contract itself => this is to allow the use of the 'execute_mint'
    // function as provided by cw20-base, which will match the 'sender' of 'MessageInfo' with the allowed minter that
    // was set when initializing the cw20 token (this contract itself).
    let execute_mint_info = MessageInfo {
        sender: env.contract.address.clone(),
        funds: vec![],
    };
    let minted_amount = INITIAL_MINT_AMOUNT;
    let mint_response = execute_mint(
        deps.branch(),
        env.clone(),
        execute_mint_info,
        depositor.clone(),
        minted_amount
    )?;

    Ok(
        Response::new()
            .add_event(
                deposit_event(
                    depositor,
                    minted_amount,
                    assets_balances
                )
            )
            .add_event(
                cw20_response_to_standard_event(
                    mint_response
                )
            )
    )
}
