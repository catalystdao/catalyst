use cosmwasm_std::{Addr, Uint128, DepsMut, Env, MessageInfo, Response, StdResult, CosmosMsg, to_binary, Deps, Binary, Uint64, Timestamp};
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg, BalanceResponse};
use cw20_base::contract::{execute_mint, execute_burn};
use cw_storage_plus::{Item, Map};
use catalyst_ibc_interface::msg::ExecuteMsg as InterfaceExecuteMsg;
use catalyst_types::{U256, I256};
use catalyst_vault_common::{
    ContractError,
    event::{local_swap_event, send_asset_event, receive_asset_event, send_liquidity_event, receive_liquidity_event, deposit_event, withdraw_event, cw20_response_to_standard_event}, 
    msg::{CalcSendAssetResponse, CalcReceiveAssetResponse, CalcLocalSwapResponse, GetLimitCapacityResponse},
    state::{ASSETS, FACTORY, MAX_ASSETS, WEIGHTS, INITIAL_MINT_AMOUNT, VAULT_FEE, MAX_LIMIT_CAPACITY, USED_LIMIT_CAPACITY, CHAIN_INTERFACE, TOTAL_ESCROWED_LIQUIDITY, TOTAL_ESCROWED_ASSETS, is_connected, update_limit_capacity, collect_governance_fee_message, compute_send_asset_hash, compute_send_liquidity_hash, create_asset_escrow, create_liquidity_escrow, on_send_asset_success, on_send_liquidity_success, total_supply, get_limit_capacity, factory_owner, initialize_limit_capacity, initialize_escrow_totals}
};
use fixed_point_math::{self, WAD, LN2, mul_wad_down, ln_wad, exp_wad};
use std::ops::Div;

use crate::{
    calculation_helpers::{calc_price_curve_area, calc_price_curve_limit, calc_combined_price_curves, calc_price_curve_limit_share},
    event::set_weights_event,
    msg::{TargetWeightResponse, WeightsUpdateFinishTimestampResponse}
};


// Weight adjustment storage variables and constants
pub const TARGET_WEIGHTS: Map<&str, Uint128> = Map::new("catalyst-vault-volatile-target-weights");
pub const WEIGHT_UPDATE_TIMESTAMP_SECONDS: Item<Uint64> = Item::new("catalyst-vault-volatile-weight-update-timestamp");
pub const WEIGHT_UPDATE_FINISH_TIMESTAMP_SECONDS: Item<Uint64> = Item::new("catalyst-vault-volatile-weight-update-finish-timestamp");

const MIN_ADJUSTMENT_TIME_SECONDS  : Uint64 = Uint64::new(7 * 24 * 60 * 60);     // 7 days
const MAX_ADJUSTMENT_TIME_SECONDS  : Uint64 = Uint64::new(365 * 24 * 60 * 60);   // 1 year
const MAX_WEIGHT_ADJUSTMENT_FACTOR : Uint128 = Uint128::new(10);


