use cosmwasm_std::{Addr, Uint128, DepsMut, Env, MessageInfo, Response, StdResult, CosmosMsg, to_binary, Deps, Binary, Uint64};
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg, BalanceResponse};
use cw20_base::{contract::{execute_mint, execute_burn}};
use cw_storage_plus::Item;
use catalyst_types::{U256, AsI256, I256, AsU256, u256};
use fixed_point_math::{mul_wad_down, self, WAD, pow_wad, WADWAD, div_wad_up, div_wad_down, mul_wad_up};
use catalyst_vault_common::{
    state::{
        ASSETS, MAX_ASSETS, WEIGHTS, INITIAL_MINT_AMOUNT, VAULT_FEE, MAX_LIMIT_CAPACITY, USED_LIMIT_CAPACITY, CHAIN_INTERFACE,
        TOTAL_ESCROWED_LIQUIDITY, TOTAL_ESCROWED_ASSETS, is_connected, update_limit_capacity,
        collect_governance_fee_message, compute_send_asset_hash, compute_send_liquidity_hash, create_asset_escrow,
        create_liquidity_escrow, on_send_asset_success, total_supply, get_limit_capacity, USED_LIMIT_CAPACITY_TIMESTAMP, FACTORY, on_send_asset_failure, on_send_liquidity_failure, factory_owner,
    },
    ContractError, msg::{CalcSendAssetResponse, CalcReceiveAssetResponse, CalcLocalSwapResponse, GetLimitCapacityResponse}, event::{local_swap_event, send_asset_event, receive_asset_event, send_liquidity_event, receive_liquidity_event, deposit_event, withdraw_event}
};
use std::ops::Div;

use catalyst_ibc_interface::msg::ExecuteMsg as InterfaceExecuteMsg;

use crate::{calculation_helpers::{calc_price_curve_area, calc_price_curve_limit, calc_combined_price_curves, calc_price_curve_limit_share, calc_weighted_alpha_0_ampped}, event::set_amplification_event, msg::{TargetAmplificationResponse, AmplificationUpdateFinishTimestampResponse, Balance0Response}};

// TODO amplification specific storage
pub const ONE_MINUS_AMP: Item<I256> = Item::new("catalyst-vault-amplified-one-minus-amp");
pub const TARGET_ONE_MINUS_AMP: Item<I256> = Item::new("catalyst-vault-amplified-target-one-minus-amp");
pub const AMP_UPDATE_TIMESTAMP: Item<Uint64> = Item::new("catalyst-vault-amplified-amp-update-timestamp");
pub const AMP_UPDATE_FINISH_TIMESTAMP: Item<Uint64> = Item::new("catalyst-vault-amplified-amp-update-finish-timestamp");
pub const UNIT_TRACKER: Item<I256> = Item::new("catalyst-vault-amplified-unit-tracker");

const MIN_ADJUSTMENT_TIME_NANOS : Uint64 = Uint64::new(7 * 24 * 60 * 60 * 1000000000);     // 7 days
const MAX_ADJUSTMENT_TIME_NANOS : Uint64 = Uint64::new(365 * 24 * 60 * 60 * 1000000000);   // 1 year
const MAX_AMP_ADJUSTMENT_FACTOR : Uint64 = Uint64::new(2);

const SMALL_SWAP_RATIO  : Uint128 = Uint128::new(1000000000000u128);   // 1e12
const SMALL_SWAP_RETURN : U256 = u256!("950000000000000000");           // 0.95 * WAD


pub fn initialize_swap_curves(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    assets: Vec<String>,
    weights: Vec<Uint64>,
    amp: Uint64,
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

    // Check that the amplification is correct (set to < 1)
    if amp >= Uint64::new(10u64.pow(18)) {
        return Err(ContractError::InvalidAmplification {})
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

    // Save the amplification value. It is stored as 1 - amp since most equations uses amp this way.
    let one_minus_amp = WAD.as_i256().wrapping_sub(I256::from(amp));
    ONE_MINUS_AMP.save(deps.storage, &one_minus_amp)?;
    TARGET_ONE_MINUS_AMP.save(deps.storage, &one_minus_amp)?;

    // Validate the depositor address
    deps.api.addr_validate(&depositor)?;    //TODO is this needed? Won't the address be validated by 'execute_mint` below?

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
    
    //TODO merge this check within the above balance-query code
    if assets_balances.iter().any(|balance| balance.is_zero()) {
        return Err(ContractError::InvalidZeroBalance {});
    }

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
        
    AMP_UPDATE_TIMESTAMP.save(deps.storage, &Uint64::zero())?;         //TODO move intialization to 'setup'?
    AMP_UPDATE_FINISH_TIMESTAMP.save(deps.storage, &Uint64::zero())?;  //TODO move intialization to 'setup'?

    // Compute the security limit
    MAX_LIMIT_CAPACITY.save(
        deps.storage,
        &(weights
            .iter()
            .zip(&assets_balances)
            .fold(
                U256::zero(),
                |acc, (next_weight, next_balance)| {
                    acc + U256::from(*next_weight).wrapping_mul(U256::from(*next_balance))     // Overflow safe, as U256 >> u64*Uint128
                }
            ))
    )?;
    USED_LIMIT_CAPACITY.save(deps.storage, &U256::zero())?;       //TODO move intialization to 'setup'?
    USED_LIMIT_CAPACITY_TIMESTAMP.save(deps.storage, &Uint64::zero())?;   //TODO move intialization to 'setup'?

    // Initialize escrow totals
    assets
        .iter()
        .map(|asset| TOTAL_ESCROWED_ASSETS.save(deps.storage, asset, &Uint128::zero()))
        .collect::<StdResult<Vec<_>>>()?;
    TOTAL_ESCROWED_LIQUIDITY.save(deps.storage, &Uint128::zero())?;

    // Initialize the unit tracker
    UNIT_TRACKER.save(deps.storage, &I256::zero())?;

    // Mint vault tokens for the depositor
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
            .add_event(
                deposit_event(
                    depositor,
                    minted_amount,
                    assets_balances
                )
            )
    )
}




