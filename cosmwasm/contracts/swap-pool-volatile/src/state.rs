use cosmwasm_std::{Addr, Uint128, DepsMut, Env, MessageInfo, Response, StdResult, CosmosMsg, to_binary, Deps, Binary};
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg, BalanceResponse};
use cw20_base::{contract::{execute_mint, execute_burn}};
use cw_storage_plus::Item;
use ethnum::{U256, AsI256};
use fixed_point_math_lib::fixed_point_math::{LN2, mul_wad_down, self, ln_wad, WAD, exp_wad};
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};
use swap_pool_common::{
    state::{
        ASSETS, MAX_ASSETS, WEIGHTS, INITIAL_MINT_AMOUNT, POOL_FEE, MAX_LIMIT_CAPACITY, USED_LIMIT_CAPACITY, CHAIN_INTERFACE,
        TOTAL_ESCROWED_LIQUIDITY, TOTAL_ESCROWED_ASSETS, is_connected, get_asset_index, update_limit_capacity,
        collect_governance_fee_message, compute_send_asset_hash, compute_send_liquidity_hash, create_asset_escrow,
        create_liquidity_escrow, on_send_asset_success, on_send_liquidity_success, total_supply, get_limit_capacity, USED_LIMIT_CAPACITY_TIMESTAMP, FACTORY,
    },
    ContractError, msg::{CalcSendAssetResponse, CalcReceiveAssetResponse, CalcLocalSwapResponse, GetLimitCapacityResponse}
};

use catalyst_ibc_interface::msg::ExecuteMsg as InterfaceExecuteMsg;

use crate::{calculation_helpers::{calc_price_curve_area, calc_price_curve_limit, calc_combined_price_curves, calc_price_curve_limit_share}, msg::{TargetWeightsResponse, WeightsUpdateFinishTimestampResponse}};

pub const TARGET_WEIGHTS: Item<Vec<u64>> = Item::new("catalyst-pool-target-weights");       //TODO use mapping instead? (see also WEIGHTS definition)
pub const WEIGHT_UPDATE_TIMESTAMP: Item<u64> = Item::new("catalyst-pool-weight-update-timestamp");
pub const WEIGHT_UPDATE_FINISH_TIMESTAMP: Item<u64> = Item::new("catalyst-pool-weight-update-finish-timestamp");

const MIN_ADJUSTMENT_TIME_NANOS    : u64 = 7 * 24 * 60 * 60 * 1000000000;     // 7 days
const MAX_ADJUSTMENT_TIME_NANOS    : u64 = 365 * 24 * 60 * 60 * 1000000000;   // 1 year
const MAX_WEIGHT_ADJUSTMENT_FACTOR : u64 = 10;

// Implement JsonSchema for U256, see https://graham.cool/schemars/examples/5-remote_derive/
//TODO VERIFY THIS IS CORRECT AND SAFE!
//TODO move to common place
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(remote = "U256")]
pub struct U256Def([u128; 2]);


pub fn initialize_swap_curves(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    assets: Vec<String>,
    weights: Vec<u64>,
    amp: u64,
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

    // Check that the amplification is correct (set to 1)
    if amp != 10u64.pow(18) {     //TODO maths WAD
        return Err(ContractError::InvalidAmplification {})
    }

    // Check the provided assets, assets balances and weights count
    if
        assets.len() == 0 || assets.len() > MAX_ASSETS ||
        weights.len() != assets.len()
    {
        return Err(ContractError::GenericError {}); //TODO error
    }

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
        return Err(ContractError::GenericError {}); //TODO error
    }

    // Validate and save weights
    if weights.iter().any(|weight| *weight == 0) {
        return Err(ContractError::GenericError {}); //TODO error
    }
    WEIGHTS.save(deps.storage, &weights)?;
    TARGET_WEIGHTS.save(deps.storage, &weights)?;               // Initialize the target_weights storage (values do not matter)
    WEIGHT_UPDATE_TIMESTAMP.save(deps.storage, &0u64)?;         //TODO move intialization to 'setup'?
    WEIGHT_UPDATE_FINISH_TIMESTAMP.save(deps.storage, &0u64)?;  //TODO move intialization to 'setup'?

    // Compute the security limit
    MAX_LIMIT_CAPACITY.save(
        deps.storage,
        &(LN2 * weights.iter().fold(
            U256::ZERO, |acc, next| acc + U256::from(*next)     // Overflow safe, as U256 >> u64    //TODO maths
        ))
    )?;
    USED_LIMIT_CAPACITY.save(deps.storage, &U256::ZERO)?;       //TODO move intialization to 'setup'?
    USED_LIMIT_CAPACITY_TIMESTAMP.save(deps.storage, &0u64)?;   //TODO move intialization to 'setup'?

    // Initialize escrow totals
    assets
        .iter()
        .map(|asset| TOTAL_ESCROWED_ASSETS.save(deps.storage, asset, &Uint128::zero()))
        .collect::<StdResult<Vec<_>>>()?;
    TOTAL_ESCROWED_LIQUIDITY.save(deps.storage, &Uint128::zero())?;

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