/// Initialize the vault swap curves.
/// 
/// The initial asset amounts must be sent to the vault before calling this function.
/// Only the instantiator of the vault may invoke this function (i.e. the `FACTORY`).
/// This should be handled by the Catalyst vault factory.
/// 
/// # Arguments:
/// * `assets` - The list of the assets that are to be supported by the vault.
/// * `weights` - The weights applied to the assets.
/// * `amp` - The amplification value applied to the vault (should be WAD).
/// * `depositor` - The account that will receive the initial vault tokens.
/// 
pub fn initialize_swap_curves(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    assets: Vec<String>,
    weights: Vec<Uint128>,
    amp: Uint64,
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

    // Check that the amplification is correct (set to 1)
    if amp != Uint64::new(WAD.as_u64()) {
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
        .zip(&assets)   // zip: assets.len() == weights.len() (checked above)
        .try_for_each(|(weight, asset)| -> Result<(), ContractError> {

            if weight.is_zero() {
                return Err(ContractError::InvalidWeight {});
            }

            WEIGHTS.save(deps.storage, asset, weight)?;
            TARGET_WEIGHTS.save(deps.storage, asset, weight)?;     // Initialize the target_weights storage (values do not matter)
            
            Ok(())
        })?;
    
    WEIGHT_UPDATE_TIMESTAMP_SECONDS.save(deps.storage, &Uint64::zero())?;
    WEIGHT_UPDATE_FINISH_TIMESTAMP_SECONDS.save(deps.storage, &Uint64::zero())?;

    // Initialize the escrows
    initialize_escrow_totals(deps, assets)?;

    // Initialize the security limit
    // The maximum unit flow is \sum{weights}·ln(2)
    let weights_sum = weights.iter().map(|weight| U256::from(*weight)).sum();
    let max_limit_capacity = LN2.checked_mul(weights_sum)?;
    initialize_limit_capacity(deps, max_limit_capacity)?;


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
        depositor.clone(),  // NOTE: the address is validated by the 'execute_mint' call
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




/// Deposit a user-configurable balance of assets on the vault.
/// 
/// **NOTE**: The vault's access to the deposited assets must be approved by the user. 
/// 
/// **NOTE**: The vault fee is imposed on deposits.
/// 
/// # Arguments:
/// * `deposit_amounts` - The asset amounts to be deposited.
/// * `min_out` - The minimum output of vault tokens to get in return.
/// 
pub fn deposit_mixed(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    deposit_amounts: Vec<Uint128>,
    min_out: Uint128
) -> Result<Response, ContractError> {

    // This deposit function works by calculating how many units the deposited assets
    // are worth, and translating those into vault tokens.

    update_weights(deps, env.block.time)?;

    let assets = ASSETS.load(deps.storage)?;

    if deposit_amounts.len() != assets.len() {
        return Err(
            ContractError::InvalidParameters{
                reason: "Invalid deposit_amounts count.".to_string()
            }
        );
    }

    // Compute how many 'units' the assets are worth.
    let u = assets.iter()
        .zip(&deposit_amounts)      // zip: deposit_amounts.len() == assets.len() (checked above)
        .try_fold(U256::zero(), |acc, (asset, deposit_amount)| {

            // Save gas if the user provides no tokens for the specific asset
            if deposit_amount.is_zero() {
                return Ok(acc);
            }

            let vault_asset_balance = deps.querier.query_wasm_smart::<BalanceResponse>(
                asset,
                &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
            )?.balance;

            let weight = WEIGHTS.load(deps.storage, asset.as_ref())?;

            acc.checked_add(
                calc_price_curve_area(
                    U256::from(*deposit_amount),
                    U256::from(vault_asset_balance),
                    U256::from(weight)
                )?
            ).map_err(|_| ContractError::ArithmeticError {})
        })?;

    // Subtract the vault fee from U to prevent deposit and withdrawals being employed as a method of swapping.
    // To reduce gas costs, the governance fee is not taken. This is not an issue as swapping via this method is 
    // disincentivized by its higher gas costs.
    let vault_fee = VAULT_FEE.load(deps.storage)?;
    let u = fixed_point_math::mul_wad_down(
        u,
        fixed_point_math::WAD.wrapping_sub(U256::from(vault_fee))   // 'wrapping_sub' is safe, as 'vault_fee' <= 'WAD'
    )?;

    // Do not include the 'escrowed' vault tokens in the total supply of vault tokens (return less)
    let effective_supply = U256::from(total_supply(deps.as_ref())?);

    // Derive the weight sum from the security limit capacity
    let weights_sum = MAX_LIMIT_CAPACITY.load(deps.storage)? / fixed_point_math::LN2;

    // Compute the vault tokens to be minted.
    let out = fixed_point_math::mul_wad_down(
        effective_supply,                               // NOTE: 'effective_supply' is not in WAD terms,
                                                        // hence the result of 'mul_wad_down' will not be either
        calc_price_curve_limit_share(u, weights_sum)?
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

    // Build the messages to order the transfer of tokens from the depositor to the vault
    // ! IMPORTANT: Some cw20 contracts disallow zero-valued token transfers. Do not generate
    // ! transfer messages for zero-valued balance transfers to prevent these cases from 
    // ! resulting in failed transactions.
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
        .add_event(
            deposit_event(
                info.sender.to_string(),
                out,
                deposit_amounts
            )
        )
        .add_event(
            cw20_response_to_standard_event(
                mint_response
            )
        )
    )
}


/// Withdraw an even amount of assets from the vault (i.e. according to the current balance ratios).
/// 
/// **NOTE**: This is the only way to withdraw 100% of the vault liquidity.
/// 
/// # Arguments:
/// * `vault_tokens` - The amount of vault tokens to burn.
/// * `min_out` - The minimum output of assets to get in return.
/// 
pub fn withdraw_all(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    vault_tokens: Uint128,
    min_out: Vec<Uint128>,
) -> Result<Response, ContractError> {

    // This withdraw function works by computing the share of the total vault tokens that 
    // the provided ones account for. That share of the vault's assets balances is returned
    // to the user.
    
    update_weights(deps, env.block.time)?;

    // Include the 'escrowed' vault tokens in the total supply of vault tokens of the vault
    let escrowed_vault_tokens = TOTAL_ESCROWED_LIQUIDITY.load(deps.storage)?;
    let effective_supply = total_supply(deps.as_ref())?.checked_add(escrowed_vault_tokens)?;

    // Burn the vault tokens of the withdrawer
    let sender = info.sender.to_string();
    let burn_response = execute_burn(deps.branch(), env.clone(), info.clone(), vault_tokens)?;

    // Compute the withdraw amounts
    let assets = ASSETS.load(deps.storage)?;

    if min_out.len() != assets.len() {
        return Err(
            ContractError::InvalidParameters {
                reason: "Invalid min_out count.".to_string()
            }
        );
    }

    let withdraw_amounts: Vec<Uint128> = assets
        .iter()
        .zip(&min_out)          // zip: assets.len() == min_out.len()
        .map(|(asset, asset_min_out)| {

            let escrowed_balance = TOTAL_ESCROWED_ASSETS.load(deps.storage, asset.as_str())?;
            
            let vault_asset_balance = deps.querier.query_wasm_smart::<BalanceResponse>(
                asset,
                &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
            )?.balance;

            let effective_vault_asset_balance = vault_asset_balance
                .checked_sub(escrowed_balance)?;    // Theoretically 'escrowed_balance' is always included 
                                                    // in 'vault_asset_balance'. However, it would be terrible
                                                    // for this calculation to underflow, hence use 'checked'.

            let withdraw_amount = U256::from(effective_vault_asset_balance)
                .wrapping_mul(U256::from(vault_tokens))     // 'wrapping_mul' is safe, as U256::MAX > Uint128::MAX*Uint128::MAX
                .div(U256::from(effective_supply))
                .as_uint128();                              // Casting is safe, as 'effective_supply' is always >= 'vault_tokens'

            // Check that the minimum output is honoured.
            if *asset_min_out > withdraw_amount {
                return Err(ContractError::ReturnInsufficient { out: withdraw_amount.clone(), min_out: *asset_min_out });
            };

            Ok(withdraw_amount)
        }).collect::<Result<Vec<Uint128>, ContractError>>()?;

    // Build the messages to order the transfer of tokens from the vault to the depositor
    let transfer_msgs: Vec<CosmosMsg> = assets
        .iter()    // zip: withdraw_amounts.len() == assets.len()
        .zip(&withdraw_amounts)
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
        }).collect::<StdResult<Vec<CosmosMsg>>>()?;


    Ok(Response::new()
        .add_messages(transfer_msgs)
        .add_event(
            withdraw_event(
                info.sender.to_string(),
                vault_tokens,
                withdraw_amounts
            )
        )
        .add_event(
            cw20_response_to_standard_event(
                burn_response
            )
        )
    )
    
}