pub fn deposit_mixed(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    deposit_amounts: Vec<Uint128>,  //TODO EVM MISMATCH
    min_out: Uint128
) -> Result<Response, ContractError> {

    update_amplification(deps, env.block.time.nanos().into())?;

    let one_minus_amp = ONE_MINUS_AMP.load(deps.storage)?;

    let assets = ASSETS.load(deps.storage)?;

    if deposit_amounts.len() != assets.len() {
        return Err(
            ContractError::InvalidParameters{
                reason: "Invalid deposit_amounts count.".to_string()
            }
        );
    }

    // Compute how much 'units' the assets are worth.
    // Iterate over the assets, weights and deposit_amounts
    let mut units: U256 = U256::zero();                             // EVM mismatch: variable is *signed* on EVM (because of stack issues)
    let mut weighted_asset_balance_ampped_sum: U256 = U256::zero();
    let mut weighted_deposit_sum: U256 = U256::zero();              // NOTE: named 'assetDepositSum' on EVM

    assets.iter()
        .zip(&deposit_amounts)          // zip: deposit_amounts.len() == assets.len()
        .try_for_each(|(asset, deposit_amount)| -> Result<_, ContractError> {

            let weight = WEIGHTS.load(deps.storage, asset.as_ref())?;

            let vault_asset_balance = deps.querier.query_wasm_smart::<BalanceResponse>(
                asset,
                &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
            )?.balance;

            let weighted_asset_balance = U256::from(vault_asset_balance)
                .wrapping_mul(weight.into());           // 'wrapping_mul' is safe as U256.max >= Uint128.max * u64.max

            let weighted_deposit = U256::from(*deposit_amount)
                .wrapping_mul(weight.into());           // 'wrapping_mul' is safe as U256.max >= Uint128.max * u64.max


            // Compute wa^(1-k) (WAD). Handle the case a=0 separatelly, as the implemented numerical calculation
            // will fail for that case.
            let weighted_asset_balance_ampped;    // EVM mismatch: variable is *signed* on EVM (because of stack issues)
                                                        // NOTE: named 'wab' on EVM
            if weighted_asset_balance.is_zero() {
                weighted_asset_balance_ampped = U256::zero();
            }
            else {
                weighted_asset_balance_ampped = pow_wad(
                    weighted_asset_balance.wrapping_mul(WAD).as_i256(), // 'wrapping_mul' is safe as U256.max >= Uint128.max * u64.max * WAD
                    one_minus_amp
                )?.as_u256();
                weighted_asset_balance_ampped_sum = weighted_asset_balance_ampped_sum.checked_add(weighted_asset_balance_ampped)?;
            }

            // Stop if the user provides no tokens for the specific asset (save gas)
            if deposit_amount.is_zero() {
                return Ok(());
            }

            // Cache the weighted deposit sum to later update the security limit
            weighted_deposit_sum = weighted_deposit_sum.checked_add(weighted_deposit)?;

            // Compute the units corresponding to the asset in question: F(a+input) - F(a)
            // where F(x) = x^(1-k)
            let units_for_asset = pow_wad(
                weighted_asset_balance
                    .checked_add(weighted_deposit)?
                    .checked_mul(WAD)?
                    .as_i256(),
                one_minus_amp
            )?.as_u256().wrapping_sub(weighted_asset_balance_ampped);

            units = units.checked_add(units_for_asset)?;

            Ok(())
        }
    )?;

    // Update the security limit variables  //TODO create helper functions for these?
    MAX_LIMIT_CAPACITY.update(
        deps.storage,
        |max_limit_capacity| -> StdResult<_> {
            max_limit_capacity
                .checked_add(weighted_deposit_sum)
                .map_err(|err| err.into())
        }
    )?;

    USED_LIMIT_CAPACITY.update(
        deps.storage,
        |used_limit_capacity| -> StdResult<_> {
            used_limit_capacity
                .checked_add(weighted_deposit_sum)          // ! Addition must be checked as 'used_limit_capacity' may be larger than 'max_limit_capacity'
                .map_err(|err| err.into())
        }
    )?;

    // Compute the reference liquidity
    let weighted_alpha_0_ampped: U256 = weighted_asset_balance_ampped_sum.as_i256()
        .wrapping_sub(UNIT_TRACKER.load(deps.storage)?)
        .as_u256()
        .div(U256::from(assets.len() as u64));

    //TODO check for intU >= 0 as in EVM?

    // Subtract the vault fee from U to prevent deposit and withdrawals being employed as a method of swapping.
    // To recude costs, the governance fee is not taken. This is not an issue as swapping via this method is 
    // disincentivized by its higher gas costs.
    let vault_fee = VAULT_FEE.load(deps.storage)?;
    let units = fixed_point_math::mul_wad_down(units, fixed_point_math::WAD.wrapping_sub(vault_fee.into()))?;   //TODO EVM mismatch

    // Do not include the 'escrowed' vault tokens in the total supply of vault tokens (return less)
    let effective_supply = U256::from(total_supply(deps.as_ref())?);

    // Compute the vault tokens to be minted.
    let out: Uint128 = calc_price_curve_limit_share(
        units,
        effective_supply,
        weighted_alpha_0_ampped.checked_mul((assets.len() as u64).into())?,
        WADWAD / one_minus_amp
    )?.try_into()?;

    // Check that the minimum output is honoured.
    if min_out > out {
        return Err(ContractError::ReturnInsufficient { out, min_out });
    }

    // Mint the vault tokens
    let mint_response = execute_mint(
        deps.branch(),
        env.clone(),
        MessageInfo {
            sender: env.contract.address.clone(),   // This contract itself is the one 'sending' the mint operation
            funds: vec![],
        },
        info.sender.to_string(),
        out
    )?;

    // Build messages to order the transfer of tokens from the depositor to the vault
    let transfer_msgs: Vec<CosmosMsg> = assets.iter()
        .zip(&deposit_amounts)                                              // zip: depsoit_amounts.len() == assets.len()
        .filter(|(_, balance)| **balance != Uint128::zero())     // Do not create transfer messages for zero-valued deposits
        .map(|(asset, balance)| {
            Ok(CosmosMsg::Wasm(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: asset.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                        owner: info.sender.to_string(),
                        recipient: env.contract.address.to_string(),
                        amount: *balance
                    })?,
                    funds: vec![]
                }
            ))
        })
        .collect::<StdResult<Vec<CosmosMsg>>>()?;

    Ok(Response::new()
        .add_messages(transfer_msgs)
        .add_events(mint_response.events)                           // Add mint events //TODO overhaul
        .add_event(
            deposit_event(
                info.sender.to_string(),
                out,
                deposit_amounts
            )
        )
    )
}