pub fn deposit_mixed(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    deposit_amounts: Vec<Uint128>,  //TODO EVM MISMATCH
    min_out: Uint128
) -> Result<Response, ContractError> {

    let assets = ASSETS.load(deps.storage)?;
    let weights = WEIGHTS.load(deps.storage)?;

    if deposit_amounts.len() != assets.len() {
        return Err(ContractError::GenericError {});     //TODO error
    }

    // Compute how much 'units' the assets are worth.
    // Iterate over the assets, weights and deposit_amounts)
    let u = assets.iter()
        .zip(weights)                           // zip: weights.len() == assets.len()
        .zip(&deposit_amounts)                  // zip: deposit_amounts.len() == assets.len()
        .try_fold(U256::ZERO, |acc, ((asset, weight), deposit_amount)| {

            // Save gas if the user provides no tokens for the specific asset
            if deposit_amount.is_zero() {
                return Ok(acc);
            }

            let pool_asset_balance = deps.querier.query_wasm_smart::<BalanceResponse>(
                asset,
                &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
            )?.balance;

            acc.checked_add(
                calc_price_curve_area(
                    U256::from(deposit_amount.u128()),
                    U256::from(pool_asset_balance.u128()),
                    U256::from(weight.clone())
                )?
            ).ok_or(ContractError::ArithmeticError {})
        })?;

    // Subtract the pool fee from U to prevent deposit and withdrawals being employed as a method of swapping.
    // To recude costs, the governance fee is not taken. This is not an issue as swapping via this method is 
    // disincentivized by its higher gas costs.
    let pool_fee = POOL_FEE.load(deps.storage)?;
    let u = fixed_point_math::mul_wad_down(u, fixed_point_math::WAD - U256::from(pool_fee))?;

    // Do not include the 'escrowed' pool tokens in the total supply of pool tokens (return less)
    let effective_supply = U256::from(total_supply(deps.as_ref())?.u128());

    // Derive the weight sum (w_sum) from the security limit capacity
    let w_sum = MAX_LIMIT_CAPACITY.load(deps.storage)? / fixed_point_math::LN2;

    // Compute the pool tokens to be minted.
    let out = Uint128::from(fixed_point_math::mul_wad_down(
        effective_supply,                                                       // Note 'effective_supply' is not WAD, hence result will not be either
        calc_price_curve_limit_share(u, w_sum)?
    )?.as_u128());      //TODO OVERFLOW

    // Check that the minimum output is honoured.
    if min_out > out {
        return Err(ContractError::ReturnInsufficient { out, min_out });
    }

    // Mint the pool tokens
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

    // Build messages to order the transfer of tokens from the depositor to the swap pool
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
        .add_attribute("to_account", info.sender.to_string())
        .add_attribute("mint", out)
        .add_attribute("assets", format_vec_for_event(deposit_amounts))  //TODO deposit_amounts event format
    )
}

pub fn withdraw_all(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    pool_tokens: Uint128,
    min_out: Vec<Uint128>,
) -> Result<Response, ContractError> {

    // Include the 'escrowed' pool tokens in the total supply of pool tokens of the pool
    let escrowed_pool_tokens = TOTAL_ESCROWED_LIQUIDITY.load(deps.storage)?;
    let effective_supply = total_supply(deps.as_ref())?.checked_add(escrowed_pool_tokens)?;

    // Burn the pool tokens of the withdrawer
    let sender = info.sender.to_string();
    let burn_response = execute_burn(deps.branch(), env.clone(), info.clone(), pool_tokens)?;

    // Compute the withdraw amounts
    let assets = ASSETS.load(deps.storage)?;

    if min_out.len() != assets.len() {
        return Err(ContractError::GenericError {});     //TODO error
    }

    let withdraw_amounts: Vec<Uint128> = assets
        .iter()
        .zip(&min_out)                                      // zip: assets.len() == min_out.len()
        .map(|(asset, asset_min_out)| {

            let escrowed_balance = TOTAL_ESCROWED_ASSETS.load(deps.storage, asset.as_str())?;
            
            let pool_asset_balance = deps.querier.query_wasm_smart::<BalanceResponse>(
                asset,
                &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
            )?.balance - escrowed_balance;

            //TODO use U256 for the calculation?
            let withdraw_amount = (pool_asset_balance * pool_tokens) / effective_supply;

            // Check that the minimum output is honoured.
            if *asset_min_out > withdraw_amount {
                //TODO include in error the asset?
                return Err(ContractError::ReturnInsufficient { out: withdraw_amount.clone(), min_out: *asset_min_out });
            };

            Ok(withdraw_amount)
        }).collect::<Result<Vec<Uint128>, ContractError>>()?;

    // Build messages to order the transfer of tokens from the swap pool to the depositor
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
        .add_attribute("to_account", info.sender.to_string())
        .add_attribute("burn", pool_tokens)
        .add_attribute("assets", format_vec_for_event(withdraw_amounts))  //TODO withdraw_amounts format
    )
    
}