/// Withdraw an uneven amount of assets from the vault (i.e. according to a user defined ratio).
/// 
/// **NOTE**: This function cannot be used to withdraw all the vault liquidity (for that use `withdraw_all`).
/// 
/// # Arguments:
/// * `vault_tokens` - The amount of vault tokens to burn.
/// * `withdraw_ratio` - The ratio at which to withdraw the assets. Each value is the percentage 
/// of the remaining units to be used for the corresponding asset (i.e. given \[r0, r1, r2\],
/// U0 = U · r0, U1 = (U - U0) · r1, U2 = (U - U0 - U1) · r2). In WAD terms.
/// * `min_out` - The minimum output of assets to get in return.
/// 
pub fn withdraw_mixed(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    vault_tokens: Uint128,
    withdraw_ratio: Vec<Uint64>,
    min_out: Vec<Uint128>,
) -> Result<Response, ContractError> {

    // This withdraw function works by computing the 'units' value of the provided vault tokens,
    // and then translating those into assets balances according to the provided 'withdraw_ratio'.
    
    update_weights(deps, env.block.time)?;

    // Include the 'escrowed' vault tokens in the total supply of vault tokens of the vault
    let escrowed_vault_tokens = TOTAL_ESCROWED_LIQUIDITY.load(deps.storage)?;
    let effective_supply = U256::from(
        total_supply(deps.as_ref())?.checked_add(escrowed_vault_tokens)?
    );

    // Burn the vault tokens of the withdrawer
    let sender = info.sender.to_string();
    let burn_response = execute_burn(deps.branch(), env.clone(), info.clone(), vault_tokens)?;

    // Derive the weights sum from the security limit capacity
    let weights_sum = MAX_LIMIT_CAPACITY.load(deps.storage)? / fixed_point_math::LN2;

    // Compute the unit worth of the vault tokens.
    // NOTE: The following logic makes it impossible to withdraw all of the vault's liquidity.
    let mut u: U256 = fixed_point_math::ln_wad(
        fixed_point_math::div_wad_down(
            effective_supply,
            effective_supply.wrapping_sub(U256::from(vault_tokens))  // 'wrapping_sub' is underflow safe, as the above 'execute_burn' guarantees that 'vault_tokens' is contained in 'effective_supply'
        )?.as_i256()                                           // Casting may overflow to a negative value. In that case, 'ln_wad' will fail.
    )?.as_u256()                                               // Casting is safe, as ln is computed of values >= 1, hence output is always positive
        .checked_mul(weights_sum)?;

    // Compute the withdraw amounts
    let assets = ASSETS.load(deps.storage)?;

    if withdraw_ratio.len() != assets.len() || min_out.len() != assets.len() {
        return Err(
            ContractError::InvalidParameters {
                reason: "Invalid withdraw_ratio/min_out count.".to_string()
            }
        );
    }

    let withdraw_amounts: Vec<Uint128> = assets
        .iter()
        .zip(&withdraw_ratio)               // zip: withdraw_ratio.len() == assets.len()
        .zip(&min_out)                      // zip: min_out.len() == assets.len()
        .map(|((asset, asset_withdraw_ratio), asset_min_out)| {

            // Calculate the units allocated for the specific asset
            let units_for_asset = fixed_point_math::mul_wad_down(u, U256::from(*asset_withdraw_ratio))?;
            if units_for_asset.is_zero() {

                // There should not be a non-zero withdraw ratio after a withdraw ratio of 1 (protect against user error)
                if !asset_withdraw_ratio.is_zero() {
                    return Err(ContractError::WithdrawRatioNotZero {}) 
                };

                // Check that the minimum output is honoured.
                if !asset_min_out.is_zero() {
                    return Err(ContractError::ReturnInsufficient { out: Uint128::zero(), min_out: *asset_min_out })
                };

                return Ok(Uint128::zero());
            }

            // Subtract the units used from the total units amount.
            u = u.checked_sub(units_for_asset)?; // ! 'checked_sub' important: This will underflow for 
                                                 // ! malicious withdraw ratios (i.e. ratios > 1).
        
            // Get the vault asset balance (subtract the escrowed assets to return less)
            let vault_asset_balance = deps.querier.query_wasm_smart::<BalanceResponse>(
                asset,
                &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
            )?.balance;

            let escrowed_balance = TOTAL_ESCROWED_ASSETS.load(deps.storage, asset.as_ref())?;

            let effective_vault_asset_balance = vault_asset_balance
                .checked_sub(escrowed_balance)?;

            // Calculate the asset amount corresponding to the asset units
            let weight = WEIGHTS.load(deps.storage, asset.as_ref())?;
            let withdraw_amount = calc_price_curve_limit(
                units_for_asset,
                U256::from(effective_vault_asset_balance),
                U256::from(weight)
            )?.try_into()?;

            // Check that the minimum output is honoured.
            if *asset_min_out > withdraw_amount {
                return Err(ContractError::ReturnInsufficient { out: withdraw_amount.clone(), min_out: *asset_min_out });
            };

            Ok(withdraw_amount)
        }).collect::<Result<Vec<Uint128>, ContractError>>()?;

    // Make sure all units have been consumed
    if !u.is_zero() { return Err(ContractError::UnusedUnitsAfterWithdrawal { units: u }) };

    // Build the messages to order the transfer of tokens from the vault to the depositor.
    // ! IMPORTANT: Some cw20 contracts disallow zero-valued token transfers. Do not generate
    // ! transfer messages for zero-valued balance transfers to prevent these cases from 
    // ! resulting in failed transactions.
    let transfer_msgs: Vec<CosmosMsg> = assets.iter()
        .zip(&withdraw_amounts)         // zip: withdraw_amounts.len() == assets.len()
        .filter(|(_, withdraw_amount)| !withdraw_amount.is_zero())     // Do not create transfer messages for zero-valued withdrawals
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
        .add_event(
            withdraw_event(
                info.sender.to_string(),
                vault_tokens,
                withdraw_amounts
            )
        )
        .add_event(
            cw20_response_to_standard_event(
                burn_response
            )
        )
    )
    
}