pub fn withdraw_all(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    vault_tokens: Uint128,
    min_out: Vec<Uint128>,
) -> Result<Response, ContractError> {

    update_amplification(deps, env.block.time.nanos().into())?;

    // Burn the vault tokens of the withdrawer
    let sender = info.sender.to_string();
    let burn_response = execute_burn(deps.branch(), env.clone(), info.clone(), vault_tokens)?;

    // Compute the withdraw amounts
    let assets = ASSETS.load(deps.storage)?;
    let one_minus_amp = ONE_MINUS_AMP.load(deps.storage)?;
    
    let weights = assets
        .iter()
        .map(|asset| WEIGHTS.load(deps.storage, asset.as_ref()))
        .collect::<StdResult<Vec<Uint64>>>()?;

    if min_out.len() != assets.len() {
        return Err(
            ContractError::InvalidParameters {
                reason: "Invalid min_out count.".to_string()
            }
        );
    }

    let mut weighted_asset_balance_ampped_sum: U256 = U256::zero();

    let effective_weighted_asset_balances = assets.iter()
        .zip(&weights)
        .map(|(asset, weight)| -> Result<U256, ContractError> {

            let vault_asset_balance = deps.querier.query_wasm_smart::<BalanceResponse>(
                asset,
                &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
            )?.balance;

            if !vault_asset_balance.is_zero() {

                let escrowed_asset_balance = TOTAL_ESCROWED_ASSETS.load(deps.storage, &asset.to_string())?;

                let weighted_asset_balance = U256::from(vault_asset_balance)
                    .wrapping_mul((*weight).into());           // 'wrapping_mul' is safe as U256.max >= Uint128.max * u64.max
    
                let weighted_asset_balance_ampped = pow_wad(
                    weighted_asset_balance.wrapping_mul(WAD).as_i256(), // 'wrapping_mul' is safe as U256.max >= Uint128.max * u64.max * WAD
                    one_minus_amp
                )?.as_u256();

                weighted_asset_balance_ampped_sum = weighted_asset_balance_ampped_sum.checked_add(weighted_asset_balance_ampped)?;

                Ok(
                    weighted_asset_balance.wrapping_sub(
                        U256::from(escrowed_asset_balance).wrapping_mul((*weight).into())
                    )
                )
            }
            else {
                Ok(U256::zero())
            }

        })
        .collect::<Result<Vec<U256>, ContractError>>()?;

    // Compute the reference liquidity
    let weighted_alpha_0_ampped: U256 = weighted_asset_balance_ampped_sum.as_i256()
        .wrapping_sub(UNIT_TRACKER.load(deps.storage)?)
        .as_u256()
        .div(U256::from(assets.len() as u64));

    // Compute the effective supply. Include 'vault_tokens' to the queried supply as these have already been burnt
    // and also include the escrowed tokens to yield a smaller return
    let effective_supply = U256::from(total_supply(deps.as_ref())?)
        .wrapping_add(vault_tokens.into())                                      // 'wrapping_add' is safe as U256.max >> Uint128.max
        .wrapping_add(TOTAL_ESCROWED_LIQUIDITY.load(deps.storage)?.into());     // 'wrapping_add' is safe as U256.max >> Uint128.max

    // Compute 'supply after withdrawal'/'supply before withdrawal' (in WAD terms)
    let vault_tokens_share = effective_supply
        .wrapping_sub(vault_tokens.into())          // 'wrapping_sub' is safe as the 'vault_tokens' have been successfully burnt at the beginning of this function
        .wrapping_mul(WAD)                          // 'wrapping_mul' is safe as U256.max > Uint128.max * WAD
        .div(effective_supply);

    let inner_diff = mul_wad_down(
        weighted_alpha_0_ampped,
        WAD.wrapping_sub(
            pow_wad(
                vault_tokens_share.as_i256(),       // Casting is safe as 'vault_tokens_share' <= 1
                one_minus_amp
            )?.as_u256()                            // Casting is safe as 'pow_wad' result is >= 0
        )
    )?;

    let one_minus_amp_inverse = WADWAD / one_minus_amp;

    let mut weighted_withdraw_sum = U256::zero();
    let withdraw_amounts: Vec<Uint128> = weights
        .iter()
        .zip(&min_out)                                      // zip: min_out.len() == weights.len()
        .zip(effective_weighted_asset_balances)             // zip: effective_weighted_asset_balances.len() == weights.len()
        .map(|((weight, asset_min_out), effective_weighted_asset_balance)| {

            let effective_weighted_asset_balance_ampped = pow_wad(
                effective_weighted_asset_balance.wrapping_mul(WAD).as_i256(), // 'wrapping_mul' is safe as U256.max >= Uint128.max * u64.max * WAD
                one_minus_amp
            )?.as_u256();

            let weighted_withdraw_amount;
            if inner_diff < effective_weighted_asset_balance_ampped {
                weighted_withdraw_amount = mul_wad_down(
                    effective_weighted_asset_balance,
                    WAD.wrapping_sub(
                        pow_wad(
                            div_wad_up(
                                effective_weighted_asset_balance_ampped.wrapping_sub(inner_diff),
                                effective_weighted_asset_balance_ampped
                            )?.as_i256(),
                            one_minus_amp_inverse
                        )?.as_u256()
                    )
                )?
            }
            else {
                weighted_withdraw_amount = effective_weighted_asset_balance_ampped;
            }

            weighted_withdraw_sum = weighted_withdraw_sum.checked_add(weighted_withdraw_amount)?;
            let withdraw_amount: Uint128 = (weighted_withdraw_amount / U256::from(*weight)).try_into()?;

            // Check that the minimum output is honoured.
            if *asset_min_out > withdraw_amount {
                return Err(ContractError::ReturnInsufficient { out: withdraw_amount.clone(), min_out: *asset_min_out });
            };

            Ok(withdraw_amount)
        }).collect::<Result<Vec<Uint128>, ContractError>>()?;

    //TODO use helper methods for the following updates?
    // Update the security limit variables
    MAX_LIMIT_CAPACITY.update(
        deps.storage,
        |max_limit_capacity| -> StdResult<_> {
            max_limit_capacity
                .checked_sub(weighted_withdraw_sum)         //TODO use wrapping_sub?
                .map_err(|err| err.into())
        }
    )?;

    USED_LIMIT_CAPACITY.update(
        deps.storage,
        |used_limit_capacity| -> StdResult<_> {
            Ok(
                used_limit_capacity.saturating_sub(weighted_withdraw_sum)
            )
        }
    )?;

    // Build messages to order the transfer of tokens from the vault to the depositor
    let transfer_msgs: Vec<CosmosMsg> = assets.iter().zip(&withdraw_amounts).map(|(asset, amount)| {    // zip: withdraw_amounts.len() == assets.len()
        Ok(CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: asset.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: sender.clone(),
                    amount: *amount
                })?,
                funds: vec![]
            }
        ))
    }).collect::<StdResult<Vec<CosmosMsg>>>()?;


    Ok(Response::new()
        .add_messages(transfer_msgs)
        .add_events(burn_response.events)                           // Add burn events //TODO overhaul
        .add_event(
            withdraw_event(
                info.sender.to_string(),
                vault_tokens,
                withdraw_amounts
            )
        )
    )
    
}


pub fn withdraw_mixed(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    vault_tokens: Uint128,
    withdraw_ratio: Vec<Uint64>,
    min_out: Vec<Uint128>,
) -> Result<Response, ContractError> {

    update_amplification(deps, env.block.time.nanos().into())?;

    // Burn the vault tokens of the withdrawer
    let sender = info.sender.to_string();
    let burn_response = execute_burn(deps.branch(), env.clone(), info.clone(), vault_tokens)?;

    // Compute the withdraw amounts
    let assets = ASSETS.load(deps.storage)?;
    let one_minus_amp = ONE_MINUS_AMP.load(deps.storage)?;
    
    let weights = assets
        .iter()
        .map(|asset| WEIGHTS.load(deps.storage, asset.as_ref()))
        .collect::<StdResult<Vec<Uint64>>>()?;

    // Compute the unit worth of the vault tokens.
    let mut weighted_asset_balance_ampped_sum: U256 = U256::zero();
    let effective_asset_balances = assets.iter()
        .zip(&weights)
        .map(|(asset, weight)| -> Result<Uint128, ContractError> {
            
            let vault_asset_balance = deps.querier.query_wasm_smart::<BalanceResponse>(
                asset,
                &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
            )?.balance;

            if !vault_asset_balance.is_zero() {

                let weighted_asset_balance = U256::from(vault_asset_balance)
                    .wrapping_mul((*weight).into());           // 'wrapping_mul' is safe as U256.max >= Uint128.max * u64.max
    
                let weighted_asset_balance_ampped = pow_wad(
                    weighted_asset_balance.wrapping_mul(WAD).as_i256(), // 'wrapping_mul' is safe as U256.max >= Uint128.max * u64.max * WAD
                    one_minus_amp
                )?.as_u256();

                weighted_asset_balance_ampped_sum = weighted_asset_balance_ampped_sum.checked_add(weighted_asset_balance_ampped)?;

                Ok(
                    vault_asset_balance.wrapping_sub(
                        TOTAL_ESCROWED_ASSETS.load(deps.storage, &asset.to_string())?
                    )
                )
            }
            else {
                Ok(Uint128::zero())
            }
        })
        .collect::<Result<Vec<Uint128>, ContractError>>()?;

    // Compute the reference liquidity
    let weighted_alpha_0_ampped: U256 = weighted_asset_balance_ampped_sum.as_i256()
        .wrapping_sub(UNIT_TRACKER.load(deps.storage)?)
        .as_u256()
        .div(U256::from(assets.len() as u64));

    // Compute the effective supply. Include 'vault_tokens' to the queried supply as these have already been burnt
    // and also include the escrowed tokens to yield a smaller return
    let effective_supply = U256::from(total_supply(deps.as_ref())?)
        .wrapping_add(vault_tokens.into())                                      // 'wrapping_add' is safe as U256.max >> Uint128.max
        .wrapping_add(TOTAL_ESCROWED_LIQUIDITY.load(deps.storage)?.into());     // 'wrapping_add' is safe as U256.max >> Uint128.max

    // Compute 'supply after withdrawal'/'supply before withdrawal' (in WAD terms)
    let vault_tokens_share = div_wad_down(
        effective_supply.wrapping_sub(vault_tokens.into()),  // 'wrapping_sub' is safe as the 'vault_tokens' have been successfully burnt at the beginning of this function
        effective_supply
    )?;

    let mut units = U256::from(assets.len() as u64).checked_mul(
        mul_wad_down(
            weighted_alpha_0_ampped,
            WAD.wrapping_sub(
                pow_wad(
                    vault_tokens_share.as_i256(),
                    one_minus_amp
                )?.as_u256()
            )
        )?
    )?;

    if withdraw_ratio.len() != assets.len() || min_out.len() != assets.len() {
        return Err(
            ContractError::InvalidParameters {
                reason: "Invalid withdraw_ratio/min_out count.".to_string()
            }
        );
    }

    let mut weighted_withdraw_sum = U256::zero();
    let withdraw_amounts: Vec<Uint128> = weights
        .iter()
        .zip(&withdraw_ratio)               // zip: withdraw_ratio.len() == weights.len()
        .zip(&min_out)                      // zip: min_out.len() == weights.len()
        .zip(effective_asset_balances)      // zip: effective_asset_balances.len() == weights.len()
        .map(|(((weight, asset_withdraw_ratio), asset_min_out), effective_asset_balance)| {

            // Calculate the units allocated for the specific asset
            let units_for_asset = fixed_point_math::mul_wad_down(units, U256::from(*asset_withdraw_ratio))?;
            if units_for_asset == U256::zero() {

                // There should not be a non-zero withdraw ratio after a withdraw ratio of 1 (protect against user error)
                if *asset_withdraw_ratio != Uint64::zero() {
                    return Err(ContractError::WithdrawRatioNotZero { ratio: *asset_withdraw_ratio }) 
                };

                // Check that the minimum output is honoured.
                if asset_min_out != Uint128::zero() {
                    return Err(ContractError::ReturnInsufficient { out: Uint128::zero(), min_out: *asset_min_out })
                };

                return Ok(Uint128::zero());
            }

            // Subtract the units used from the total units amount. This will underflow for malicious withdraw ratios (i.e. ratios > 1).
            units = units.checked_sub(units_for_asset)?;

            // Calculate the asset amount corresponding to the asset units
            let withdraw_amount = calc_price_curve_limit(
                units_for_asset,
                U256::from(effective_asset_balance),
                U256::from(*weight),
                one_minus_amp
            )?.try_into()?;

            // Check that the minimum output is honoured.
            if *asset_min_out > withdraw_amount {
                return Err(ContractError::ReturnInsufficient { out: withdraw_amount.clone(), min_out: *asset_min_out });
            };

            weighted_withdraw_sum = weighted_withdraw_sum.checked_add(
                U256::from(withdraw_amount).checked_mul((*weight).into())?
            )?;

            Ok(withdraw_amount)
        }).collect::<Result<Vec<Uint128>, ContractError>>()?;

    // Make sure all units have been consumed
    if units != U256::zero() { return Err(ContractError::UnusedUnitsAfterWithdrawal { units }) };

    //TODO use helper methods for the following updates?
    // Update the security limit variables
    MAX_LIMIT_CAPACITY.update(
        deps.storage,
        |max_limit_capacity| -> StdResult<_> {
            max_limit_capacity
                .checked_sub(weighted_withdraw_sum)         //TODO use wrapping_sub?
                .map_err(|err| err.into())
        }
    )?;

    USED_LIMIT_CAPACITY.update(
        deps.storage,
        |used_limit_capacity| -> StdResult<_> {
            Ok(
                used_limit_capacity.saturating_sub(weighted_withdraw_sum)
            )
        }
    )?;

    // Build messages to order the transfer of tokens from the vault to the depositor
    let transfer_msgs: Vec<CosmosMsg> = assets.iter()
        .zip(&withdraw_amounts)                                                             // zip: withdraw_amounts.len() == assets.len()
        .filter(|(_, withdraw_amount)| **withdraw_amount != Uint128::zero())     // Do not create transfer messages for zero-valued withdrawals
        .map(|(asset, amount)| {
            Ok(CosmosMsg::Wasm(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: asset.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: sender.clone(),
                        amount: *amount
                    })?,
                    funds: vec![]
                }
            ))
        })
        .collect::<StdResult<Vec<CosmosMsg>>>()?;


    Ok(Response::new()
        .add_messages(transfer_msgs)
        .add_events(burn_response.events)                           // Add burn events //TODO overhaul
        .add_event(
            withdraw_event(
                info.sender.to_string(),
                vault_tokens,
                withdraw_amounts
            )
        )
    )
    
}