pub fn withdraw_mixed(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    pool_tokens: Uint128,
    withdraw_ratio: Vec<u64>,
    min_out: Vec<Uint128>,
) -> Result<Response, ContractError> {

    // Include the 'escrowed' pool tokens in the total supply of pool tokens of the pool
    let escrowed_pool_tokens = TOTAL_ESCROWED_LIQUIDITY.load(deps.storage)?;
    let effective_supply = U256::from(
        total_supply(deps.as_ref())?.checked_add(escrowed_pool_tokens)?.u128()
    );

    // Burn the pool tokens of the withdrawer
    let sender = info.sender.to_string();
    let burn_response = execute_burn(deps.branch(), env.clone(), info.clone(), pool_tokens)?;

    // Derive the weight sum (w_sum) from the security limit capacity
    let w_sum = MAX_LIMIT_CAPACITY.load(deps.storage)? / fixed_point_math::LN2;

    // Compute the unit worth of the pool tokens.
    let mut u: U256 = fixed_point_math::ln_wad(
        fixed_point_math::div_wad_down(
            effective_supply,
            effective_supply - U256::from(pool_tokens.u128())  // Subtraction is underflow safe, as the above 'execute_burn' guarantees that 'pool_tokens' is contained in 'effective_supply'
        )?.as_i256()                                           // Casting my overflow to a negative value. In that case, 'ln_wad' will fail.
    )?.as_u256()                                               // Casting is safe, as ln is computed of values >= 1, hence output is always positive
        .checked_mul(w_sum).ok_or(ContractError::ArithmeticError {})?;

    // Compute the withdraw amounts
    let assets = ASSETS.load(deps.storage)?;
    let weights = WEIGHTS.load(deps.storage)?;

    if withdraw_ratio.len() != assets.len() || min_out.len() != assets.len() {
        return Err(ContractError::GenericError {})
    }

    let withdraw_amounts: Vec<Uint128> = assets
        .iter()
        .zip(weights)                       // zip: weights.len() == assets.len()    
        .zip(&withdraw_ratio)               // zip: withdraw_ratio.len() == assets.len()
        .zip(&min_out)                      // zip: min_out.len() == assets.len()
        .map(|(((asset, weight), asset_withdraw_ratio), asset_min_out)| {

            let escrowed_balance = TOTAL_ESCROWED_ASSETS.load(deps.storage, asset.as_ref())?;

            // Calculate the units allocated for the specific asset
            let units_for_asset = fixed_point_math::mul_wad_down(u, U256::from(*asset_withdraw_ratio))?;
            if units_for_asset == U256::ZERO {

                // There should not be a non-zero withdraw ratio after a withdraw ratio of 1 (protect against user error)
                if *asset_withdraw_ratio != 0 {
                    return Err(ContractError::WithdrawRatioNotZero { ratio: *asset_withdraw_ratio }) 
                };

                // Check that the minimum output is honoured.
                if asset_min_out != Uint128::zero() {
                    return Err(ContractError::ReturnInsufficient { out: Uint128::zero(), min_out: *asset_min_out })
                };

                return Ok(Uint128::zero());
            }

            // Subtract the units used from the total units amount. This will underflow for malicious withdraw ratios (i.e. ratios > 1).
            u = u.checked_sub(units_for_asset).ok_or(ContractError::ArithmeticError {})?;
        
            // Get the pool asset balance (subtract the escrowed assets to return less)
            let pool_asset_balance = deps.querier.query_wasm_smart::<BalanceResponse>(
                asset,
                &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
            )?.balance - escrowed_balance;

            // Calculate the asset amount corresponding to the asset units
            let withdraw_amount = Uint128::from(
                calc_price_curve_limit(
                    units_for_asset,
                    U256::from(pool_asset_balance.u128()),
                    U256::from(weight)
                )?.as_u128()        // TODO unsafe overflow
            );

            // Check that the minimum output is honoured.
            if *asset_min_out > withdraw_amount {
                return Err(ContractError::ReturnInsufficient { out: withdraw_amount.clone(), min_out: *asset_min_out });
            };

            Ok(withdraw_amount)
        }).collect::<Result<Vec<Uint128>, ContractError>>()?;

    // Make sure all units have been consumed
    if u != U256::ZERO { return Err(ContractError::UnusedUnitsAfterWithdrawal { units: u }) };       //TODO error

    // Build messages to order the transfer of tokens from the swap pool to the depositor
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
        .add_attribute("to_account", info.sender.to_string())
        .add_attribute("burn", pool_tokens)
        .add_attribute("assets", format_vec_for_event(withdraw_amounts))  //TODO withdraw_amounts format
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

    update_weights(deps, env.block.time.nanos())?;

    let pool_fee: Uint128 = mul_wad_down(            //TODO alternative to not have to use U256 conversion? (or wrapper?)
        U256::from(amount.u128()),
        U256::from(POOL_FEE.load(deps.storage)?)
    )?.as_u128().into();    // Casting safe, as fee < amount, and amount is Uint128

    // Calculate the return value
    let out: Uint128 = calc_local_swap(
        &deps.as_ref(),
        env.clone(),
        &from_asset,
        &to_asset,
        amount - pool_fee
    )?;

    if min_out > out {
        return Err(ContractError::ReturnInsufficient { out, min_out });
    }

    // Build message to transfer input assets to the pool
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
        env,
        from_asset.clone(),
        pool_fee
    )?;

    // Build response
    let mut response = Response::new()
        .add_message(transfer_from_asset_msg)
        .add_message(transfer_to_asset_msg);

    if let Some(msg) = collect_governance_fee_message {
        response = response.add_message(msg);
    }

    Ok(response
        .add_attribute("to_account", info.sender.to_string())
        .add_attribute("from_asset", from_asset)
        .add_attribute("to_asset", to_asset)
        .add_attribute("from_amount", amount)
        .add_attribute("to_amount", out)
    )
}