/// Perform a local asset swap.
/// 
/// **NOTE**: The vault's access to the source asset must be approved by the user. 
/// 
/// # Arguments:
/// * `from_asset` - The source asset.
/// * `to_asset` - The destination asset.
/// * `amount` - The `from_asset` amount sold to the vault.
/// * `min_out` - The mininmum return to get of `to_asset`.
/// 
pub fn local_swap(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    from_asset: String,
    to_asset: String,
    amount: Uint128,
    min_out: Uint128
) -> Result<Response, ContractError> {

    update_weights(deps, env.block.time)?;

    let vault_fee: Uint128 = mul_wad_down(
        U256::from(amount),
        U256::from(VAULT_FEE.load(deps.storage)?)
    )?.as_uint128();    // Casting safe, as fee < amount, and amount is Uint128

    // Calculate the return value
    let out: Uint128 = calc_local_swap(
        &deps.as_ref(),
        env.clone(),
        &from_asset,
        &to_asset,
        amount.checked_sub(vault_fee)?      // Using 'checked_sub' for extra precaution ('wrapping_sub' should suffice)
    )?;

    if min_out > out {
        return Err(ContractError::ReturnInsufficient { out, min_out });
    }

    // Build the message to transfer the input assets to the vault.
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

    // Build the message to transfer the output assets to the swapper
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

    // Build the message to collect the governance fee.
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


/// Initiate a cross-chain asset swap.
/// 
/// **NOTE**: The vault's access to the source asset must be approved by the user. 
/// 
/// # Arguments:
/// * `channel_id` - The target chain identifier.
/// * `to_vault` - The target vault on the target chain (Catalyst encoded).
/// * `to_account` - The recipient of the swap on the target chain (Catalyst encoded).
/// * `from_asset` - The source asset.
/// * `to_asset_index` - The destination asset index.
/// * `amount` - The `from_asset` amount sold to the vault.
/// * `min_out` - The mininum `to_asset` output amount to get on the target vault.
/// * `fallback_account` - The recipient of the swapped amount should the swap fail.
/// * `calldata` - Arbitrary data to be executed on the target chain upon successful execution of the swap.
/// 
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
    fallback_account: String,
    calldata: Binary
) -> Result<Response, ContractError> {

    // Only allow connected vaults.
    if !is_connected(&deps.as_ref(), &channel_id, to_vault.clone()) {
        return Err(ContractError::VaultNotConnected { channel_id, vault: to_vault })
    }

    update_weights(deps, env.block.time)?;

    let vault_fee: Uint128 = mul_wad_down(
        U256::from(amount),
        U256::from(VAULT_FEE.load(deps.storage)?)
    )?.as_uint128();    // Casting safe, as fee < amount, and amount is Uint128

    let effective_swap_amount = amount.checked_sub(vault_fee)?;     // Using 'checked_sub' for extra precaution ('wrapping_sub' should suffice)

    // Calculate the units bought.
    let u = calc_send_asset(
        &deps.as_ref(),
        env.clone(),
        &from_asset,
        effective_swap_amount
    )?;

    // Create a 'send asset' escrow
    let block_number = env.block.height as u32;
    let send_asset_hash = compute_send_asset_hash(
        to_account.as_slice(),
        u,
        effective_swap_amount,
        &from_asset,
        block_number
    );

    create_asset_escrow(
        deps,
        send_asset_hash.clone(),
        effective_swap_amount,  // NOTE: The fee is also deducted from the escrow  
                                // amount to prevent denial of service attacks.
        &from_asset,
        fallback_account
    )?;

    // NOTE: The security limit adjustment is delayed until the swap confirmation is received to
    // prevent a router from abusing swap 'timeouts' to circumvent the security limit.

    // Build the message to transfer the input assets to the vault.
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

    // Build the message to collect the governance fee.
    let collect_governance_fee_message = collect_governance_fee_message(
        &deps.as_ref(),
        from_asset.clone(),
        vault_fee
    )?;

    // Build the message to send the purchased units via the IBC interface.
    let send_cross_chain_asset_msg = InterfaceExecuteMsg::SendCrossChainAsset {
        channel_id: channel_id.clone(),
        to_vault: to_vault.clone(),
        to_account: to_account.clone(),
        to_asset_index,
        u,
        min_out,
        from_amount: effective_swap_amount,
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


/// Receive a cross-chain asset swap.
/// 
/// **NOTE**: Only the chain interface may invoke this function.
/// 
/// # Arguments:
/// * `channel_id` - The source chain identifier.
/// * `from_vault` - The source vault on the source chain.
/// * `to_asset_index` - The index of the purchased asset.
/// * `to_account` - The recipient of the swap.
/// * `u` - The incoming units.
/// * `min_out` - The mininum output amount.
/// * `from_amount` - The `from_asset` amount sold to the source vault.
/// * `from_asset` - The source asset.
/// * `from_block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32).
/// * `calldata_target` - The contract address to invoke upon successful execution of the swap.
/// * `calldata` - The data to pass to `calldata_target` upon successful execution of the swap.
/// 
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

    // Only allow the 'chain_interface' to invoke this function.
    if Some(info.sender) != CHAIN_INTERFACE.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    // Only allow connected vaults.
    if !is_connected(&deps.as_ref(), &channel_id, from_vault.clone()) {
        return Err(ContractError::VaultNotConnected { channel_id, vault: from_vault })
    }

    update_weights(deps, env.block.time)?;

    // Check and update the security limit.
    update_limit_capacity(deps, env.block.time, u)?;

    // Calculate the swap return.
    // NOTE: no fee is taken here, the fee is always taken on the sending side.
    let assets = ASSETS.load(deps.storage)?;
    let to_asset = assets
        .get(to_asset_index as usize)
        .ok_or(ContractError::AssetNotFound {})?
        .clone();
    let out = calc_receive_asset(&deps.as_ref(), env.clone(), to_asset.as_str(), u)?;
    
    if min_out > out {
        return Err(ContractError::ReturnInsufficient { out, min_out });
    }


    // Build the message to transfer the output assets to the swapper.
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

    // Build the calldata message.
    let calldata_message = calldata_target.map(|target| {
        CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: target.to_string(),
                msg: Binary::from(calldata.unwrap_or(Binary(vec![]))),
                funds: vec![]
            }
        )
    });

    // Build and send the response.
    let mut response = Response::new()
        .add_message(transfer_to_asset_msg);

    if let Some(msg) = calldata_message {
        response = response.add_message(msg);
    }

    Ok(response
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


/// Initiate a cross-chain liquidity swap.
/// 
/// This is a macro *equivalent* to performing a withdrawal from the vault and 
/// sending the withdrawed assets to another vault. This is implemeted in a single
/// step (i.e. without performing the individual operations).
/// 
/// # Arguments:
/// * `channel_id` - The target chain identifier.
/// * `to_vault` - The target vault on the target chain (Catalyst encoded).
/// * `to_account` - The recipient of the swap on the target chain (Catalyst encoded).
/// * `amount` - The vault tokens amount sold to the vault.
/// * `min_vault_tokens` - The mininum vault tokens output amount to get on the target vault.
/// * `min_reference_asset` - The mininum reference asset value on the target vault.
/// * `fallback_account` - The recipient of the swapped amount should the swap fail.
/// * `calldata` - Arbitrary data to be executed on the target chain upon successful execution of the swap.
/// 
pub fn send_liquidity(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: String,
    to_vault: Binary,
    to_account: Binary,
    amount: Uint128,
    min_vault_tokens: U256,
    min_reference_asset: U256,
    fallback_account: String,
    calldata: Binary
) -> Result<Response, ContractError> {

    // Only allow connected vaults
    if !is_connected(&deps.as_ref(), &channel_id, to_vault.clone()) {
        return Err(ContractError::VaultNotConnected { channel_id, vault: to_vault })
    }

    update_weights(deps, env.block.time)?;

    // Include the 'escrowed' vault tokens in the total supply of vault tokens of the vault
    let escrowed_vault_tokens = TOTAL_ESCROWED_LIQUIDITY.load(deps.storage)?;
    let effective_supply = U256::from(total_supply(deps.as_ref())?)
        .wrapping_add(U256::from(escrowed_vault_tokens));        // 'wrapping_add' is overflow safe because of casting into U256

    // Burn the vault tokens of the sender
    let burn_response = execute_burn(deps.branch(), env.clone(), info, amount)?;

    // Derive the weights sum from the security limit capacity
    let weights_sum = MAX_LIMIT_CAPACITY.load(deps.storage)? / fixed_point_math::LN2;

    // Compute the unit value of the provided vault tokens
    // This step simplifies withdrawing and swapping into a single step
    let u = fixed_point_math::ln_wad(
        fixed_point_math::div_wad_down(
            effective_supply,
            effective_supply.wrapping_sub(U256::from(amount))   // 'wrapping_sub' is safe, as 'amount' is always contained in 'effective_supply'
        )?.as_i256()                    // if casting overflows into a negative value, the 'ln' calculation will fail
    )?.as_u256()                        // casting is safe as 'ln' is computed of a value >= 1 (hence result is always positive)
        .checked_mul(weights_sum)?;

    // Create a 'send liquidity' escrow
    let block_number = env.block.height as u32;
    let send_liquidity_hash = compute_send_liquidity_hash(
        to_account.as_slice(),
        u,
        amount,
        block_number
    );

    create_liquidity_escrow(
        deps,
        send_liquidity_hash.clone(),
        amount,
        fallback_account
    )?;

    // NOTE: The security limit adjustment is delayed until the swap confirmation is received to
    // prevent a router from abusing swap 'timeouts' to circumvent the security limit.

    // Build the message to 'send' the liquidity via the IBC interface.
    let send_cross_chain_asset_msg = InterfaceExecuteMsg::SendCrossChainLiquidity {
        channel_id: channel_id.clone(),
        to_vault: to_vault.clone(),
        to_account: to_account.clone(),
        u,
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
                u
            )
        )
        .add_event(
            cw20_response_to_standard_event(
                burn_response
            )
        )
    )
}