pub fn local_swap(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    from_asset: String,
    to_asset: String,
    amount: Uint128,
    min_out: Uint128
) -> Result<Response, ContractError> {

    update_amplification(deps, env.block.time.nanos().into())?;

    let vault_fee: Uint128 = mul_wad_down(            //TODO alternative to not have to use U256 conversion? (or wrapper?)
        U256::from(amount.u128()),
        U256::from(VAULT_FEE.load(deps.storage)?)
    )?.as_uint128();    // Casting safe, as fee < amount, and amount is Uint128

    // Calculate the return value
    let out: Uint128 = calc_local_swap(
        &deps.as_ref(),
        env.clone(),
        &from_asset,
        &to_asset,
        amount - vault_fee
    )?;

    if min_out > out {
        return Err(ContractError::ReturnInsufficient { out, min_out });
    }

    // Update the max limit capacity, as for amplified vaults it is based on the vault asset balances
    let from_weight = WEIGHTS.load(deps.storage, from_asset.as_ref())?;
    let to_weight = WEIGHTS.load(deps.storage, to_asset.as_ref())?;
    let limit_capacity_increase = U256::from(amount).wrapping_mul(U256::from(from_weight));    // wrapping_mul is overflow safe as U256.max >= Uint128.max * u64.max
    let limit_capacity_decrease = U256::from(out).wrapping_mul(U256::from(to_weight));    // wrapping_mul is overflow safe as U256.max >= Uint128.max * u64.max
    MAX_LIMIT_CAPACITY.update(
        deps.storage,
        |max_limit_capacity| -> StdResult<_> {
            if limit_capacity_decrease > limit_capacity_increase {
                max_limit_capacity
                    .checked_sub(
                        limit_capacity_decrease.wrapping_sub(limit_capacity_increase)
                    )
                    .map_err(|err| err.into())
            }
            else {
                max_limit_capacity
                    .checked_add(
                        limit_capacity_increase.wrapping_sub(limit_capacity_decrease)
                    )
                    .map_err(|err| err.into())
            }
        }
    )?;

    // Build message to transfer input assets to the vault
    let transfer_from_asset_msg = CosmosMsg::Wasm(
        cosmwasm_std::WasmMsg::Execute {
            contract_addr: from_asset.clone(),
            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                owner: info.sender.to_string(),
                recipient: env.contract.address.to_string(),
                amount
            })?,
            funds: vec![]
        }
    );

    // Build message to transfer output assets to the swapper
    let transfer_to_asset_msg = CosmosMsg::Wasm(
        cosmwasm_std::WasmMsg::Execute {
            contract_addr: to_asset.clone(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount: out
            })?,
            funds: vec![]
        }
    );

    // Build collect governance fee message
    let collect_governance_fee_message = collect_governance_fee_message(
        &deps.as_ref(),
        from_asset.clone(),
        vault_fee
    )?;

    // Build response
    let mut response = Response::new()
        .add_message(transfer_from_asset_msg)
        .add_message(transfer_to_asset_msg);

    if let Some(msg) = collect_governance_fee_message {
        response = response.add_message(msg);
    }

    Ok(response
        .add_event(
            local_swap_event(
                info.sender.to_string(),
                from_asset,
                to_asset,
                amount,
                out
            )
        )
    )
}