pub fn send_asset(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: String,
    to_pool: Binary,
    to_account: Binary,
    from_asset: String,
    to_asset_index: u8,
    amount: Uint128,
    min_out: U256,
    fallback_account: String,   //TODO EVM mismatch
    calldata: Binary
) -> Result<Response, ContractError> {

    // Only allow connected pools
    if !is_connected(&deps.as_ref(), &channel_id, to_pool.clone()) {
        return Err(ContractError::PoolNotConnected { channel_id, pool: to_pool })
    }

    // Make sure 'to_pool' and 'to_account have the correct length
    if to_pool.len() != 65 {                            //TODO use global const variable for address length
        return Err(ContractError::GenericError {});     //TODO error
    }

    if to_account.len() != 65 {                         //TODO use global const variable for address length
        return Err(ContractError::GenericError {});     //TODO error
    }

    update_weights(deps, env.block.time.nanos())?;

    let pool_fee: Uint128 = mul_wad_down(            //TODO alternative to not have to use U256 conversion? (or wrapper?)
        U256::from(amount.u128()),
        U256::from(POOL_FEE.load(deps.storage)?)
    )?.as_u128().into();    // Casting safe, as fee < amount, and amount is Uint128

    // Calculate the group-specific units bought
    let u = calc_send_asset(
        &deps.as_ref(),
        env.clone(),
        &from_asset,
        amount - pool_fee
    )?;

    let block_number = env.block.height as u32;
    let send_asset_hash = compute_send_asset_hash(
        to_account.as_slice(),
        u,
        amount - pool_fee,
        &from_asset,
        block_number
    );

    create_asset_escrow(
        deps,
        send_asset_hash.clone(),
        amount - pool_fee,
        &from_asset,
        fallback_account
    )?;

    // Build message to transfer input assets to the pool
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
        env,
        from_asset.clone(),
        pool_fee
    )?;

    // Build message to 'send' the asset via the IBC interface
    let send_cross_chain_asset_msg = InterfaceExecuteMsg::SendCrossChainAsset {
        channel_id,
        to_pool: to_pool.clone(),
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
            contract_addr: chain_interface.ok_or(ContractError::PoolHasNoInterface {})?.to_string(),
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
        .add_attribute("to_pool", to_pool.to_base64())
        .add_attribute("to_account", to_account.to_base64())
        .add_attribute("from_asset", from_asset)
        .add_attribute("to_asset_index", to_asset_index.to_string())
        .add_attribute("from_amount", amount)
        .add_attribute("fee", pool_fee)                     //TODO review once implemented on EVM
        .add_attribute("units", u.to_string())
        .add_attribute("min_out", min_out.to_string())      //TODO review string format
    )
}

pub fn receive_asset(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: String,
    from_pool: Binary,
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

    // Only allow connected pools
    if !is_connected(&deps.as_ref(), &channel_id, from_pool.clone()) {
        return Err(ContractError::PoolNotConnected { channel_id, pool: from_pool })
    }

    update_weights(deps, env.block.time.nanos())?;

    let assets = ASSETS.load(deps.storage)?;
    let to_asset = assets
        .get(to_asset_index as usize)
        .ok_or(ContractError::GenericError {})?
        .clone(); //TODO error

    update_limit_capacity(deps, env.block.time, u)?;

    let out = calc_receive_asset(&deps.as_ref(), env.clone(), to_asset.as_str(), u)?;
    

    if min_out > out {
        return Err(ContractError::ReturnInsufficient { out, min_out });
    }

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
        .add_attribute("from_pool", from_pool.to_base64())
        .add_attribute("to_account", to_account)
        .add_attribute("to_asset", to_asset)
        .add_attribute("units", u.to_string())  //TODO format of .to_string()?
        .add_attribute("to_amount", out)
        .add_attribute("from_amount", from_amount.to_string())
        .add_attribute("from_asset", from_asset.to_base64())
        .add_attribute("from_block_number_mod", from_block_number_mod.to_string())  //TODO format
    )
}