/// Receive a cross-chain liquidity swap.
/// 
/// This is a macro *equivalent* to receiveing assets from another vault and performing
/// a deposit of those assets into the vault. This is implemeted in a single step
/// (i.e. without performing the individual operations).
/// 
/// **NOTE**: Only the chain interface may invoke this function.
/// 
/// # Arguments:
/// * `channel_id` - The source chain identifier.
/// * `from_vault` - The source vault on the source chain.
/// * `to_account` - The recipient of the swap.
/// * `u` - The incoming units.
/// * `min_vault_tokens` - The mininum vault tokens output amount.
/// * `min_reference_asset` - The mininum reference asset value.
/// * `from_amount` - The `from_asset` amount sold to the source vault.
/// * `from_block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32).
/// * `calldata_target` - The contract address to invoke upon successful execution of the swap.
/// * `calldata` - The data to pass to `calldata_target` upon successful execution of the swap.
/// 
pub fn receive_liquidity(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: String,
    from_vault: Binary,
    to_account: String,
    u: U256,
    min_vault_tokens: Uint128,
    min_reference_asset: Uint128,
    from_amount: U256,
    from_block_number_mod: u32,
    calldata_target: Option<Addr>,
    calldata: Option<Binary>
) -> Result<Response, ContractError> {

    // Only allow the 'chain_interface' to invoke this function.
    if Some(info.sender) != CHAIN_INTERFACE.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    // Only allow connected vaults.
    if !is_connected(&deps.as_ref(), &channel_id, from_vault.clone()) {
        return Err(ContractError::VaultNotConnected { channel_id, vault: from_vault })
    }

    update_weights(deps, env.block.time)?;

    // Check and update the security limit
    update_limit_capacity(deps, env.block.time, u)?;

    // Derive the weights sum from the security limit capacity
    let weights_sum = MAX_LIMIT_CAPACITY.load(deps.storage)? / fixed_point_math::LN2;

    // Do not include the 'escrowed' vault tokens in the total supply of vault tokens of the vault (return less)
    let effective_supply = U256::from(total_supply(deps.as_ref())?);

    // Use 'calc_price_curve_limit_share' to get the % of vault tokens that should be minted (in WAD terms)
    // Multiply by 'effective_supply' to get the absolute amount (not in WAD terms) using 'mul_wad_down' so
    // that the result is also NOT in WAD terms.
    let out: Uint128 = fixed_point_math::mul_wad_down(
        calc_price_curve_limit_share(u, weights_sum)?,
        effective_supply
    )?.try_into()?;

    if min_vault_tokens > out {
        return Err(ContractError::ReturnInsufficient { out, min_out: min_vault_tokens });
    }

    if !min_reference_asset.is_zero() {

        let assets = ASSETS.load(deps.storage)?;

        // Compute the vault reference amount: [product(balance(i)**weight(i))]**(1/weights_sum)
        // The direct calculation of this value would overflow, hence it is calculated as:
        //      exp( sum( ln(balance(i)) * weight(i) ) / weights_sum )

        // Compute first: sum( ln(balance(i)) * weight(i) )
        let weighted_balance_sum = assets.iter()
            .try_fold(U256::zero(), |acc, asset| {

                let weight = WEIGHTS.load(deps.storage, asset.as_ref())?;

                let vault_asset_balance = deps.querier.query_wasm_smart::<BalanceResponse>(
                    asset,
                    &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
                )?.balance;

                acc.checked_add(
                    ln_wad(
                        I256::from(vault_asset_balance)     // i256 casting: safe as Uint128::MAX < I256::MAX
                            .wrapping_mul(WAD.as_i256())    //'wrapping_mul' safe as Uint128::MAX * WAD < I256::MAX (i.e. ~2^128 * ~2^64 < ~2^255)
                    )?.as_u256()                            // u256 casting: 'vault_asset_balance * WAD' >= WAD (for balance != 0), hence 'ln_wad' return is always positive (otherwise 'ln_wad' will fail)
                    .checked_mul(U256::from(weight))?
                ).map_err(|_| ContractError::ArithmeticError {})
            })?;

        // Finish the calculation: exp( 'weighted_balance_sum' / weights_sum )
        let vault_reference_amount = exp_wad(
            (weighted_balance_sum / weights_sum)    // Division is safe, as w_sum is never 0
                .as_i256()                          // If casting overflows to a negative number, the result of the exponent will be 0
                                                    // (after dividing by WAD), which will cause the 'min_reference_asset' check to fail.
        )?.as_u256() / WAD;                         // Division is safe, as WAD != 0

        // Compute the fraction of the 'vault_reference_amount' that the swapper owns.
        // Include the escrowed vault tokens in the total supply to ensure that even if all the ongoing transactions revert, the specified min_reference_asset is fulfilled.
        // Include the vault tokens as they are going to be minted.
        let escrowed_vault_tokens = TOTAL_ESCROWED_LIQUIDITY.load(deps.storage)?;
        let user_reference_amount: Uint128 = (
            vault_reference_amount
                .checked_mul(U256::from(out))?
                .div(
                    effective_supply
                        .wrapping_add(U256::from(escrowed_vault_tokens))    // 'wrapping_add' is safe because of casting to U256 (and 'effective_supply' <= Uint128::MAX)
                        .wrapping_add(U256::from(out))                      // 'wrapping_add' is safe because of casting to U256 (and 'effective_supply' <= Uint128::MAX)
                )
        ).try_into()?;

        if min_reference_asset > user_reference_amount {
            return Err(ContractError::ReturnInsufficient { out: user_reference_amount, min_out: min_reference_asset });
        }

    }

    // Mint the vault tokens
    let mint_response = execute_mint(
        deps.branch(),
        env.clone(),
        MessageInfo {
            sender: env.contract.address.clone(),   // This contract itself is the one 'sending' the mint operation
            funds: vec![],
        },
        to_account.clone(),  // NOTE: the address is validated by the 'execute_mint' call
        out
    )?;

    // Build the calldata message.
    let calldata_message = calldata_target.map(|target| {
        CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: target.to_string(),
                msg: Binary::from(calldata.unwrap_or(Binary(vec![]))),
                funds: vec![]
            }
        )
    });

    // Build and send the response.
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
                u,
                out,
                from_amount,
                from_block_number_mod
            )
        )
        .add_event(
            cw20_response_to_standard_event(
                mint_response
            )
        )
    )
}