pub fn send_asset(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: String,
    to_vault: Binary,
    to_account: Binary,
    from_asset: String,
    to_asset_index: u8,
    amount: Uint128,
    min_out: U256,
    fallback_account: String,   //TODO EVM mismatch
    calldata: Binary
) -> Result<Response, ContractError> {

    // Only allow connected vaults
    if !is_connected(&deps.as_ref(), &channel_id, to_vault.clone()) {
        return Err(ContractError::VaultNotConnected { channel_id, vault: to_vault })
    }

    update_amplification(deps, env.block.time.nanos().into())?;

    let vault_fee: Uint128 = mul_wad_down(            //TODO alternative to not have to use U256 conversion? (or wrapper?)
        U256::from(amount.u128()),
        U256::from(VAULT_FEE.load(deps.storage)?)
    )?.as_uint128();    // Casting safe, as fee < amount, and amount is Uint128

    // Calculate the group-specific units bought
    let u = calc_send_asset(
        &deps.as_ref(),
        env.clone(),
        &from_asset,
        amount - vault_fee
    )?;

    //TODO create helper function to update the unit tracker?
    UNIT_TRACKER.update(
        deps.storage,
        |unit_tracker| -> StdResult<_> {
            unit_tracker
            .checked_add(u.try_into()?)  // Safely casting into I256 is also very important for 
                                        // 'on_send_asset_success', as it requires this same casting 
                                        // and must never revert.
            .map_err(|err| err.into())
        }
    )?;

    let block_number = env.block.height as u32;
    let send_asset_hash = compute_send_asset_hash(
        to_account.as_slice(),
        u,
        amount - vault_fee,
        &from_asset,
        block_number
    );

    create_asset_escrow(
        deps,
        send_asset_hash.clone(),
        amount - vault_fee,
        &from_asset,
        fallback_account
    )?;

    // Build message to transfer input assets to the vault
    let transfer_from_asset_msg = CosmosMsg::Wasm(
        cosmwasm_std::WasmMsg::Execute {
            contract_addr: from_asset.clone(),
            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                owner: info.sender.to_string(),
                recipient: env.contract.address.to_string(),
                amount
            })?,
            funds: vec![]
        }
    );

    // Build collect governance fee message
    let collect_governance_fee_message = collect_governance_fee_message(
        &deps.as_ref(),
        from_asset.clone(),
        vault_fee
    )?;

    // Build message to 'send' the asset via the IBC interface
    let send_cross_chain_asset_msg = InterfaceExecuteMsg::SendCrossChainAsset {
        channel_id: channel_id.clone(),
        to_vault: to_vault.clone(),
        to_account: to_account.clone(),
        to_asset_index,
        u,
        min_out,
        from_amount: amount,
        from_asset: from_asset.clone(),
        block_number,
        calldata
    };
    let chain_interface = CHAIN_INTERFACE.load(deps.storage)?;
    let send_asset_execute_msg = CosmosMsg::Wasm(
        cosmwasm_std::WasmMsg::Execute {
            contract_addr: chain_interface.ok_or(ContractError::VaultHasNoInterface {})?.to_string(),
            msg: to_binary(&send_cross_chain_asset_msg)?,
            funds: vec![]
        }
    );

    // Build response
    let mut response = Response::new()
        .add_message(transfer_from_asset_msg);

    if let Some(msg) = collect_governance_fee_message {
        response = response.add_message(msg);
    }

    response = response.add_message(send_asset_execute_msg);

    Ok(response
        .add_event(
            send_asset_event(
                channel_id,
                to_vault,
                to_account,
                from_asset,
                to_asset_index,
                amount,
                min_out,
                u,
                vault_fee
            )
        )
    )
}

pub fn receive_asset(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
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
) -> Result<Response, ContractError> {

    // Only allow the 'chain_interface' to invoke this function
    if Some(info.sender) != CHAIN_INTERFACE.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    // Only allow connected vaults
    if !is_connected(&deps.as_ref(), &channel_id, from_vault.clone()) {
        return Err(ContractError::VaultNotConnected { channel_id, vault: from_vault })
    }

    update_amplification(deps, env.block.time.nanos().into())?;

    let assets = ASSETS.load(deps.storage)?;
    let to_asset = assets
        .get(to_asset_index as usize)
        .ok_or(ContractError::AssetNotFound {})?
        .clone();
    let to_weight = WEIGHTS.load(deps.storage, to_asset.as_ref())?;


    let out = calc_receive_asset(&deps.as_ref(), env.clone(), to_asset.as_str(), u)?;

    // Update the max limit capacity and the used limit capacity
    let limit_capacity_delta = U256::from(out).checked_mul(U256::from(to_weight))?;
    MAX_LIMIT_CAPACITY.update(
        deps.storage,
        |max_limit_capacity| -> StdResult<_> {
            max_limit_capacity
                .checked_sub(limit_capacity_delta)
                .map_err(|err| err.into())
        }
    )?;
    update_limit_capacity(deps, env.block.time, limit_capacity_delta)?;
    

    if min_out > out {
        return Err(ContractError::ReturnInsufficient { out, min_out });
    }

    UNIT_TRACKER.update(
        deps.storage,
        |unit_tracker| -> StdResult<_> {
            unit_tracker
                .checked_sub(u.try_into()?)
                .map_err(|err| err.into())
        }
    )?;

    // Build message to transfer output assets to to_account
    let transfer_to_asset_msg = CosmosMsg::Wasm(
        cosmwasm_std::WasmMsg::Execute {
            contract_addr: to_asset.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: to_account.to_string(),
                amount: out
            })?,
            funds: vec![]
        }
    );

    // Build data message
    let calldata_message = calldata_target.map(|target| {
        CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: target.to_string(),
                msg: Binary::from(calldata.unwrap_or(Binary(vec![]))),
                funds: vec![]
            }
        )
    });

    // Build and send response
    let mut response = Response::new();

    if let Some(msg) = calldata_message {
        response = response.add_message(msg);
    }

    Ok(response
        .add_message(transfer_to_asset_msg)
        .add_event(
            receive_asset_event(
                channel_id,
                from_vault,
                to_account,
                to_asset.to_string(),
                u,
                out,
                from_amount,
                from_asset,
                from_block_number_mod
            )
        )
    )
}



pub fn send_liquidity(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: String,
    to_vault: Binary,
    to_account: Binary,
    amount: Uint128,            //TODO EVM mismatch
    min_vault_tokens: U256,
    min_reference_asset: U256,
    fallback_account: String,   //TODO EVM mismatch
    calldata: Binary
) -> Result<Response, ContractError> {

    // Only allow connected vaults
    if !is_connected(&deps.as_ref(), &channel_id, to_vault.clone()) {
        return Err(ContractError::VaultNotConnected { channel_id, vault: to_vault })
    }

    update_amplification(deps, env.block.time.nanos().into())?;

    // Burn the vault tokens of the sender
    execute_burn(deps.branch(), env.clone(), info, amount)?;

    let one_minus_amp = ONE_MINUS_AMP.load(deps.storage)?;

    let (balance_0_ampped, asset_count) = calc_balance_0_ampped(
        deps.as_ref(),
        env.clone(),
        one_minus_amp
    )?;

    // Compute the effective supply. Include 'vault_tokens' to the queried supply as these have already been burnt
    // and also include the escrowed tokens to yield a smaller return
    let effective_supply = U256::from(total_supply(deps.as_ref())?)
        .wrapping_add(amount.into())                                      // 'wrapping_add' is safe as U256.max >> Uint128.max
        .wrapping_add(TOTAL_ESCROWED_LIQUIDITY.load(deps.storage)?.into());     // 'wrapping_add' is safe as U256.max >> Uint128.max

    // Compute 'supply after withdrawal'/'supply before withdrawal' (in WAD terms)
    let vault_tokens_share = div_wad_down(
        effective_supply.checked_add(amount.into())?,
        effective_supply
    )?;

    // Compute the unit value of the provided vaultTokens
    // This step simplifies withdrawing and swapping into a single step
    let units = U256::from(asset_count as u64).checked_mul(
        mul_wad_down(
            balance_0_ampped,
            pow_wad(
                vault_tokens_share.as_i256(),   // If casting overflows to a negative number 'pow_wad' will fail
                one_minus_amp
            )?.as_u256()                        // Casting is safe, as pow_wad result is always positive
                .wrapping_sub(U256::one())      // 'wrapping_sub' is safe as 'pow_wad' result is always >= 1
        )?
    )?;

    //TODO create helper function to update the unit tracker?
    UNIT_TRACKER.update(
        deps.storage,
        |unit_tracker| -> StdResult<_> {
            unit_tracker
            .checked_add(units.try_into()?)  // Safely casting into I256 is also very important for 
                                             // 'on_send_liquidity_success', as it requires this same casting 
                                             // and must never revert.
            .map_err(|err| err.into())
        }
    )?;

    // Compute the hash of the 'send_liquidity' transaction
    let block_number = env.block.height as u32;
    let send_liquidity_hash = compute_send_liquidity_hash(
        to_account.as_slice(),
        units,
        amount,
        block_number
    );

    // Escrow the vault tokens
    create_liquidity_escrow(
        deps,
        send_liquidity_hash.clone(),
        amount,
        fallback_account
    )?;

    // Build message to 'send' the liquidity via the IBC interface
    let send_cross_chain_asset_msg = InterfaceExecuteMsg::SendCrossChainLiquidity {
        channel_id: channel_id.clone(),
        to_vault: to_vault.clone(),
        to_account: to_account.clone(),
        u: units,
        min_vault_tokens,
        min_reference_asset,
        from_amount: amount,
        block_number,
        calldata
    };
    let chain_interface = CHAIN_INTERFACE.load(deps.storage)?;
    let send_liquidity_execute_msg = CosmosMsg::Wasm(
        cosmwasm_std::WasmMsg::Execute {
            contract_addr: chain_interface.as_ref().ok_or(ContractError::VaultHasNoInterface {})?.to_string(),
            msg: to_binary(&send_cross_chain_asset_msg)?,
            funds: vec![]
        }
    );

    //TODO add min_out? (it is present on send_asset)
    Ok(Response::new()
        .add_message(send_liquidity_execute_msg)
        .add_event(
            send_liquidity_event(
                channel_id,
                to_vault,
                to_account,
                amount,
                min_vault_tokens,
                min_reference_asset,
                units
            )
        )
    )
}