pub fn send_liquidity(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: String,
    to_pool: Binary,
    to_account: Binary,
    amount: Uint128,            //TODO EVM mismatch
    min_pool_tokens: U256,
    min_reference_asset: U256,
    fallback_account: String,   //TODO EVM mismatch
    calldata: Binary
) -> Result<Response, ContractError> {

    // Only allow connected pools
    if !is_connected(&deps.as_ref(), &channel_id, to_pool.clone()) {
        return Err(ContractError::PoolNotConnected { channel_id, pool: to_pool })
    }

    // Make sure 'to_pool' and 'to_account have the correct length
    if to_pool.len() != 65 {                            //TODO use global const variable for address length
        return Err(ContractError::GenericError {});     //TODO error
    }

    if to_account.len() != 65 {                         //TODO use global const variable for address length
        return Err(ContractError::GenericError {});     //TODO error
    }

    // Update weights
    update_weights(deps, env.block.time.nanos())?;

    // Include the 'escrowed' pool tokens in the total supply of pool tokens of the pool
    let escrowed_pool_tokens = TOTAL_ESCROWED_LIQUIDITY.load(deps.storage)?;
    let effective_supply = U256::from(total_supply(deps.as_ref())?.u128()) 
        + U256::from(escrowed_pool_tokens.u128());        // Addition is overflow safe because of casting into U256

    // Burn the pool tokens of the sender
    execute_burn(deps.branch(), env.clone(), info, amount)?;

    // Derive the weight sum (w_sum) from the security limit capacity
    let w_sum = MAX_LIMIT_CAPACITY.load(deps.storage)? / fixed_point_math::LN2;

    // Compute the unit value of the provided poolTokens
    // This step simplifies withdrawing and swapping into a single step
    let u = fixed_point_math::ln_wad(
        fixed_point_math::div_wad_down(
            effective_supply,
            effective_supply - U256::from(amount.u128())   // subtraction is safe, as 'amount' is always contained in 'effective_supply'
        )?.as_i256()                                         // if casting overflows into a negative value, posterior 'ln' calc will fail
    )?.as_u256()                                         // casting safe as 'ln' is computed of a value >= 1 (hence result always positive)
        .checked_mul(w_sum)
        .ok_or(ContractError::ArithmeticError {})?;

    // Compute the hash of the 'send_liquidity' transaction
    let block_number = env.block.height as u32;
    let send_liquidity_hash = compute_send_liquidity_hash(
        to_account.as_slice(),
        u,
        amount,
        block_number
    );

    // Escrow the pool tokens
    create_liquidity_escrow(
        deps,
        send_liquidity_hash.clone(),
        amount,
        fallback_account
    )?;

    // Build message to 'send' the liquidity via the IBC interface
    let send_cross_chain_asset_msg = InterfaceExecuteMsg::SendCrossChainLiquidity {
        channel_id,
        to_pool: to_pool.clone(),
        to_account: to_account.clone(),
        u,
        min_pool_tokens,
        min_reference_asset,
        from_amount: amount,
        block_number,
        calldata
    };
    let chain_interface = CHAIN_INTERFACE.load(deps.storage)?;
    let send_liquidity_execute_msg = CosmosMsg::Wasm(
        cosmwasm_std::WasmMsg::Execute {
            contract_addr: chain_interface.as_ref().ok_or(ContractError::PoolHasNoInterface {})?.to_string(),
            msg: to_binary(&send_cross_chain_asset_msg)?,
            funds: vec![]
        }
    );

    //TODO add min_out? (it is present on send_asset)
    Ok(Response::new()
        .add_message(send_liquidity_execute_msg)
        .add_attribute("to_pool", to_pool.to_base64())
        .add_attribute("to_account", to_account.to_base64())
        .add_attribute("from_amount", amount)
        .add_attribute("units", u.to_string())
        .add_attribute("min_pool_tokens", min_pool_tokens.to_string())
        .add_attribute("min_reference_asset", min_reference_asset.to_string())
    )
}