/// Compute the return of 'send_asset' (not including fees).
/// 
/// **NOTE**: This function reverts if 'from_asset' does not form part of the vault.
/// 
/// # Arguments:
/// * `from_asset` - The source asset.
/// * `amount` - The `from_asset` amount sold to the vault (excluding the vault fee).
/// 
pub fn calc_send_asset(
    deps: &Deps,
    env: Env,
    from_asset: &str,
    amount: Uint128
) -> Result<U256, ContractError> {

    let from_asset_weight = WEIGHTS.load(deps.storage, from_asset)
        .map_err(|_| ContractError::AssetNotFound {})?;

    let from_asset_balance: Uint128 = deps.querier.query_wasm_smart::<BalanceResponse>(
        from_asset,
        &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
    )?.balance;

    calc_price_curve_area(
        amount.into(),
        from_asset_balance.into(),
        from_asset_weight.into(),
    )
}

/// Compute the return of 'receive_asset'.
/// 
/// **NOTE**: This function reverts if 'to_asset' does not form part of the vault.
/// 
/// # Arguments:
/// * `to_asset` - The target asset.
/// * `u` - The incoming units.
/// 
pub fn calc_receive_asset(
    deps: &Deps,
    env: Env,
    to_asset: &str,
    u: U256
) -> Result<Uint128, ContractError> {

    let to_asset_weight = WEIGHTS.load(deps.storage, to_asset)
        .map_err(|_| ContractError::AssetNotFound {})?;

    // Subtract the escrowed balance from the vault's total balance to return a smaller output.
    let to_asset_escrowed_balance: Uint128 = TOTAL_ESCROWED_ASSETS.load(
        deps.storage,
        to_asset
    )?;
    let to_asset_balance: Uint128 = deps.querier.query_wasm_smart::<BalanceResponse>(
        to_asset,
        &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
    )?.balance.checked_sub(to_asset_escrowed_balance)?;
    
    calc_price_curve_limit(
        u,
        to_asset_balance.into(),
        to_asset_weight.into(),
    ).and_then(
        |val| TryInto::<Uint128>::try_into(val).map_err(|err| err.into())
    )

}

/// Compute the return of 'local_swap' (not including fees).
/// 
/// If 'from_asset' and 'to_asset' have equal weights, a simplified formula is used.
/// 
/// **NOTE**: This function reverts if 'from_asset' or 'to_asset' do not form part 
/// of the vault.
/// 
/// # Arguments:
/// * `from_asset` - The source asset.
/// * `to_asset` - The destination asset.
/// * `amount` - The `from_asset` amount sold to the vault (excluding fees).
/// 
pub fn calc_local_swap(
    deps: &Deps,
    env: Env,
    from_asset: &str,
    to_asset: &str,
    amount: Uint128
) -> Result<Uint128, ContractError> {

    let from_asset_weight = WEIGHTS.load(deps.storage, from_asset)
        .map_err(|_| ContractError::AssetNotFound {})?;

    let to_asset_weight = WEIGHTS.load(deps.storage, to_asset)
        .map_err(|_| ContractError::AssetNotFound {})?;

    let from_asset_balance: Uint128 = deps.querier.query_wasm_smart::<BalanceResponse>(
        from_asset,
        &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
    )?.balance;

    // Subtract the 'to_asset' escrowed balance from the vault's total balance 
    // to return a smaller output.
    let to_asset_escrowed_balance: Uint128 = TOTAL_ESCROWED_ASSETS.load(
        deps.storage,
        to_asset
    )?;
    let to_asset_balance: Uint128 = deps.querier.query_wasm_smart::<BalanceResponse>(
        to_asset,
        &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
    )?.balance.checked_sub(to_asset_escrowed_balance)?;

    // Use a simplified formula for equal 'from' and 'to' weights (saves gas and is exact).
    if from_asset_weight == to_asset_weight {
        // NOTE: in this implementation it is not possible for either weight to be zero.
        return Ok(
            to_asset_balance.checked_mul(amount)? / from_asset_balance.checked_add(amount)?
        )
    }

    calc_combined_price_curves(
        amount.into(),
        from_asset_balance.into(),
        to_asset_balance.into(),
        from_asset_weight.into(),
        to_asset_weight.into()
    ).and_then(
        |val| TryInto::<Uint128>::try_into(val).map_err(|err| err.into())
    )
}