pub fn receive_liquidity(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: String,
    from_vault: Binary,
    to_account: String,
    units: U256,
    min_vault_tokens: Uint128,
    min_reference_asset: Uint128,
    from_amount: U256,
    from_block_number_mod: u32,
    calldata_target: Option<Addr>,
    calldata: Option<Binary>
) -> Result<Response, ContractError> {

    // Only allow the 'chain_interface' to invoke this function
    if Some(info.sender) != CHAIN_INTERFACE.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    // Only allow connected vaults
    if !is_connected(&deps.as_ref(), &channel_id, from_vault.clone()) {
        return Err(ContractError::VaultNotConnected { channel_id, vault: from_vault })
    }

    update_amplification(deps, env.block.time.nanos().into())?;

    let one_minus_amp = ONE_MINUS_AMP.load(deps.storage)?;

    let (balance_0_ampped, asset_count) = calc_balance_0_ampped(
        deps.as_ref(),
        env.clone(),
        one_minus_amp
    )?;

    let one_minus_amp_inverse = WADWAD / one_minus_amp;

    let n_weighted_balance_ampped = U256::from(asset_count as u64).checked_mul(
        balance_0_ampped
    )?;

    // Do not include the 'escrowed' vault tokens in the total supply of vault tokens of the vault (return less)
    let total_supply = U256::from(total_supply(deps.as_ref())?);

    let vault_tokens: Uint128 = calc_price_curve_limit_share(
        units,
        total_supply,
        n_weighted_balance_ampped,
        one_minus_amp_inverse
    )?.try_into()?;

    if min_vault_tokens > vault_tokens {
        return Err(ContractError::ReturnInsufficient { out: vault_tokens, min_out: min_vault_tokens });
    }

    if !min_reference_asset.is_zero() {

        let balance_0 = pow_wad(
            balance_0_ampped.as_i256(),     // If casting overflows to a negative number 'pow_wad' will fail
            one_minus_amp_inverse
        )?.as_u256();                       // Casting is safe, as pow_wad result is always positive

        let escrowed_balance = TOTAL_ESCROWED_LIQUIDITY.load(deps.storage)?;

        // Compute the fraction of the 'balance0' that the swapper owns.
        // Include the escrowed vault tokens in the total supply to ensure that even if all the ongoing transactions revert, the specified min_reference_asset is fulfilled.
        // Include the vault tokens as they are going to be minted.
        let balance_0_share: Uint128 = balance_0
            .checked_mul(vault_tokens.into())?
            .div(
                total_supply
                    .wrapping_add(escrowed_balance.into())      // 'wrapping_add' is safe, as U256.max >> Uint128.max
                    .wrapping_add(vault_tokens.into())          // 'wrapping_add' is safe, as U256.max >> Uint128.max
            )
            .div(WAD)
            .try_into()?;

        if min_reference_asset > balance_0_share {
            return Err(ContractError::ReturnInsufficient { out: balance_0_share, min_out: min_reference_asset });
        }

    }

    UNIT_TRACKER.update(
        deps.storage, 
        |unit_tracker| -> StdResult<_> {
            unit_tracker.checked_sub(
                units.try_into()?
            ).map_err(|err| err.into())
        }
    )?;

    // Check and update the security limit
    // If units >= n_weighted_balance_ampped, then they can purchase more than 50% of the vault.
    if n_weighted_balance_ampped <= units {
        return Err(ContractError::SecurityLimitExceeded { amount: units, capacity: n_weighted_balance_ampped }) //TODO review error
    }

    // Otherwise calculate the vault_token_equivalent of the provided units to check if the limit is
    // being honoured.
    let vault_token_equivalent = mul_wad_up(
        pow_wad(
            n_weighted_balance_ampped.as_i256(),    // If casting overflows to a negative number 'pow_wad' will fail
            one_minus_amp_inverse
        )?.as_u256(),                               // Casting is safe, as 'pow_wad' result is always positive
        WAD.wrapping_sub(                           // 'wrapping_sub' is safe as 'pow_wad' result is always <= 1
            pow_wad(
                div_wad_down(
                    n_weighted_balance_ampped.wrapping_sub(units),  // 'wrapping_sub' is safe as 'n_weighted_balance_ampped is always <= units (check is shortly above)
                    n_weighted_balance_ampped
                )?.as_i256(),                       // If casting overflows to a negative number 'pow_wad' will fail
                one_minus_amp_inverse
            )?.as_u256()                            // Casting is safe, as 'pow_wad' result is always positive
        )
    )?;

    update_limit_capacity(
        deps,
        env.block.time,
        mul_wad_down(U256::from(2u64), vault_token_equivalent)?
    )?;

    // Validate the to_account
    deps.api.addr_validate(&to_account)?;   //TODO is this necessary? Isn't the account validated by `execute_mint`?

    // Mint the vault tokens
    let mint_response = execute_mint(
        deps.branch(),
        env.clone(),
        MessageInfo {
            sender: env.contract.address.clone(),   // This contract itself is the one 'sending' the mint operation
            funds: vec![],
        },
        to_account.clone(),
        vault_tokens
    )?;

    // Build data message
    let calldata_message = calldata_target.map(|target| {
        CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: target.to_string(),
                msg: Binary::from(calldata.unwrap_or(Binary(vec![]))),
                funds: vec![]
            }
        )
    });

    // Build and send response
    let mut response = Response::new();

    if let Some(msg) = calldata_message {
        response = response.add_message(msg);
    }

    Ok(response
        .add_event(
            receive_liquidity_event(
                channel_id,
                from_vault,
                to_account,
                units,
                vault_tokens,
                from_amount,
                from_block_number_mod
            )
        )
        .add_events(mint_response.events)       //TODO overhaul
    )
}



pub fn calc_send_asset(
    deps: &Deps,
    env: Env,
    from_asset: &str,
    amount: Uint128
) -> Result<U256, ContractError> {

    let one_minus_amp = ONE_MINUS_AMP.load(deps.storage)?;

    let from_asset_balance: Uint128 = deps.querier.query_wasm_smart::<BalanceResponse>(
        from_asset,
        &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
    )?.balance;

    let from_asset_weight = WEIGHTS.load(deps.storage, from_asset)
        .map_err(|_| ContractError::AssetNotFound {})?;

    let units = calc_price_curve_area(
        amount.u128().into(),
        from_asset_balance.u128().into(),
        U256::from(from_asset_weight),
        one_minus_amp
    )?;

    if from_asset_balance / SMALL_SWAP_RATIO >= amount {
        return Ok(
            units.wrapping_mul(SMALL_SWAP_RETURN) / WAD
        )
    }

    Ok(units)
}

pub fn calc_receive_asset(
    deps: &Deps,
    env: Env,
    to_asset: &str,
    u: U256
) -> Result<Uint128, ContractError> {

    let one_minus_amp = ONE_MINUS_AMP.load(deps.storage)?;

    let to_asset_escrowed_balance: Uint128 = TOTAL_ESCROWED_ASSETS.load(
        deps.storage,
        to_asset
    )?;
    let to_asset_balance: Uint128 = deps.querier.query_wasm_smart::<BalanceResponse>(
        to_asset,
        &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
    )?.balance.checked_sub(to_asset_escrowed_balance)?;      // vault balance minus escrowed balance
    let to_asset_weight = WEIGHTS.load(deps.storage, to_asset)?;
    
    calc_price_curve_limit(
        u,
        to_asset_balance.u128().into(),
        U256::from(to_asset_weight),
        one_minus_amp
    ).and_then(
        |val| TryInto::<Uint128>::try_into(val).map_err(|err| err.into())
    )

}