pub fn receive_liquidity(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: String,
    from_pool: Binary,
    to_account: String,
    u: U256,
    min_pool_tokens: Uint128,
    min_reference_asset: Uint128,       //TODO type
    from_amount: U256,
    from_block_number_mod: u32,
    calldata_target: Option<Addr>,
    calldata: Option<Binary>
) -> Result<Response, ContractError> {

    // Only allow the 'chain_interface' to invoke this function
    if Some(info.sender) != CHAIN_INTERFACE.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    // Only allow connected pools
    if !is_connected(&deps.as_ref(), &channel_id, from_pool.clone()) {
        return Err(ContractError::PoolNotConnected { channel_id, pool: from_pool })
    }

    update_weights(deps, env.block.time.nanos())?;

    update_limit_capacity(deps, env.block.time, u)?;

    // Derive the weight sum (w_sum) from the security limit capacity
    let w_sum = MAX_LIMIT_CAPACITY.load(deps.storage)? / fixed_point_math::LN2;

    // Do not include the 'escrowed' pool tokens in the total supply of pool tokens of the pool (return less)
    let effective_supply = U256::from(total_supply(deps.as_ref())?.u128());

    // Use 'calc_price_curve_limit_share' to get the % of pool tokens that should be minted (in WAD terms)
    // Multiply by 'effective_supply' to get the absolute amount (not in WAD terms) using 'mul_wad_down' so
    // that the result is also NOT in WAD terms.
    let out = fixed_point_math::mul_wad_down(
        calc_price_curve_limit_share(u, w_sum)?,
        effective_supply
    ).map(|val| Uint128::from(val.as_u128()))?;     //TODO OVERFLOW when casting U256 to Uint128. Theoretically calc_price_curve_limit_share < 1, hence casting is safe

    if min_pool_tokens > out {
        return Err(ContractError::ReturnInsufficient { out, min_out: min_pool_tokens });
    }

    if !min_reference_asset.is_zero() {

        let assets = ASSETS.load(deps.storage)?;
        let weights = WEIGHTS.load(deps.storage)?;

        // Compute the vault reference amount: product(balance(i)**weight(i))**(1/weights_sum)
        // The direct calculation of this value would overflow, hence it is calculated as:
        //      exp( sum( ln(balance(i)) * weight(i) ) / weights_sum )

        // Compute first: sum( ln(balance(i)) * weight(i) )
        let weighted_balance_sum = assets.iter()
            .zip(weights)       // zip: weights.len() == assets.len()
            .try_fold(U256::ZERO, |acc, (asset, weight)| {

                let pool_asset_balance = deps.querier.query_wasm_smart::<BalanceResponse>(
                    asset,
                    &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
                )?.balance;

                acc.checked_add(
                    ln_wad(     // TODO what if the pool gets depleted ==> ln(0)
                        pool_asset_balance.u128().as_i256() * WAD.as_i256()     // i256 casting: 'pool_asset_balance * WAD' always fits in an I256 (~2^128 * ~2^64)
                    )?.as_u256()                                                // u256 casting: 'pool_asset_balance * WAD' >= WAD (for balance != 0), hence 'ln_wad' return is always positive //TODO review 0 condition
                    .checked_mul(U256::from(weight)).ok_or(ContractError::ArithmeticError {})?
                ).ok_or(ContractError::ArithmeticError {})
            })?;

        // Finish the calculation: exp( 'weighted_balance_sum' / weights_sum )
        let vault_reference_amount = exp_wad(
            (weighted_balance_sum / w_sum)          // Division is safe, as w_sum is never 0
                .as_i256()                          // If casting overflows to a negative number, the result of the exponent will be 0, which will cause the min_reference_asset check to fail //TODO denial of service attack?
        )?.as_u256() / WAD;                         // Division is safe, as WAD != 0

        // Compute the fraction of the 'vault_reference_amount' that the swapper owns.
        // Include the escrowed pool tokens in the total supply to ensure that even if all the ongoing transactions revert, the specified min_reference_asset is fulfilled.
        // Include the pool tokens as they are going to be minted.
        let escrowed_pool_tokens = TOTAL_ESCROWED_LIQUIDITY.load(deps.storage)?;
        let user_reference_amount = Uint128::from((     //TODO is the use of Uint128/U256 correct in this calculation?
            (vault_reference_amount * U256::from(out.u128()))/(effective_supply + U256::from(escrowed_pool_tokens.u128()) + U256::from(out.u128()))
        ).as_u128());        //TODO casting overflow

        if min_reference_asset > user_reference_amount {
            return Err(ContractError::ReturnInsufficient { out: user_reference_amount, min_out: min_reference_asset });
        }

    }

    // Validate the to_account
    deps.api.addr_validate(&to_account)?;   //TODO is this necessary? Isn't the account validated by `execute_mint`?

    // Mint the pool tokens
    let mint_response = execute_mint(
        deps.branch(),
        env.clone(),
        MessageInfo {
            sender: env.contract.address.clone(),   // This contract itself is the one 'sending' the mint operation
            funds: vec![],
        },
        to_account.clone(),
        out
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
        .add_attribute("from_pool", from_pool.to_base64())
        .add_attribute("to_account", to_account)
        .add_attribute("units", u.to_string())  //TODO format of .to_string()?
        .add_attribute("to_amount", out)
        .add_attribute("from_amount", from_amount.to_string())
        .add_attribute("from_block_number_mod", from_block_number_mod.to_string())  //TODO format
        .add_events(mint_response.events)       //TODO overhaul
    )
}



pub fn calc_send_asset(
    deps: &Deps,
    env: Env,
    from_asset: &str,
    amount: Uint128
) -> Result<U256, ContractError> {

    let assets = ASSETS.load(deps.storage)?;
    let weights = WEIGHTS.load(deps.storage)?;

    let from_asset_index: usize = get_asset_index(&assets, from_asset.as_ref())?;
    let from_asset_balance: Uint128 = deps.querier.query_wasm_smart::<BalanceResponse>(
        from_asset,
        &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
    )?.balance;
    let from_asset_weight = weights[from_asset_index];

    calc_price_curve_area(
        amount.u128().into(),
        from_asset_balance.u128().into(),
        U256::from(from_asset_weight),
    ).map_err(|_| ContractError::GenericError {})
}

pub fn calc_receive_asset(
    deps: &Deps,
    env: Env,
    to_asset: &str,
    u: U256
) -> Result<Uint128, ContractError> {

    let assets = ASSETS.load(deps.storage)?;
    let weights = WEIGHTS.load(deps.storage)?;

    let to_asset_index: usize = get_asset_index(&assets, to_asset.as_ref())?;
    let to_asset_escrowed_balance: Uint128 = TOTAL_ESCROWED_ASSETS.load(
        deps.storage,
        to_asset
    )?;
    let to_asset_balance: Uint128 = deps.querier.query_wasm_smart::<BalanceResponse>(
        to_asset,
        &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
    )?.balance.checked_sub(to_asset_escrowed_balance)?;      // pool balance minus escrowed balance
    let to_asset_weight = weights[to_asset_index];
    
    calc_price_curve_limit(
        u,
        to_asset_balance.u128().into(),
        U256::from(to_asset_weight),
    ).map(
        |val| Uint128::from(val.as_u128())      //TODO! .as_u128 may overflow silently
    ).map_err(
        |_| ContractError::GenericError {}
    )
}