/// Volatile-specific handling of the confirmation of a successful asset swap.
/// 
/// This function adds security limit adjustment to the default implementation.
/// 
/// **DEV NOTE**: This function should never revert (for valid swap data).
/// 
/// # Arguments:
/// * `channel_id` - The swap's channel id.
/// * `to_account` - The recipient of the swap output.
/// * `u` - The units value of the swap.
/// * `escrow_amount` - The escrowed asset amount.
/// * `asset` - The swap source asset.
/// * `block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32).
/// 
pub fn on_send_asset_success_volatile(
    deps: &mut DepsMut,
    info: MessageInfo,
    channel_id: String,
    to_account: Binary,
    u: U256,
    escrow_amount: Uint128,
    asset: String,
    block_number_mod: u32
) -> Result<Response, ContractError> {

    // Execute the common 'success' logic
    let response = on_send_asset_success(
        deps,
        info,
        channel_id,
        to_account,
        u,
        escrow_amount,
        asset,
        block_number_mod
    )?;

    // Outgoing units are subtracted from the used limit capacity to avoid having a fixed
    // one-sided maximum daily cross chain volume. If the router was fraudulent, no one would 
    // execute an outgoing swap.

    // Minor optimization: avoid storage write if the used capacity is already at zero
    let used_capacity = USED_LIMIT_CAPACITY.load(deps.storage)?;
    if !used_capacity.is_zero() {
        USED_LIMIT_CAPACITY.save(deps.storage, &used_capacity.saturating_sub(u))?;
    }

    Ok(response)
}


/// Volatile-specific handling of the confirmation of a successful liquidity swap.
/// 
/// This function adds security limit adjustment to the default implementation.
/// 
/// **DEV NOTE**: This function should never revert (for valid swap data).
/// 
/// # Arguments:
/// * `channel_id` - The swap's channel id.
/// * `to_account` - The recipient of the swap output.
/// * `u` - The units value of the swap.
/// * `escrow_amount` - The escrowed liquidity amount.
/// * `block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32).
/// 
pub fn on_send_liquidity_success_volatile(
    deps: &mut DepsMut,
    info: MessageInfo,
    channel_id: String,
    to_account: Binary,
    u: U256,
    escrow_amount: Uint128,
    block_number_mod: u32
) -> Result<Response, ContractError> {

    // Execute the common 'success' logic
    let response = on_send_liquidity_success(
        deps,
        info,
        channel_id,
        to_account,
        u,
        escrow_amount,
        block_number_mod
    )?;

    // Outgoing units are subtracted from the used limit capacity to avoid having a fixed
    // one-sided maximum daily cross chain volume. If the router was fraudulent, no one would 
    // execute an outgoing swap.

    // Minor optimization: avoid storage write if the used capacity is already at zero
    let used_capacity = USED_LIMIT_CAPACITY.load(deps.storage)?;
    if !used_capacity.is_zero() {
        USED_LIMIT_CAPACITY.save(deps.storage, &used_capacity.saturating_sub(u))?;
    }

    Ok(response)
}


/// Allow governance to modify the vault asset weights.
/// 
/// **NOTE**: the weights cannot be set to 0 nor can they introduce a change larger than
/// `MAX_WEIGHT_ADJUSTMENT_FACTOR`.
/// 
/// **NOTE**: `target_timestamp` must be within `MIN_ADJUSTMENT_TIME_SECONDS` and
/// `MAX_ADJUSTMENT_TIME_SECONDS` from the current time.
/// 
/// **NOTE**: It is not recommended to update the weights if they have been initialized
/// to small values (<≈100), as the incremental `update_weights` function would result 
/// in large step updates which are undesirable (because of the low integer resolution).
/// 
/// # Arguments:
/// * `target_timestamp` - The time at which the weights update must be completed.
/// * `new_weights` - The new weights
/// 
pub fn set_weights(
    deps: &mut DepsMut,
    env: &Env,
    info: MessageInfo,
    target_timestamp: Uint64,
    new_weights: Vec<Uint128>
) -> Result<Response, ContractError> {

    // Only allow weight changes by the factory owner
    if info.sender != factory_owner(&deps.as_ref())? {
        return Err(ContractError::Unauthorized {});
    }

    // Check that 'target_timestamp' is within the defined acceptable bounds
    let current_time = Uint64::new(env.block.time.seconds());
    if
        target_timestamp < current_time + MIN_ADJUSTMENT_TIME_SECONDS ||
        target_timestamp > current_time + MAX_ADJUSTMENT_TIME_SECONDS
    {
        return Err(ContractError::InvalidTargetTime {});
    }

    // Check the new requested weights and store them
    let assets = ASSETS.load(deps.storage)?;
    if new_weights.len() != assets.len() {
        return Err(ContractError::InvalidParameters { reason: "Invalid weights count.".to_string() });
    }

    assets
        .iter()
        .zip(&new_weights)      // zip: weights.len() == current_weights.len() (checked above)
        .try_for_each(|(asset, new_weight)| -> Result<(), ContractError> {

            let current_weight = WEIGHTS.load(deps.storage, asset.as_ref())?;

            // Check that the new weight is neither 0 nor larger/smaller than the maximum 
            // allowed relative change
            if 
                *new_weight == Uint128::zero() ||
                *new_weight > current_weight.checked_mul(MAX_WEIGHT_ADJUSTMENT_FACTOR)? ||
                *new_weight < current_weight / MAX_WEIGHT_ADJUSTMENT_FACTOR
            {
                return Err(ContractError::InvalidWeight {});
            }

            TARGET_WEIGHTS.save(deps.storage, asset.as_ref(), new_weight)?;

            Ok(())

        })?;
    
    // Set the weight update time parameters
    WEIGHT_UPDATE_FINISH_TIMESTAMP_SECONDS.save(deps.storage, &target_timestamp)?;
    WEIGHT_UPDATE_TIMESTAMP_SECONDS.save(deps.storage, &current_time)?;

    Ok(
        Response::new()
            .add_event(
                set_weights_event(
                    target_timestamp,
                    new_weights
                )
            )
    )
}