pub fn calc_local_swap(
    deps: &Deps,
    env: Env,
    from_asset: &str,
    to_asset: &str,
    amount: Uint128
) -> Result<Uint128, ContractError> {

    let one_minus_amp = ONE_MINUS_AMP.load(deps.storage)?;

    let from_asset_weight = WEIGHTS.load(deps.storage, from_asset)
        .map_err(|_| ContractError::AssetNotFound {})?;

    let to_asset_weight = WEIGHTS.load(deps.storage, to_asset)
        .map_err(|_| ContractError::AssetNotFound {})?;

    let from_asset_balance: Uint128 = deps.querier.query_wasm_smart::<BalanceResponse>(
        from_asset,
        &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
    )?.balance;

    let to_asset_escrowed_balance: Uint128 = TOTAL_ESCROWED_ASSETS.load(
        deps.storage,
        to_asset
    )?;
    let to_asset_balance: Uint128 = deps.querier.query_wasm_smart::<BalanceResponse>(
        to_asset,
        &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
    )?.balance.checked_sub(to_asset_escrowed_balance)?;      // vault balance minus escrowed balance

    let output: Uint128 = calc_combined_price_curves(
        amount.u128().into(),
        from_asset_balance.u128().into(),
        to_asset_balance.u128().into(),
        U256::from(from_asset_weight),
        U256::from(to_asset_weight),
        one_minus_amp
    )?.try_into()?;

    if output / SMALL_SWAP_RATIO >= amount {
        return Ok(
            (U256::from(output).wrapping_mul(SMALL_SWAP_RETURN) / WAD).as_uint128()     // Casting is safe, as the result is always <= output, and output is <= Uint128::max
        )
    }

    Ok(output)
}



pub fn calc_balance_0(
    deps: Deps,
    env: Env,
    one_minus_amp: I256
) -> Result<(U256, usize), ContractError> {

    let (balance_0_ampped, asset_count) = calc_balance_0_ampped(
        deps,
        env,
        one_minus_amp
    )?;

    let balance_0 = pow_wad(
        balance_0_ampped.as_i256(),
        WADWAD / one_minus_amp
    )?.as_u256();

    Ok((balance_0, asset_count))
}

pub fn calc_balance_0_ampped(
    deps: Deps,
    env: Env,
    one_minus_amp: I256
) -> Result<(U256, usize), ContractError> {
    
    let assets = ASSETS.load(deps.storage)?;
    let unit_tracker = UNIT_TRACKER.load(deps.storage)?;

    let weights = assets
        .iter()
        .map(|asset| {
            WEIGHTS.load(deps.storage, asset.as_ref())
        })
        .collect::<StdResult<Vec<Uint64>>>()?;

    let asset_balances = assets
        .iter()
        .map(|asset| -> Result<Uint128, ContractError> {
            Ok(
                deps.querier.query_wasm_smart::<BalanceResponse>(
                    asset,
                    &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
                )?.balance
            )
        })
        .collect::<Result<Vec<Uint128>, ContractError>>()?;

    let assets_count = assets.len();

    let weighted_alpha_0_ampped = calc_weighted_alpha_0_ampped(
        weights,
        asset_balances,
        one_minus_amp,
        unit_tracker
    )?;

    let balance_0_ampped = pow_wad(
        weighted_alpha_0_ampped.as_i256(),
        WADWAD / one_minus_amp
    )?.as_u256();

    Ok((balance_0_ampped, assets_count))
}


pub fn on_send_asset_success_amplified(
    deps: &mut DepsMut,
    info: MessageInfo,
    channel_id: String,
    to_account: Binary,
    u: U256,
    amount: Uint128,
    asset: String,
    block_number_mod: u32
) -> Result<Response, ContractError> {  //TODO replace ContractError with NEVER (i.e. it never errors)?

    let response = on_send_asset_success(
        deps,
        info,
        channel_id,
        to_account,
        u,
        amount,
        asset.clone(),
        block_number_mod
    )?;

    let used_capacity = USED_LIMIT_CAPACITY.load(deps.storage)?;

    // Minor optimization: avoid storage write if the used capacity is already at zero
    if used_capacity != U256::zero() {
        USED_LIMIT_CAPACITY.save(deps.storage, &used_capacity.saturating_sub(u))?;
    }

    let weight = WEIGHTS.load(deps.storage, asset.as_ref())?;

    MAX_LIMIT_CAPACITY.update(
        deps.storage,
        |max_limit_capacity| -> StdResult<_> {
            // The max capacity update calculation might overflow, yet it should never make the callback revert.
            // Hence the capacity is set to the maximum allowed value without allowing it to overflow (saturating_mul and saturating_add).
            Ok(
                max_limit_capacity.saturating_add(
                    U256::from(amount).wrapping_mul(U256::from(weight))     // Multiplication is overflow safe, as U256.max >= Uint128.max * u64.max
                )
            )
        }
    )?;

    Ok(response)
}

pub fn on_send_asset_failure_amplified(
    deps: &mut DepsMut,
    info: MessageInfo,
    channel_id: String,
    to_account: Binary,
    u: U256,
    amount: Uint128,
    asset: String,
    block_number_mod: u32
) -> Result<Response, ContractError> {  //TODO replace ContractError with NEVER (i.e. it never errors)?

    let response = on_send_asset_failure(
        deps,
        info,
        channel_id,
        to_account,
        u,
        amount,
        asset,
        block_number_mod
    )?;

    // Remove the timed-out units from the unit tracker.
    UNIT_TRACKER.update(deps.storage, |unit_tracker| -> StdResult<_> {
        Ok(unit_tracker.wrapping_sub(u.as_i256()))      //TODO can wrapping_sub underflow? // 'u' casting to i256 is safe, this has been checked on 'send_asset'
    })?;

    Ok(response)
}

// on_send_liquidity_success is not overwritten since we are unable to increase
// the security limit. This is because it is very expensive to compute the update
// to the security limit. If someone liquidity swapped a significant amount of assets
// it is assumed the vault has low liquidity. In these cases, liquidity swaps shouldn't be used.

pub fn on_send_liquidity_failure_amplified(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: String,
    to_account: Binary,
    u: U256,
    amount: Uint128,
    block_number_mod: u32
) -> Result<Response, ContractError> {  //TODO replace ContractError with NEVER (i.e. it never errors)?

    let response = on_send_liquidity_failure(
        deps,
        env,
        info,
        channel_id,
        to_account,
        u,
        amount,
        block_number_mod
    )?;

    // Remove the timed-out units from the unit tracker.
    UNIT_TRACKER.update(deps.storage, |unit_tracker| -> StdResult<_> {
        Ok(unit_tracker.wrapping_sub(u.as_i256()))      //TODO can wrapping_sub underflow? // 'u' casting to i256 is safe, this has been checked on 'send_liquidity'
    })?;

    Ok(response)
}