pub fn calc_local_swap(
    deps: &Deps,
    env: Env,
    from_asset: &str,
    to_asset: &str,
    amount: Uint128
) -> Result<Uint128, ContractError> {

    let assets = ASSETS.load(deps.storage)?;
    let weights = WEIGHTS.load(deps.storage)?;

    let from_asset_index: usize = get_asset_index(&assets, from_asset.as_ref())?;
    let from_asset_balance: Uint128 = deps.querier.query_wasm_smart::<BalanceResponse>(
        from_asset,
        &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
    )?.balance;
    let from_asset_weight = weights[from_asset_index];

    let to_asset_index: usize = get_asset_index(&assets, to_asset.as_ref())?;
    let to_asset_escrowed_balance: Uint128 = TOTAL_ESCROWED_ASSETS.load(
        deps.storage,
        to_asset
    )?;
    let to_asset_balance: Uint128 = deps.querier.query_wasm_smart::<BalanceResponse>(
        to_asset,
        &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
    )?.balance.checked_sub(to_asset_escrowed_balance)?;      // pool balance minus escrowed balance
    let to_asset_weight = weights[to_asset_index];

    //TODO move condition into 'calc_combined_price_curves'?
    if from_asset_weight == to_asset_weight {
        // Saves gas and is exact
        // NOTE: If W_A == 0 and W_B == 0 then W_A == W_B and the calculation will not fail (unlike with the full calculation).
        // This cannot be used to extract an asset from the pool using an asset that is not in the pool, as all assets in 
        // the pool have a non-zero weight.
        return Ok(
            to_asset_balance.checked_mul(amount)? / from_asset_balance.checked_add(amount)?
        )
    }

    calc_combined_price_curves(
        amount.u128().into(),
        from_asset_balance.u128().into(),
        to_asset_balance.u128().into(),
        U256::from(from_asset_weight),
        U256::from(to_asset_weight)
    ).map(
        |val| Uint128::from(val.as_u128())  //TODO! silent overflow possible!
    ).map_err(
        |_| ContractError::GenericError {}
    ) 
}


pub fn on_send_asset_success_volatile(
    deps: &mut DepsMut,
    info: MessageInfo,
    channel_id: String,
    to_account: Binary,
    u: U256,
    amount: Uint128,
    asset: String,
    block_number_mod: u32
) -> Result<Response, ContractError> {

    let response = on_send_asset_success(
        deps,
        info,
        channel_id,
        to_account,
        u,
        amount,
        asset,
        block_number_mod
    )?;

    let used_capacity = USED_LIMIT_CAPACITY.load(deps.storage)?;

    // Minor optimization: avoid storage write if the used capacity is already at zero
    if used_capacity != U256::ZERO {
        USED_LIMIT_CAPACITY.save(deps.storage, &used_capacity.saturating_sub(u))?;
    }

    Ok(response)
}

pub fn on_send_liquidity_success_volatile(
    deps: &mut DepsMut,
    info: MessageInfo,
    channel_id: String,
    to_account: Binary,
    u: U256,
    amount: Uint128,
    block_number_mod: u32
) -> Result<Response, ContractError> {

    let response = on_send_liquidity_success(
        deps,
        info,
        channel_id,
        to_account,
        u,
        amount,
        block_number_mod
    )?;

    let used_capacity = USED_LIMIT_CAPACITY.load(deps.storage)?;

    // Minor optimization: avoid storage write if the used capacity is already at zero
    if used_capacity != U256::ZERO {
        USED_LIMIT_CAPACITY.save(deps.storage, &used_capacity.saturating_sub(u))?;
    }

    Ok(response)
}


pub fn set_weights(         //TODO EVM mismatch arguments order
    deps: &mut DepsMut,
    env: &Env,
    weights: Vec<u64>,      //TODO rename new_weights
    target_timestamp: u64   //TODO EVM mismatch (targetTime)
) -> Result<Response, ContractError> {

    let current_weights = WEIGHTS.load(deps.storage)?;

    // Check 'target_timestamp' is within the defined acceptable bounds
    let current_time = env.block.time.nanos();
    if
        target_timestamp < current_time + MIN_ADJUSTMENT_TIME_NANOS ||
        target_timestamp > current_time + MAX_ADJUSTMENT_TIME_NANOS
    {
        return Err(ContractError::GenericError {});  //TODO error
    }

    // Check the new requested weights and store them
    if weights.len() != current_weights.len() {
        return Err(ContractError::GenericError {});     //TODO error
    }

    let target_weights = current_weights
        .iter()
        .zip(&weights)                                      // zip: weights.len() == current_weights.len()
        .map(|(current_weight, new_weight)| {

            // Check that the new weight is neither 0 nor larger than the maximum allowed relative change
            if 
                *new_weight == 0 ||
                *new_weight > current_weight
                    .checked_mul(MAX_WEIGHT_ADJUSTMENT_FACTOR).ok_or(ContractError::ArithmeticError {})? ||
                *new_weight < current_weight / MAX_WEIGHT_ADJUSTMENT_FACTOR     //TODO fix: replace MAX_WEIGHT_ADJUSTMENT_FACTOR with MIN_WEIGHT_ADJUSTMENT_FACTOR
            {
                return Err(ContractError::GenericError {});     //TODO error
            }

            Ok(*new_weight)

        }).collect::<Result<Vec<u64>, ContractError>>()?;
    TARGET_WEIGHTS.save(deps.storage, &target_weights)?;
    
    // Set the weight update time parameters
    WEIGHT_UPDATE_FINISH_TIMESTAMP.save(deps.storage, &target_timestamp)?;
    WEIGHT_UPDATE_TIMESTAMP.save(deps.storage, &current_time)?;

    Ok(Response::new()
        .add_attribute("weights", format!("{:?}", weights))
        .add_attribute("target_timestamp", target_timestamp.to_string())
    )
}