/// Perform an incremental weights update.
/// 
/// **NOTE**: This algorithm is intended to introduce a gradual small-stepped update to the
/// asset weights. This will not be the case if the weights are initialized to small values.
/// 
/// **DEV-NOTE**: This function should be called at the beginning of weight-dependent functions.
/// 
/// # Arguments:
/// * `current_timestamp` - The current time.
/// 
pub fn update_weights(
    deps: &mut DepsMut,
    current_timestamp: Timestamp
) -> Result<(), ContractError> {

    // This algorithm incrementally adjusts the current weights to the target weights via 
    // linear interpolation.

    let current_timestamp = Uint64::new(current_timestamp.seconds());
    
    // Only run update logic if 'param_update_finish_timestamp' is set
    let param_update_finish_timestamp = WEIGHT_UPDATE_FINISH_TIMESTAMP_SECONDS.load(deps.storage)?;
    if param_update_finish_timestamp.is_zero() {
        return Ok(());
    }

    // Skip the update if the weights have already been updated on the same block
    let param_update_timestamp = WEIGHT_UPDATE_TIMESTAMP_SECONDS.load(deps.storage)?;
    if current_timestamp == param_update_timestamp {
        return Ok(());
    }

    let assets = ASSETS.load(deps.storage)?;
    let mut new_weight_sum = U256::zero();

    // If the 'param_update_finish_timestamp' has been reached, finish the weights update
    if current_timestamp >= param_update_finish_timestamp {

        // Set the weights equal to the target_weights
        assets
            .iter()
            .try_for_each(|asset| -> StdResult<()> {

                let new_weight = TARGET_WEIGHTS.load(deps.storage, asset.as_ref())?;

                new_weight_sum = new_weight_sum
                    .wrapping_add(U256::from(new_weight));  // 'wrapping_add' is safe because of casting to U256 (N*Uint128::MAX << U256::MAX for small N)

                WEIGHTS.save(deps.storage, asset.as_ref(), &new_weight)?;

                Ok(())

            })?;

        // Clear the 'param_update_finish_timestamp' to disable the update logic
        WEIGHT_UPDATE_FINISH_TIMESTAMP_SECONDS.save(
            deps.storage,
            &Uint64::zero()
        )?;

    }
    else {

        // Calculate and set the partial weight change
        assets
            .iter()
            .try_for_each(|asset| -> StdResult<()> {

                let current_weight = WEIGHTS.load(deps.storage, asset.as_ref())?;
                let target_weight = TARGET_WEIGHTS.load(deps.storage, asset.as_ref())?;

                // Skip the partial update if the weight has already reached the target
                if current_weight == target_weight {

                    new_weight_sum = new_weight_sum
                        .wrapping_add(U256::from(target_weight));  // 'wrapping_add' is safe because of casting to U256 (N*Uint128::MAX << U256::MAX for small N)

                    return Ok(());

                }

                // Compute the partial update (linear update)
                //     current_weight +/- [
                //        (distance to the target weight) x (time since last update) / (time from last update until update finish)
                //     ]
                let new_weight: Uint128;

                let time_since_last_update = Uint128::from(
                    current_timestamp.checked_sub(param_update_timestamp)?   // Using 'checked_sub' for extra precaution.
                );

                let total_update_time_remaining = Uint128::from(
                    param_update_finish_timestamp.checked_sub(param_update_timestamp)?  // Using 'checked_sub' for extra precaution.
                );

                if target_weight > current_weight {

                    let weight_delta = target_weight.wrapping_sub(current_weight);    // 'wrapping_sub' is safe, as it has been checked that 'target_weight' > 'current_weight'

                    new_weight = current_weight.wrapping_add(         // 'wrapping_add' is safe, as from the algorithm's design, the resulting 'new_weight' is <= 'target_weight'
                        weight_delta
                            .checked_mul(time_since_last_update)?
                            .div(total_update_time_remaining)
                    );
                }
                else {

                    let weight_delta = current_weight.wrapping_sub(target_weight);    // 'wrapping_sub' is safe, as it has been checked that 'current_weight' >= 'target_weight'

                    new_weight = current_weight.wrapping_sub(         // 'wrapping_sub' is safe, as from the algorithm's design, the resulting 'new_weight' is >= 'target_weight'
                        weight_delta
                            .checked_mul(time_since_last_update)?
                            .div(total_update_time_remaining)
                    );
                }

                new_weight_sum = new_weight_sum
                    .wrapping_add(U256::from(new_weight));  // 'wrapping_add' is safe because of casting to U256 (N*Uint128::MAX << U256::MAX for small N)

                // Update the weight
                WEIGHTS.save(deps.storage, asset.as_ref(), &new_weight)?;

                Ok(())

            })?;

    }
        
    // Update the maximum limit capacity
    MAX_LIMIT_CAPACITY.save(
        deps.storage,
        &new_weight_sum.wrapping_mul(fixed_point_math::LN2) // 'wrapping_mul' is safe as N*2^128 * ~2^60 << U256::MAX for small N
    )?;

    // Update the update timestamp
    WEIGHT_UPDATE_TIMESTAMP_SECONDS.save(
        deps.storage,
        &current_timestamp
    )?;

    Ok(())

}



// Query helpers ****************************************************************************************************************

/// Query a 'send_asset' calculation.
/// 
/// # Arguments:
/// * `from_asset` - The source asset.
/// * `amount` - The `from_asset` amount (excluding the vault fee).
/// 
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


/// Query a 'receive_asset' calculation.
/// 
/// # Arguments:
/// * `to_asset` - The target asset.
/// * `u` - The incoming units (in WAD notation).
/// 
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


/// Query a 'local_swap' calculation.
/// 
/// # Arguments:
/// * `from_asset` - The source asset.
/// * `to_asset` - The target asset.
/// * `amount` - The `from_asset` amount (excluding the vault fee).
/// 
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


/// Query the current limit capacity.
pub fn query_get_limit_capacity(
    deps: Deps,
    env: Env
) -> StdResult<GetLimitCapacityResponse> {

    Ok(
        GetLimitCapacityResponse {
            capacity: get_limit_capacity(&deps, env.block.time)?
        }
    )

}


/// Query an asset's target weight.
/// 
/// # Arguments:
/// * `asset` - The asset of which to query the target weight.
/// 
pub fn query_target_weight(
    deps: Deps,
    asset: String
) -> StdResult<TargetWeightResponse> {
    
    Ok(
        TargetWeightResponse {
            target_weight: TARGET_WEIGHTS.load(deps.storage, &asset)?
        }
    )

}


/// Query the weights update finish timestamp.
pub fn query_weights_update_finish_timestamp(
    deps: Deps
) -> StdResult<WeightsUpdateFinishTimestampResponse> {

    Ok(
        WeightsUpdateFinishTimestampResponse {
            timestamp: WEIGHT_UPDATE_FINISH_TIMESTAMP_SECONDS.load(deps.storage)?
        }
    )

}