pub fn set_amplification(
    deps: &mut DepsMut,
    env: &Env,
    info: MessageInfo,
    target_timestamp: Uint64,
    target_amplification: Uint64
) -> Result<Response, ContractError> {

    // Only allow amplification changes by the factory owner
    if info.sender != factory_owner(&deps.as_ref())? {
        return Err(ContractError::Unauthorized {});
    }
    
    // Check 'target_timestamp' is within the defined acceptable bounds
    let current_time = Uint64::new(env.block.time.nanos());
    if
        target_timestamp < current_time + MIN_ADJUSTMENT_TIME_NANOS ||
        target_timestamp > current_time + MAX_ADJUSTMENT_TIME_NANOS
    {
        return Err(ContractError::InvalidTargetTime {});
    }

    let current_amplification: Uint64 = WAD
        .as_i256()                                          // Casting is safe as 'WAD' < I256.max
        .wrapping_sub(ONE_MINUS_AMP.load(deps.storage)?)    // 'wrapping_sub' is safe as 'ONE_MINUS_AMP' <= 'WAD'
        .as_u64().into();                                   // Casting is safe as 'AMP' <= u64.max

    // Check that the target_amplification is correct (set to < 1)
    if target_amplification >= WAD.as_u64().into() {        // Casting is safe as 'WAD' < u64.max
        return Err(ContractError::InvalidAmplification {})
    }
    
    // Limit the maximum allowed relative amplification change to a factor of 'MAX_AMP_ADJUSTMENT_FACTOR'.
    // Note that this effectively 'locks' the amplification if it gets intialized to 0. Similarly, the 
    // amplification will never be allowed to be set to 0 if it is initialized to any other value 
    // (note how 'target_amplification*MAX_AMP_ADJUSTMENT_FACTOR < current_amplification' is used
    // instead of 'target_amplification < current_amplification/MAX_AMP_ADJUSTMENT_FACTOR').
    if
        target_amplification > current_amplification.checked_mul(MAX_AMP_ADJUSTMENT_FACTOR)? ||
        target_amplification.checked_mul(MAX_AMP_ADJUSTMENT_FACTOR)? < current_amplification
    {
        return Err(ContractError::InvalidAmplification {});
    }

    if CHAIN_INTERFACE.load(deps.storage)?.is_some() {
        return Err(ContractError::Error("Amplification adjustment is disabled for cross-chain vaults.".to_string()));
    }

    // Save the target amplification
    ONE_MINUS_AMP.save(
        deps.storage,
        &WAD
            .as_i256()                                         // Casting is safe, as WAD < I256.max
            .wrapping_sub(I256::from(target_amplification))    // 'wrapping_sub' is safe, as 'target_amplification' is always < WAD (checked shortly above)
    )?;

    // Set the amplification update time parameters
    AMP_UPDATE_FINISH_TIMESTAMP.save(deps.storage, &target_timestamp)?;
    AMP_UPDATE_TIMESTAMP.save(deps.storage, &current_time)?;

    Ok(
        Response::new()
            .add_event(
                set_amplification_event(
                    target_timestamp,
                    target_amplification
                )
            )
    )

}

pub fn update_amplification(
    deps: &mut DepsMut,
    current_timestamp: Uint64
) -> Result<(), ContractError> {
    
    // TODO check instead if the variable *exists* rather than it being set to 0?
    // Only run update logic if 'amp_update_finish_timestamp' is set
    let amp_update_finish_timestamp = AMP_UPDATE_FINISH_TIMESTAMP.load(deps.storage)?;
    if amp_update_finish_timestamp == Uint64::zero() {
        return Ok(());
    }

    // Skip the update if the amplification has already been updated on the same block
    let amp_update_timestamp = AMP_UPDATE_TIMESTAMP.load(deps.storage)?;
    if current_timestamp == amp_update_timestamp {
        return Ok(());
    }

    let target_one_minus_amp = TARGET_ONE_MINUS_AMP.load(deps.storage)?;

    // If the 'amp_update_finish_timestamp' has been reached, finish the amplification update
    if current_timestamp >= amp_update_finish_timestamp {

        ONE_MINUS_AMP.update(
            deps.storage,
            |_| -> StdResult<_> {
                Ok(target_one_minus_amp)
            }
        )?;

        // Clear the 'amp_update_finish_timestamp' to disable the update logic
        AMP_UPDATE_FINISH_TIMESTAMP.save(
            deps.storage,
            &Uint64::zero()
        )?;

    }
    else {

        // Update the amplification value linearly according to the ellapsed time *since the last update*.
        //      new_value = current_value + remaning_value_change * percentage_of_ellapsed_time (where percentage_of_ellapsed_time < 1)

        // The following algorithm uses 'wrapping' functions to save gas. This is safe as:
        //      remaining_value_change = target_one_minus_amp - current_one_minus_amp
        //          => |remaining_value_change| <= WAD = 10**18 < 2**64
        //      time_since_last_update = current_timestamp - amp_update_timestamp
        //          => time_since_last_update <= current_timestamp < 2**64
        //      remaining_update_time = amp_update_finish_timestamp - amp_update_timestamp
        //          => remaining_update_time > time_since_last_update since amp_update_finish_timestamp > current_timestamp
        //
        //      X = remaining_value_change*time_since_last_update
        //          => |X| < 2**128, which means I256.min < X < I256.max
        //      value_change = X / remaining_update_time
        //          => |value_change| < |remaining_value_change| since time_since_last_update < remaining_update_time
        //      new_value = current_value + value_change
        //          => new_value >= 0 and new_value <= WAD, since by definition of the algorithm 'new_value' lies
        //             somewhere between 'current_value' and 'new_value'
    
        ONE_MINUS_AMP.update(
            deps.storage,
            |current_one_minus_amp| -> StdResult<_> {
                let remaining_value_change : I256 = target_one_minus_amp.wrapping_sub(current_one_minus_amp);
                let time_since_last_update : I256 = current_timestamp.wrapping_sub(amp_update_timestamp).into();
                let remaining_update_time  : I256 = amp_update_finish_timestamp.wrapping_sub(amp_update_timestamp).into();

                Ok(
                    current_one_minus_amp.wrapping_add(
                        remaining_value_change
                            .wrapping_mul(time_since_last_update)
                            .div(remaining_update_time)
                    )
                )
            }
        )?;

    }

    // Update the update timestamp
    AMP_UPDATE_TIMESTAMP.save(
        deps.storage,
        &current_timestamp
    )?;

    Ok(())

}


// Query helpers ****************************************************************************************************************

pub fn query_calc_send_asset(
    deps: Deps,
    env: Env,
    from_asset: &str,
    amount: Uint128
) -> StdResult<CalcSendAssetResponse> {

    Ok(
        CalcSendAssetResponse {
            u: calc_send_asset(&deps, env, from_asset, amount)?
        }
    )

}


pub fn query_calc_receive_asset(
    deps: Deps,
    env: Env,
    to_asset: &str,
    u: U256
) -> StdResult<CalcReceiveAssetResponse> {

    Ok(
        CalcReceiveAssetResponse {
            to_amount: calc_receive_asset(&deps, env, to_asset, u)?
        }
    )

}


pub fn query_calc_local_swap(
    deps: Deps,
    env: Env,
    from_asset: &str,
    to_asset: &str,
    amount: Uint128
) -> StdResult<CalcLocalSwapResponse> {

    Ok(
        CalcLocalSwapResponse {
            to_amount: calc_local_swap(&deps, env, from_asset, to_asset, amount)?
        }
    )

}


pub fn query_get_limit_capacity(
    deps: Deps,
    env: Env
) -> StdResult<GetLimitCapacityResponse> {

    Ok(
        GetLimitCapacityResponse {
            capacity: get_limit_capacity(&deps, env)?
        }
    )

}


pub fn query_target_amplification(
    deps: Deps
) -> StdResult<TargetAmplificationResponse> {
    
    Ok(
        TargetAmplificationResponse {
            target_amplification: WAD
                .as_i256()              // Casting is safe as WAD < I256.max
                .wrapping_sub(          // 'wrapping_sub' is safe as WAD >= 'one_minus_amp'
                    TARGET_ONE_MINUS_AMP.load(deps.storage)?
                )
                .as_u64()               // Casting is safe as 'amplification' < u64.max
                .into()
        }
    )

}

pub fn query_amplification_update_finish_timestamp(
    deps: Deps
) -> StdResult<AmplificationUpdateFinishTimestampResponse> {

    Ok(
        AmplificationUpdateFinishTimestampResponse {
            timestamp: AMP_UPDATE_FINISH_TIMESTAMP.load(deps.storage)?
        }
    )

}

pub fn query_balance_0(
    deps: Deps,
    env: Env
) -> StdResult<Balance0Response> {

    Ok(
        Balance0Response {
            balance_0: calc_balance_0(
                deps,
                env,
                ONE_MINUS_AMP.load(deps.storage)?
            )?.0
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