pub fn update_weights(
    deps: &mut DepsMut,
    current_timestamp: u64
) -> Result<(), ContractError> {
    
    // Only run update logic if 'param_update_finish_timestamp' is set
    let param_update_finish_timestamp = WEIGHT_UPDATE_FINISH_TIMESTAMP.load(deps.storage)?;
    if param_update_finish_timestamp == 0 {
        return Ok(());
    }

    // Skip the update if the weights have already been updated on the same block
    let param_update_timestamp = WEIGHT_UPDATE_TIMESTAMP.load(deps.storage)?;
    if current_timestamp == param_update_timestamp {
        return Ok(());
    }

    let target_weights = TARGET_WEIGHTS.load(deps.storage)?;

    let new_weights: Vec<u64>;
    let mut new_weight_sum = U256::ZERO;

    // If the 'param_update_finish_timestamp' has been reached, finish the weights update
    if current_timestamp >= param_update_finish_timestamp {

        //TODO: why using 'map' here? use 'fold' or 'forEach'
        // Set the weights equal to the target_weights
        new_weights = target_weights
            .iter()
            .map(|target_weight| {

                new_weight_sum = new_weight_sum
                    .checked_add(U256::from(*target_weight))
                    .ok_or(ContractError::ArithmeticError {})?;

                Ok(*target_weight)

            }).collect::<Result<Vec<u64>, ContractError>>()?;

        // Clear the 'param_update_finish_timestamp' to disable the update logic
        WEIGHT_UPDATE_FINISH_TIMESTAMP.save(
            deps.storage,
            &0
        )?;

    }
    else {

        // Calculate and set the partial weight change
        let weights = WEIGHTS.load(deps.storage)?;
        new_weights = weights
            .iter()
            .zip(&target_weights)                                       // zip: target_weights.len() == weights.len()
            .map(|(current_weight, target_weight)| {

                // Skip the partial update if the weight has already reached the target
                if current_weight == target_weight {

                    new_weight_sum = new_weight_sum
                        .checked_add(U256::from(*target_weight))
                        .ok_or(ContractError::ArithmeticError {})?;

                    return Ok(*target_weight);

                }

                // Compute the partial update (linear update)
                //     current_weight +/- [
                //        (distance to the target weight) x (time since last update) / (time from last update until update finish)
                //     ]
                let new_weight: u64;
                if target_weight > current_weight {
                    new_weight = current_weight + (
                        (target_weight - current_weight)
                            .checked_mul(current_timestamp - param_update_timestamp)
                            .ok_or(ContractError::ArithmeticError {})?
                            .div_euclid(param_update_finish_timestamp - param_update_timestamp)
                    );
                }
                else {
                    new_weight = current_weight - (
                        (current_weight - target_weight)
                        .checked_mul(current_timestamp - param_update_timestamp)
                        .ok_or(ContractError::ArithmeticError {})?
                        .div_euclid(param_update_finish_timestamp - param_update_timestamp)
                    );
                }

                new_weight_sum = new_weight_sum
                    .checked_add(U256::from(new_weight))
                    .ok_or(ContractError::ArithmeticError {})?;

                Ok(*target_weight)

            }).collect::<Result<Vec<u64>, ContractError>>()?;

    }

    // Update weights
    WEIGHTS.save(
        deps.storage,
        &new_weights
    )?;
        
    // Update the maximum limit capacity
    MAX_LIMIT_CAPACITY.save(
        deps.storage,
        &new_weight_sum.checked_mul(fixed_point_math::LN2).ok_or(ContractError::ArithmeticError {})?
    )?;

    // Update the update timestamp
    WEIGHT_UPDATE_TIMESTAMP.save(
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


pub fn query_target_weights(
    deps: Deps
) -> StdResult<TargetWeightsResponse> {
    
    Ok(
        TargetWeightsResponse {
            target_weights: TARGET_WEIGHTS.load(deps.storage)?
        }
    )

}

pub fn query_weights_update_finish_timestamp(
    deps: Deps
) -> StdResult<WeightsUpdateFinishTimestampResponse> {

    Ok(
        WeightsUpdateFinishTimestampResponse {
            timestamp: WEIGHT_UPDATE_FINISH_TIMESTAMP.load(deps.storage)?
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