use cosmwasm_std::{Uint128, DepsMut, Env, MessageInfo, Response, StdResult, CosmosMsg, to_binary, Deps, Binary, Uint64, Timestamp};
use cw20_base::contract::{execute_mint, execute_burn};
use cw_storage_plus::Item;
use catalyst_ibc_interface::msg::ExecuteMsg as InterfaceExecuteMsg;
use catalyst_types::{U256, I256, u256};
use catalyst_vault_common::{
    ContractError,
    event::{local_swap_event, send_asset_event, receive_asset_event, send_liquidity_event, receive_liquidity_event, deposit_event, withdraw_event, cw20_response_to_standard_event},
    msg::{CalcSendAssetResponse, CalcReceiveAssetResponse, CalcLocalSwapResponse, GetLimitCapacityResponse}, 
    state::{FACTORY, MAX_ASSETS, WEIGHTS, INITIAL_MINT_AMOUNT, VAULT_FEE, MAX_LIMIT_CAPACITY, USED_LIMIT_CAPACITY, CHAIN_INTERFACE, TOTAL_ESCROWED_LIQUIDITY, TOTAL_ESCROWED_ASSETS, is_connected, update_limit_capacity, collect_governance_fee_message, compute_send_asset_hash, compute_send_liquidity_hash, create_asset_escrow, create_liquidity_escrow, on_send_asset_success, total_supply, get_limit_capacity, on_send_asset_failure, on_send_liquidity_failure, factory_owner, initialize_escrow_totals, initialize_limit_capacity, create_on_catalyst_call_msg}, asset::{Asset, AssetTrait, VaultAssets, VaultAssetsTrait}
};
use fixed_point_math::{self, WAD, WADWAD, mul_wad_down, pow_wad, div_wad_up, div_wad_down, mul_wad_up};
use std::ops::Div;

use crate::{
    calculation_helpers::{calc_price_curve_area, calc_price_curve_limit, calc_combined_price_curves, calc_price_curve_limit_share, calc_weighted_alpha_0_ampped}, 
    event::set_amplification_event,
    msg::{TargetAmplificationResponse, AmplificationUpdateFinishTimestampResponse, Balance0Response, AmplificationResponse, UnitTrackerResponse}
};


// Amplified-vault specific storage variables and constants
pub const ONE_MINUS_AMP : Item<I256> = Item::new("catalyst-vault-amplified-one-minus-amp");
pub const UNIT_TRACKER  : Item<I256> = Item::new("catalyst-vault-amplified-unit-tracker");

const SMALL_SWAP_RATIO  : Uint128 = Uint128::new(1000000000000u128);   // 1e12
const SMALL_SWAP_RETURN : U256 = u256!("950000000000000000");          // 0.95 * WAD

// Amplification adjustment storage variables and constants
pub const TARGET_ONE_MINUS_AMP: Item<I256> = Item::new("catalyst-vault-amplified-target-one-minus-amp");
pub const AMP_UPDATE_TIMESTAMP_SECONDS: Item<Uint64> = Item::new("catalyst-vault-amplified-amp-update-timestamp");
pub const AMP_UPDATE_FINISH_TIMESTAMP_SECONDS: Item<Uint64> = Item::new("catalyst-vault-amplified-amp-update-finish-timestamp");

const MIN_ADJUSTMENT_TIME_SECONDS : Uint64 = Uint64::new(7 * 24 * 60 * 60);     // 7 days
const MAX_ADJUSTMENT_TIME_SECONDS : Uint64 = Uint64::new(365 * 24 * 60 * 60);   // 1 year
const MAX_AMP_ADJUSTMENT_FACTOR   : Uint64 = Uint64::new(2);


/// Initialize the vault swap curves.
/// 
/// The initial asset amounts must be sent to the vault before calling this function.
/// Only the instantiator of the vault may invoke this function (i.e. the `FACTORY`).
/// This should be handled by the Catalyst vault factory.
/// 
/// # Arguments:
/// * `assets` - The list of the assets that are to be supported by the vault.
/// * `weights` - The weights applied to the assets. These should be set such that
/// all weight-asset products are equal (to effectively set 1:1 asset prices).
/// * `amp` - The amplification value applied to the vault (should be < WAD).
/// * `depositor` - The account that will receive the initial vault tokens.
/// 
pub fn initialize_swap_curves(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    assets: Vec<Asset>,
    weights: Vec<Uint128>,
    amp: Uint64,
    depositor: String
) -> Result<Response, ContractError> {

    // Check the caller is the Factory
    if info.sender != FACTORY.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    // Make sure this function may only be invoked once (check whether assets have already been saved)
    if VaultAssets::load_refs(&deps.as_ref()).is_ok() {
        return Err(ContractError::Unauthorized {});
    }

    // Check that the amplification is correct (set to < 1)
    if amp >= Uint64::new(WAD.as_u64()) {
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

    // Save the amplification value. It is stored as '1 - amplification' since most 
    // equations use the value in this way.
    let one_minus_amp = WAD.as_i256()
        .wrapping_sub(I256::from(amp));    // 'wrapping_sub' is safe, as amp < WAD (checked above)
    ONE_MINUS_AMP.save(deps.storage, &one_minus_amp)?;

    TARGET_ONE_MINUS_AMP.save(deps.storage, &one_minus_amp)?;
    AMP_UPDATE_TIMESTAMP_SECONDS.save(deps.storage, &Uint64::zero())?;
    AMP_UPDATE_FINISH_TIMESTAMP_SECONDS.save(deps.storage, &Uint64::zero())?;

    // Query and validate the vault asset balances
    let assets_balances = assets.iter()
        .map(|asset| {

            let balance = asset.query_prior_balance(&deps.as_ref(), &env, Some(&info))?;

            if balance.is_zero() {
                return Err(ContractError::InvalidZeroBalance {});
            }

            Ok(balance)
        })
        .collect::<Result<Vec<Uint128>, ContractError>>()?;

    // Save the assets
    // NOTE: there is no need to validate the assets, as invalid asset addresses
    // would have caused the previous 'asset balance' check to fail.
    let vault_assets = VaultAssets::new(assets);
    vault_assets.save(deps)?;

    let asset_refs = vault_assets.get_assets_refs();

    // Validate and save weights
    weights
        .iter()
        .zip(&asset_refs)   // zip: asset_refs.len() == weights.len() (checked above)
        .try_for_each(|(weight, asset_ref)| -> Result<(), ContractError> {

            if weight.is_zero() {
                return Err(ContractError::InvalidWeight {});
            }

            WEIGHTS.save(deps.storage, asset_ref, weight)?;

            Ok(())
        })?;

    // Initialize the escrows
    initialize_escrow_totals(deps, asset_refs)?;

    // Initialize the security limit
    // The maximum limit is derived from the sum of the of the weight-asset products.
    let max_limit_capacity = weights
        .iter()
        .zip(&assets_balances)      // zip: weights.len() == assets_balances.len()
        .try_fold(
            U256::zero(),
            |acc, (weight, balance)| {
                acc.checked_add(
                    U256::from(*weight).wrapping_mul(U256::from(*balance))     // Overflow safe, as U256::MAX > Uint128::MAX*Uint128::MAX
                )
            }
        )?;
    initialize_limit_capacity(deps, max_limit_capacity)?;

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

    update_amplification(deps, env.block.time)?;

    let one_minus_amp = ONE_MINUS_AMP.load(deps.storage)?;

    let assets = VaultAssets::load(&deps.as_ref())?;

    if deposit_amounts.len() != assets.get_assets().len() {
        return Err(
            ContractError::InvalidParameters{
                reason: "Invalid deposit_amounts count.".to_string()
            }
        );
    }

    // Compute how much 'units' the assets are worth.
    // Iterate over the assets, weights and deposit_amounts
    let mut units: U256 = U256::zero();                             // EVM mismatch: variable is *signed* on EVM (because of stack issues)
    let mut weighted_asset_balance_ampped_sum: I256 = I256::zero();
    let mut weighted_deposit_sum: U256 = U256::zero();              // NOTE: named 'assetDepositSum' on EVM

    assets.get_assets()
        .iter()
        .zip(&deposit_amounts)          // zip: deposit_amounts.len() == assets.len()
        .try_for_each(|(asset, deposit_amount)| -> Result<_, ContractError> {

            let weight = U256::from(
                WEIGHTS.load(deps.storage, asset.get_asset_ref())?
            );

            let vault_asset_balance = asset.query_prior_balance(
                &deps.as_ref(),
                &env,
                Some(&info)
            )?;

            let weighted_asset_balance = U256::from(vault_asset_balance)
                .wrapping_mul(weight);           // 'wrapping_mul' is safe as U256.max >= Uint128.max * Uint128.max

            // Compute (w·a)^(1-k) (WAD). Handle the case a=0 separatelly, as the implemented numerical calculation
            // will fail for that case.
            let weighted_asset_balance_ampped;    // EVM mismatch: variable is *signed* on EVM (because of stack issues)
                                                        // NOTE: named 'wab' on EVM
            if weighted_asset_balance.is_zero() {
                weighted_asset_balance_ampped = I256::zero();
            }
            else {
                weighted_asset_balance_ampped = pow_wad(
                    weighted_asset_balance.checked_mul(WAD)?
                        .as_i256(),     // If casting overflows to a negative number, 'pow_wad' will fail
                    one_minus_amp
                )?;

                weighted_asset_balance_ampped_sum = weighted_asset_balance_ampped_sum.checked_add(weighted_asset_balance_ampped)?;
            }

            // Stop if the user provides no tokens for the specific asset (save gas)
            if deposit_amount.is_zero() {
                return Ok(());
            }

            // Compute the units corresponding to the asset in question: F(a+input) - F(a)
            // where F(x) = x^(1-k)

            let weighted_deposit = U256::from(*deposit_amount)
                .wrapping_mul(weight);           // 'wrapping_mul' is safe as U256.max >= Uint128.max * Uint128.max

            let weighted_asset_balance_ampped_after_deposit = pow_wad(
                weighted_asset_balance
                    .checked_add(weighted_deposit)?
                    .checked_mul(WAD)?
                    .as_i256(),         // If casting overflows to a negative number, 'pow_wad' will fail
                one_minus_amp
            )?.as_u256();               // Casting always casts a positive number

            let units_for_asset = weighted_asset_balance_ampped_after_deposit
                .checked_sub(                               // Using 'checked_sub' for extra precaution ('wrapping_sub' should suffice).
                    weighted_asset_balance_ampped.as_u256() // Casting always casts a positive number
                )?;

            units = units.checked_add(units_for_asset)?;

            // Cache the weighted deposit sum to later update the security limit
            weighted_deposit_sum = weighted_deposit_sum.checked_add(weighted_deposit)?;

            Ok(())
        }
    )?;


    // Update the security limit variables
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
                .checked_add(weighted_deposit_sum)      // ! Addition must be 'checked', as 'used_limit_capacity' 
                                                        // ! may be larger than 'max_limit_capacity' (i.e. cannot 
                                                        // ! rely on the 'max' capacity calculation not overflowing)
                .map_err(|err| err.into())
        }
    )?;


    // Compute the reference liquidity.
    // 'wrapping_sub' in the following calculation is safe:
    //   - for positive 'unit_tracker': by desing, 'weighted_asset_balance_ampped_sum' > 'unit_tracker'
    //   - for negative 'unit_tracker': the subtraction could actually overflow, but the result will 
    //     be correct once casted to u256.
    // NOTE: The division by 'asset_count' is technically not required, as 'weighted_alpha_0_ampped'
    // is multiplied by this same value shortly after this line. This is intentional and is to
    // make sure the rounding introduced into 'weighted_alpha_0_ampped' is consistent everywhere it
    // is computed (that is, standardizing the way 'weighted_alpha_0_ampped' is calculated).
    let asset_count = U256::from(assets.get_assets().len() as u64);
    let weighted_alpha_0_ampped: U256 = weighted_asset_balance_ampped_sum
        .wrapping_sub(UNIT_TRACKER.load(deps.storage)?) 
        .as_u256()                      // Casting is safe (see reasoning above)
        .div(asset_count);


    // Subtract the vault fee from U to prevent deposit and withdrawals being employed as a method of swapping.
    // To recude costs, the governance fee is not taken. This is not an issue as swapping via this method is 
    // disincentivized by its higher gas costs.
    let vault_fee = VAULT_FEE.load(deps.storage)?;
    // EVM-MISMATCH: The following calculation is implemented on EVM manually using 'unchecked' operations to
    // further optimize the implementation. This optimization has not been implemented on this implementation.
    let units = fixed_point_math::mul_wad_down(
        units,
        fixed_point_math::WAD.wrapping_sub(vault_fee.into())    // 'wrapping_sub' is safe, as 'vault_fee' <= 'WAD'
    )?;

    // Do not include the 'escrowed' vault tokens in the total supply of vault tokens (return less)
    let effective_supply = U256::from(total_supply(deps.as_ref())?);

    // Compute the vault tokens to be minted.
    let out: Uint128 = calc_price_curve_limit_share(
        units,
        effective_supply,
        weighted_alpha_0_ampped.wrapping_mul(asset_count),  // 'wrapping_mul' is safe, see the 
                                                            // 'weighted_alpha_0_ampped' calculation
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

    // Handle asset transfer from the depositor to the vault
    let receive_asset_msgs = assets.receive_assets(
        &env,
        &info,
        deposit_amounts.clone()
    )?;

    Ok(Response::new()
        .set_data(to_binary(&out)?)     // Return the deposit output
        .add_messages(receive_asset_msgs)
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
    // the provided ones account for. Assets are then returned to the user according to
    // this share.

    update_amplification(deps, env.block.time)?;

    // Burn the vault tokens of the withdrawer
    // NOTE: since the vault tokens are burnt at this point, the 'vault_tokens' amount
    // has to be added to 'total_supply' whenever it's queried.
    let burn_response = execute_burn(deps.branch(), env.clone(), info.clone(), vault_tokens)?;


    // Compute weighted_alpha_0 to find the reference vault balances (i.e. the number of assets
    // the vault should have for 1:1 pricing).
    let assets = VaultAssets::load(&deps.as_ref())?;
    let one_minus_amp = ONE_MINUS_AMP.load(deps.storage)?;
    
    let weights = assets.get_assets()
        .iter()
        .map(|asset| WEIGHTS.load(deps.storage, asset.get_asset_ref()))
        .collect::<StdResult<Vec<Uint128>>>()?;

    if min_out.len() != assets.get_assets().len() {
        return Err(
            ContractError::InvalidParameters {
                reason: "Invalid min_out count.".to_string()
            }
        );
    }

    let mut weighted_asset_balance_ampped_sum: I256 = I256::zero();

    let effective_weighted_asset_balances = assets.get_assets()
        .iter()
        .zip(&weights)          // zip: weights.len() == assets.len()
        .map(|(asset, weight)| -> Result<U256, ContractError> {

            let vault_asset_balance = asset.query_prior_balance(
                &deps.as_ref(),
                &env,
                Some(&info)
            )?;

            if !vault_asset_balance.is_zero() {

                let weighted_asset_balance = U256::from(vault_asset_balance)
                    .wrapping_mul((*weight).into());           // 'wrapping_mul' is safe as U256.max >= Uint128.max * Uint128.max
    
                let weighted_asset_balance_ampped = pow_wad(
                    weighted_asset_balance.checked_mul(WAD)?
                        .as_i256(), // If casting overflows to a negative number 'pow_wad' will fail
                    one_minus_amp
                )?;

                weighted_asset_balance_ampped_sum = weighted_asset_balance_ampped_sum
                    .checked_add(weighted_asset_balance_ampped)?;

                let escrowed_asset_balance = TOTAL_ESCROWED_ASSETS.load(deps.storage, asset.get_asset_ref())?;

                Ok(
                    weighted_asset_balance.checked_sub(
                        U256::from(escrowed_asset_balance).wrapping_mul((*weight).into())
                    )?
                )
            }
            else {
                Ok(U256::zero())
            }

        })
        .collect::<Result<Vec<U256>, ContractError>>()?;

    // Compute the reference liquidity.
    // 'wrapping_sub' in the following calculation is safe:
    //   - for positive 'unit_tracker': by desing, 'weighted_asset_balance_ampped_sum' > 'unit_tracker'
    //   - for negative 'unit_tracker': the subtraction could actually overflow, but the result will 
    //     be correct once casted to u256.
    let weighted_alpha_0_ampped: U256 = weighted_asset_balance_ampped_sum
        .wrapping_sub(UNIT_TRACKER.load(deps.storage)?) 
        .as_u256()                                      // Casting is safe (see reasoning above)
        .div(U256::from(assets.get_assets().len() as u64));

    // Compute the effective supply. Include 'vault_tokens' to the queried supply as these have already been burnt
    // and also include the escrowed tokens to yield a smaller return.
    let effective_supply = U256::from(total_supply(deps.as_ref())?)
        .wrapping_add(vault_tokens.into())                                   // 'wrapping_add' is safe as U256.max >> Uint128.max
        .wrapping_add(TOTAL_ESCROWED_LIQUIDITY.load(deps.storage)?.into());  // 'wrapping_add' is safe as U256.max >> Uint128.max

    // Compute 'supply after withdrawal'/'supply before withdrawal' (in WAD terms)
    //    vault_tokens_share = (TS - vault_tokens) / TS
    let vault_tokens_share = effective_supply
        .wrapping_sub(vault_tokens.into())          // 'wrapping_sub' is safe as per the previous line
        .wrapping_mul(WAD)                          // 'wrapping_mul' is safe as U256.max > Uint128.max * ~2^60
        .div(effective_supply);

    // inner_diff = (w·a_0)^(1-k) · (1-vault_tokens_share^(1-k))
    let inner_diff = mul_wad_down(
        weighted_alpha_0_ampped,
        WAD.checked_sub(                            // Using 'checked_sub' for extra precaution ('wrapping_sub' should suffice)
            pow_wad(
                vault_tokens_share.as_i256(),       // Casting is safe as 'vault_tokens_share' <= 1 (WAD)
                one_minus_amp
            )?.as_u256()                            // Casting is safe as 'pow_wad' result is >= 0
        )?
    )?;


    // Compute the asset withdraw amounts
    let one_minus_amp_inverse = WADWAD / one_minus_amp;

    let mut weighted_withdraw_sum = U256::zero(); // NOTE: named 'totalWithdrawn' on EVM
    let withdraw_amounts: Vec<Uint128> = weights
        .iter()
        .zip(&min_out)                                  // zip: min_out.len() == weights.len()
        .zip(effective_weighted_asset_balances)         // zip: effective_weighted_asset_balances.len() == weights.len()
        .map(|((weight, asset_min_out), effective_weighted_asset_balance)| {

            // Compute the 'weighted_asset_balance_ampped' **with** the escrow balance taken into account
            let effective_weighted_asset_balance_ampped = pow_wad(
                effective_weighted_asset_balance
                    .checked_mul(WAD)?
                    .as_i256(),     // If casting overflows to a negative number 'pow_wad' will fail
                one_minus_amp
            )?.as_u256();           // Casting always casts a positive number

            // Compute the user's withdraw amount
            let weighted_withdraw_amount;
            if inner_diff < effective_weighted_asset_balance_ampped {
                // w·a · ( 1 - [ ((w·a)^(1-k) - inner_diff)/((w·a)^(1-k)) ]^(1/(1-k)) )
                weighted_withdraw_amount = mul_wad_down(
                    effective_weighted_asset_balance,
                    WAD.checked_sub(    // Using 'checked_sub' for extra precaution ('wrapping_sub' should suffice)
                        pow_wad(
                            div_wad_up(
                                // 'wrapping_sub' is safe because of the previous 'if' statement
                                effective_weighted_asset_balance_ampped.wrapping_sub(inner_diff),
                                effective_weighted_asset_balance_ampped
                            )?.as_i256(),   // Casting is safe, as the result is <= 1 (WAD)
                            one_minus_amp_inverse
                        )?.as_u256()        // Casting always casts a positive number
                    )?
                )?
            }
            else {
                // If the vault does not have enough assets, withdraw all of the vault's assets. This
                // happens if 'inner_diff' >= 'effective_weighted_asset_balance_ampped'. The user can
                // protect itself from such a scenario by specifying a minimum output.
                weighted_withdraw_amount = effective_weighted_asset_balance;
            }

            weighted_withdraw_sum = weighted_withdraw_sum.checked_add(weighted_withdraw_amount)?;

            let withdraw_amount: Uint128 = (weighted_withdraw_amount / U256::from(*weight)).try_into()?;

            // Check that the minimum output is honoured.
            if *asset_min_out > withdraw_amount {
                return Err(ContractError::ReturnInsufficient { out: withdraw_amount.clone(), min_out: *asset_min_out });
            };

            Ok(withdraw_amount)
        }).collect::<Result<Vec<Uint128>, ContractError>>()?;


    // Update the security limit variables
    MAX_LIMIT_CAPACITY.update(
        deps.storage,
        |max_limit_capacity| -> StdResult<_> {
            max_limit_capacity
                .checked_sub(weighted_withdraw_sum)
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


    // Handle asset transfer from the vault to the withdrawer
    let transfer_msgs: Vec<CosmosMsg> = assets.send_assets(
        &env,
        withdraw_amounts.clone(),
        info.sender.to_string()
    )?;


    Ok(Response::new()
        .set_data(to_binary(&withdraw_amounts)?)    // Return the withdrawn amounts
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

    update_amplification(deps, env.block.time)?;

    // Burn the vault tokens of the withdrawer
    let burn_response = execute_burn(deps.branch(), env.clone(), info.clone(), vault_tokens)?;


    // Compute weighted_alpha_0 to find the reference vault balances (i.e. the number of assets
    // the vault should have for 1:1 pricing). This value is then used to compute the corresponding
    // 'units' of the withdrawal.
    let assets = VaultAssets::load(&deps.as_ref())?;
    let one_minus_amp = ONE_MINUS_AMP.load(deps.storage)?;
    
    let weights = assets.get_assets()
        .iter()
        .map(|asset| WEIGHTS.load(deps.storage, asset.get_asset_ref()))
        .collect::<StdResult<Vec<Uint128>>>()?;

    let mut weighted_asset_balance_ampped_sum: I256 = I256::zero();

    let effective_asset_balances = assets.get_assets()
        .iter()
        .zip(&weights)      // zip: weights.len() == assets.len()
        .map(|(asset, weight)| -> Result<Uint128, ContractError> {
            
            let vault_asset_balance = asset.query_prior_balance(
                &deps.as_ref(),
                &env,
                Some(&info)
            )?;

            if !vault_asset_balance.is_zero() {

                let weighted_asset_balance = U256::from(vault_asset_balance)
                    .wrapping_mul((*weight).into());     // 'wrapping_mul' is safe as U256.max >= Uint128.max * Uint128.max
    
                let weighted_asset_balance_ampped = pow_wad(
                    weighted_asset_balance.checked_mul(WAD)?
                        .as_i256(), // If casting overflows to a negative number 'pow_wad' will fail
                    one_minus_amp
                )?;

                weighted_asset_balance_ampped_sum = weighted_asset_balance_ampped_sum
                    .checked_add(weighted_asset_balance_ampped)?;

                Ok(
                    vault_asset_balance.checked_sub(
                        TOTAL_ESCROWED_ASSETS.load(deps.storage, &asset.get_asset_ref())?
                    )?
                )
            }
            else {
                Ok(Uint128::zero())
            }
        })
        .collect::<Result<Vec<Uint128>, ContractError>>()?;

    // Compute the reference liquidity.
    // 'wrapping_sub' in the following calculation is safe:
    //   - for positive 'unit_tracker': by desing, 'weighted_asset_balance_ampped_sum' > 'unit_tracker'
    //   - for negative 'unit_tracker': the subtraction could actually overflow, but the result will 
    //     be correct once casted to u256.
    let weighted_alpha_0_ampped: U256 = weighted_asset_balance_ampped_sum
        .wrapping_sub(UNIT_TRACKER.load(deps.storage)?) 
        .as_u256()                                      // Casting is safe (see reasoning above)
        .div(U256::from(assets.get_assets().len() as u64));

    // Compute the effective supply. Include 'vault_tokens' to the queried supply as these have already been burnt
    // and also include the escrowed tokens to yield a smaller return.
    let effective_supply = U256::from(total_supply(deps.as_ref())?)
        .wrapping_add(vault_tokens.into())                                   // 'wrapping_add' is safe as U256.max >> Uint128.max
        .wrapping_add(TOTAL_ESCROWED_LIQUIDITY.load(deps.storage)?.into());  // 'wrapping_add' is safe as U256.max >> Uint128.max

    // Compute 'supply after withdrawal'/'supply before withdrawal' (in WAD terms)
    //    vault_tokens_share = (TS - vault_tokens) / TS
    let vault_tokens_share = effective_supply
        .wrapping_sub(vault_tokens.into())          // 'wrapping_sub' is safe as per the previous line
        .wrapping_mul(WAD)                          // 'wrapping_mul' is safe as U256.max > Uint128.max * ~2^60
        .div(effective_supply);

    // Compute the 'units' worth of the provided vault tokens.
    let mut units = U256::from(assets.get_assets().len() as u64).checked_mul(
        mul_wad_down(
            weighted_alpha_0_ampped,
            WAD.checked_sub(        // Using 'checked_sub' for extra precaution ('wrapping_sub' should suffice)
                pow_wad(
                    vault_tokens_share.as_i256(),   // If casting overflows to a negative number 'pow_wad' will fail
                    one_minus_amp
                )?.as_u256()        // Casting always casts a positive number
            )?
        )?
    )?;


    // Compute the asset withdraw amounts
    let assets_count = assets.get_assets().len();
    if withdraw_ratio.len() != assets_count || min_out.len() != assets_count {
        return Err(
            ContractError::InvalidParameters {
                reason: "Invalid withdraw_ratio/min_out count.".to_string()
            }
        );
    }

    let mut weighted_withdraw_sum = U256::zero(); // NOTE: named 'totalWithdrawn' on EVM
    let withdraw_amounts: Vec<Uint128> = weights
        .iter()
        .zip(&withdraw_ratio)               // zip: withdraw_ratio.len() == weights.len()
        .zip(&min_out)                      // zip: min_out.len() == weights.len()
        .zip(effective_asset_balances)      // zip: effective_asset_balances.len() == weights.len()
        .map(|(((weight, asset_withdraw_ratio), asset_min_out), effective_asset_balance)| {

            // Calculate the units allocated for the specific asset
            let units_for_asset = fixed_point_math::mul_wad_down(units, U256::from(*asset_withdraw_ratio))?;
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
            units = units.checked_sub(units_for_asset)?; // ! 'checked_sub' important: This will underflow for 
                                                         // ! malicious withdraw ratios (i.e. ratios > 1).

            // Calculate the asset amount corresponding to the asset units
            let withdraw_amount = calc_price_curve_limit(
                units_for_asset,
                effective_asset_balance.into(),
                (*weight).into(),
                one_minus_amp
            )?.try_into()?;

            // Check that the minimum output is honoured.
            if *asset_min_out > withdraw_amount {
                return Err(ContractError::ReturnInsufficient { out: withdraw_amount.clone(), min_out: *asset_min_out });
            };

            weighted_withdraw_sum = weighted_withdraw_sum.checked_add(
                U256::from(withdraw_amount).wrapping_mul((*weight).into())  // 'wrapping_mul' is safe as U256.max >= Uint128.max * Uint128.max
            )?;

            Ok(withdraw_amount)
        }).collect::<Result<Vec<Uint128>, ContractError>>()?;

    // Make sure all units have been consumed
    if !units.is_zero() { return Err(ContractError::UnusedUnitsAfterWithdrawal { units }) };


    // Update the security limit variables
    MAX_LIMIT_CAPACITY.update(
        deps.storage,
        |max_limit_capacity| -> StdResult<_> {
            max_limit_capacity
                .checked_sub(weighted_withdraw_sum)
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

    // Handle asset transfer from the vault to the withdrawer
    let transfer_msgs: Vec<CosmosMsg> = assets.send_assets(
        &env,
        withdraw_amounts.clone(),
        info.sender.to_string()
    )?;   


    Ok(Response::new()
        .set_data(to_binary(&withdraw_amounts)?)    // Return the withdrawn amounts
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
/// * `from_asset_ref` - The source asset reference.
/// * `to_asset_ref` - The destination asset reference.
/// * `amount` - The `from_asset_ref` amount sold to the vault.
/// * `min_out` - The mininmum return to get of `to_asset_ref`.
/// 
pub fn local_swap(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    from_asset_ref: String,
    to_asset_ref: String,
    amount: Uint128,
    min_out: Uint128
) -> Result<Response, ContractError> {

    update_amplification(deps, env.block.time)?;

    let vault_fee: Uint128 = mul_wad_down(
        U256::from(amount),
        U256::from(VAULT_FEE.load(deps.storage)?)
    )?.as_uint128();    // Casting safe, as fee < amount, and amount is Uint128

    // Calculate the return value
    let from_asset = Asset::from_asset_ref(&deps.as_ref(), &from_asset_ref)?;
    let to_asset = Asset::from_asset_ref(&deps.as_ref(), &to_asset_ref)?;
    let out: Uint128 = calc_local_swap(
        &deps.as_ref(),
        env.clone(),
        Some(&info),
        &from_asset,
        &to_asset,
        amount.checked_sub(vault_fee)?      // Using 'checked_sub' for extra precaution ('wrapping_sub' should suffice)
    )?;

    if min_out > out {
        return Err(ContractError::ReturnInsufficient { out, min_out });
    }

    // Update the max limit capacity, as for amplified vaults it is based on the vault's asset balances
    let from_weight = WEIGHTS.load(deps.storage, from_asset.get_asset_ref())?;
    let to_weight = WEIGHTS.load(deps.storage, to_asset.get_asset_ref())?;
    let limit_capacity_increase = U256::from(amount).wrapping_mul(U256::from(from_weight));  // wrapping_mul is overflow safe as U256.max >= Uint128.max * Uint128.max
    let limit_capacity_decrease = U256::from(out).wrapping_mul(U256::from(to_weight));       // wrapping_mul is overflow safe as U256.max >= Uint128.max * Uint128.max
    MAX_LIMIT_CAPACITY.update(
        deps.storage,
        |max_limit_capacity| -> StdResult<_> {
            if limit_capacity_decrease > limit_capacity_increase {
                max_limit_capacity.checked_sub(
                    limit_capacity_decrease.wrapping_sub(limit_capacity_increase)   // 'wrapping_sub' is safe because of the previous 'if' statement
                )
                .map_err(|err| err.into())
            }
            else {
                max_limit_capacity.checked_add(
                    limit_capacity_increase.wrapping_sub(limit_capacity_decrease)   // 'wrapping_sub' is safe because of the previous 'if' statement
                )
                .map_err(|err| err.into())
            }
        }
    )?;

    // Handle asset transfer from the swapper to the vault
    let receive_asset_msg = from_asset.receive_asset(&env, &info, amount)?;

    // Handle asset transfer from the vault to the swapper
    let send_asset_msg = to_asset.send_asset(&env, out, info.sender.to_string())?;

    // Build the message to collect the governance fee.
    let collect_governance_fee_message = collect_governance_fee_message(
        &deps.as_ref(),
        &env,
        &from_asset,
        vault_fee
    )?;

    // Build response
    let mut response = Response::new()
        .set_data(to_binary(&out)?);     // Return the swap output

    if let Some(msg) = receive_asset_msg {
        response = response.add_message(msg);
    }

    if let Some(msg) = send_asset_msg {
        response = response.add_message(msg);
    }

    if let Some(msg) = collect_governance_fee_message {
        response = response.add_message(msg);
    }

    Ok(response
        .add_event(
            local_swap_event(
                info.sender.to_string(),
                from_asset.get_asset_ref(),
                to_asset.get_asset_ref(),
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
/// * `from_asset_ref` - The source asset reference.
/// * `to_asset_index` - The destination asset index.
/// * `amount` - The `from_asset_ref` amount sold to the vault.
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
    from_asset_ref: String,
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

    update_amplification(deps, env.block.time)?;

    let vault_fee: Uint128 = mul_wad_down(
        U256::from(amount),
        U256::from(VAULT_FEE.load(deps.storage)?)
    )?.as_uint128();    // Casting safe, as fee < amount, and amount is Uint128

    let effective_swap_amount = amount.checked_sub(vault_fee)?;     // Using 'checked_sub' for extra precaution ('wrapping_sub' should suffice)

    // Calculate the units bought.
    let from_asset = Asset::from_asset_ref(&deps.as_ref(), &from_asset_ref)?;
    let u = calc_send_asset(
        &deps.as_ref(),
        env.clone(),
        Some(&info),
        &from_asset,
        effective_swap_amount
    )?;

    // Update the 'unit_tracker' for the 'balance0' calculations.
    UNIT_TRACKER.update(
        deps.storage,
        |unit_tracker| -> StdResult<_> {
            unit_tracker
                .checked_add(u.try_into()?)  // ! IMPORTANT: Safely casting into I256 is also very 
                                             // ! important for 'on_send_asset_success', as it requires 
                                             // ! this same casting and must never revert.
                .map_err(|err| err.into())
        }
    )?;

    // Create a 'send asset' escrow
    let block_number = env.block.height as u32;
    let send_asset_hash = compute_send_asset_hash(
        to_account.as_slice(),
        u,
        effective_swap_amount,
        &from_asset.get_asset_ref(),
        block_number
    );

    create_asset_escrow(
        deps,
        send_asset_hash.clone(),
        effective_swap_amount,  // NOTE: The fee is also deducted from the escrow  
                                // amount to prevent denial of service attacks.
        &from_asset.get_asset_ref(),
        fallback_account
    )?;

    // NOTE: The security limit adjustment is delayed until the swap confirmation is received to
    // prevent a router from abusing swap 'timeouts' to circumvent the security limit.

    // Handle asset transfer from the swapper to the vault
    let receive_asset_msg = from_asset.receive_asset(&env, &info, amount)?;

    // Build the message to collect the governance fee.
    let collect_governance_fee_message = collect_governance_fee_message(
        &deps.as_ref(),
        &env,
        &from_asset,
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
        from_asset: from_asset.get_asset_ref().to_string(),
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
        .set_data(to_binary(&u)?);       // Return the purchased 'units'

    if let Some(msg) = receive_asset_msg {
        response = response.add_message(msg);
    }

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
                from_asset.get_asset_ref(),
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
    calldata_target: Option<String>,
    calldata: Option<Binary>
) -> Result<Response, ContractError> {

    // Only allow the 'chain_interface' to invoke this function.
    if Some(info.sender.clone()) != CHAIN_INTERFACE.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    // Only allow connected vaults.
    if !is_connected(&deps.as_ref(), &channel_id, from_vault.clone()) {
        return Err(ContractError::VaultNotConnected { channel_id, vault: from_vault })
    }

    update_amplification(deps, env.block.time)?;

    // Calculate the swap return.
    // NOTE: no fee is taken here, the fee is always taken on the sending side.
    let to_asset_ref = VaultAssets::load_refs(&deps.as_ref())?
        .get(to_asset_index as usize)
        .ok_or(ContractError::AssetNotFound {})?
        .clone();
    let to_asset = Asset::from_asset_ref(&deps.as_ref(), &to_asset_ref)?;
    let out = calc_receive_asset(&deps.as_ref(), env.clone(), Some(&info), &to_asset, u)?;

    if min_out > out {
        return Err(ContractError::ReturnInsufficient { out, min_out });
    }

    // Update the max limit capacity and the used limit capacity.
    let to_weight = WEIGHTS.load(deps.storage, to_asset.get_asset_ref())?;
    let limit_capacity_delta = U256::from(out)
        .wrapping_mul(U256::from(to_weight));   // 'wrapping_mul' is safe as U256.max >= Uint128.max * Uint128.max


    // The 'max_limit_capacity' must be updated, as for amplified vaults it depends on
    // the vault's asset balances.
    MAX_LIMIT_CAPACITY.update(
        deps.storage,
        |max_limit_capacity| -> StdResult<_> {
            max_limit_capacity
                .checked_sub(limit_capacity_delta)
                .map_err(|err| err.into())
        }
    )?;
    update_limit_capacity(deps, env.block.time, limit_capacity_delta)?;

    // Update the 'unit_tracker' for the 'balance0' calculations.
    UNIT_TRACKER.update(
        deps.storage,
        |unit_tracker| -> StdResult<_> {
            unit_tracker
                .checked_sub(u.try_into()?)
                .map_err(|err| err.into())
        }
    )?;

    // Handle asset transfer from the vault to the swapper
    let send_asset_msg = to_asset.send_asset(&env, out, to_account.clone())?;

    // Build the calldata message.
    let calldata_message = match calldata_target {
        Some(target) => Some(create_on_catalyst_call_msg(
            target,
            out,
            calldata.unwrap_or_default()
        )?),
        None => None
    };

    // Build and send the response.
    let mut response = Response::new()
        .set_data(to_binary(&out)?);     // Return the purchased tokens

    if let Some(msg) = send_asset_msg {
        response = response.add_message(msg);
    }

    if let Some(msg) = calldata_message {
        response = response.add_message(msg);
    }

    Ok(response
        .add_event(
            receive_asset_event(
                channel_id,
                from_vault,
                to_account,
                to_asset.get_asset_ref(),
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

    update_amplification(deps, env.block.time)?;

    // Burn the vault tokens of the sender
    let burn_response = execute_burn(deps.branch(), env.clone(), info.clone(), amount)?;

    // Compute (w·alpha_0)^(1-k) to find the vault's reference balances (point at which
    // the assets are priced 1:1).
    let one_minus_amp = ONE_MINUS_AMP.load(deps.storage)?;
    let (balance_0_ampped, asset_count) = calc_balance_0_ampped(
        deps.as_ref(),
        env.clone(),
        Some(&info),
        one_minus_amp
    )?;

    // Compute the effective supply. Include 'vault_tokens' to the queried supply as these have 
    // already been burnt and also include the escrowed tokens to yield a smaller return.
    let effective_supply = U256::from(total_supply(deps.as_ref())?)
        .wrapping_add(amount.into())                                        // 'wrapping_add' is safe as U256.max >> Uint128.max
        .wrapping_add(TOTAL_ESCROWED_LIQUIDITY.load(deps.storage)?.into()); // 'wrapping_add' is safe as U256.max >> Uint128.max

    // Compute 'supply after withdrawal'/'supply before withdrawal' (in WAD terms)
    //    vault_tokens_share = (TS + vault_tokens) / TS
    let vault_tokens_share = div_wad_down(
        effective_supply.checked_add(amount.into())?,
        effective_supply
    )?;

    // Compute the unit value of the provided vaultTokens as
    //    asset_count · balance_0_ampped · (vault_tokens_share^(1-k) - 1)
    // This step simplifies withdrawing and swapping into a single step
    let units = U256::from(asset_count as u64).checked_mul(
        mul_wad_down(
            balance_0_ampped,
            pow_wad(
                vault_tokens_share.as_i256(),   // If casting overflows to a negative number 'pow_wad' will fail
                one_minus_amp
            )?.as_u256()                        // Casting is safe, as pow_wad result is always positive
                .checked_sub(WAD)?              // Using 'checked_sub' for extra precaution ('wrapping_sub' should suffice)
        )?
    )?;

    // Update the 'unit_tracker' for the 'balance0' calculations.
    UNIT_TRACKER.update(
        deps.storage,
        |unit_tracker| -> StdResult<_> {
            unit_tracker
            .checked_add(units.try_into()?) // ! IMPORTANT: Safely casting into I256 is also very 
                                            // ! important for 'on_send_liquidity_success', as it requires 
                                            // ! this same casting and must never revert.
            .map_err(|err| err.into())
        }
    )?;

    // Create a 'send liquidity' escrow
    let block_number = env.block.height as u32;
    let send_liquidity_hash = compute_send_liquidity_hash(
        to_account.as_slice(),
        units,
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
    let send_cross_chain_liquidity_msg = InterfaceExecuteMsg::SendCrossChainLiquidity {
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
            msg: to_binary(&send_cross_chain_liquidity_msg)?,
            funds: vec![]
        }
    );

    Ok(Response::new()
        .set_data(to_binary(&units)?)   // Return the 'units' sent
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
/// * `from_amount` - The liquidity amount sold to the source vault.
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
    units: U256,
    min_vault_tokens: Uint128,
    min_reference_asset: Uint128,
    from_amount: U256,
    from_block_number_mod: u32,
    calldata_target: Option<String>,
    calldata: Option<Binary>
) -> Result<Response, ContractError> {

    // Only allow the 'chain_interface' to invoke this function.
    if Some(info.sender.clone()) != CHAIN_INTERFACE.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    // Only allow connected vaults.
    if !is_connected(&deps.as_ref(), &channel_id, from_vault.clone()) {
        return Err(ContractError::VaultNotConnected { channel_id, vault: from_vault })
    }

    update_amplification(deps, env.block.time)?;

    // Compute (w·alpha_0)^(1-k) to find the vault's reference balances (point at which
    // the assets are priced 1:1).
    let one_minus_amp = ONE_MINUS_AMP.load(deps.storage)?;
    let (balance_0_ampped, asset_count) = calc_balance_0_ampped(
        deps.as_ref(),
        env.clone(),
        Some(&info),
        one_minus_amp
    )?;

    let one_minus_amp_inverse = WADWAD / one_minus_amp;

    let n_weighted_balance_ampped = U256::from(asset_count as u64).checked_mul(
        balance_0_ampped
    )?;

    // Do not include the 'escrowed' vault tokens in the total supply of vault tokens 
    // of the vault to return less.
    let total_supply = U256::from(total_supply(deps.as_ref())?);

    // Calculate the 'vault_tokens' corresponding to the received 'units'.
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
        
        // Compute the fraction of the 'balance0' that the swapper owns.

        let balance_0 = pow_wad(
            balance_0_ampped.as_i256(),     // If casting overflows to a negative number 'pow_wad' will fail
            one_minus_amp_inverse
        )?.as_u256();                       // Casting is safe, as pow_wad result is always positive

        // Include the escrowed vault tokens in the total supply to ensure that even if all the ongoing 
        // transactions revert, the specified 'min_reference_asset' is fulfilled. Include the vault tokens 
        // as they are going to be minted.
        let escrowed_balance = TOTAL_ESCROWED_LIQUIDITY.load(deps.storage)?;

        let effective_supply = total_supply
            .wrapping_add(escrowed_balance.into())   // 'wrapping_add' is safe, as U256.max >> Uint128.max
            .wrapping_add(vault_tokens.into());      // 'wrapping_add' is safe, as U256.max >> Uint128.max

        let balance_0_share: Uint128 = balance_0
            .checked_mul(vault_tokens.into())?
            .div(effective_supply)
            .div(WAD)
            .try_into()?;

        if min_reference_asset > balance_0_share {
            return Err(ContractError::ReturnInsufficient { out: balance_0_share, min_out: min_reference_asset });
        }

    }

    // Update the 'unit_tracker' for the 'balance0' calculations.
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
        return Err(
            ContractError::SecurityLimitExceeded {
                overflow: units.wrapping_sub(n_weighted_balance_ampped)  // 'wrapping_sub' is safe, as 'units' >= n_weighted_balance_ampped
            }
        )
    }

    // Otherwise calculate the 'vault_token_equivalent' of the provided units to check if the
    // limit is being honoured.
    let vault_token_equivalent = mul_wad_up(
        pow_wad(
            n_weighted_balance_ampped.as_i256(),    // If casting overflows to a negative number 'pow_wad' will fail
            one_minus_amp_inverse
        )?.as_u256(),                               // Casting is safe, as 'pow_wad' result is always positive
        WAD.checked_sub(                            // Using 'checked_sub' for extra precaution ('wrapping_sub' should suffice)
            pow_wad(
                div_wad_down(
                    n_weighted_balance_ampped.wrapping_sub(units),  // 'wrapping_sub' is safe as 'n_weighted_balance_ampped' 
                                                                    // is always <= 'units' (check is shortly above)
                    n_weighted_balance_ampped
                )?.as_i256(),                       // If casting overflows to a negative number 'pow_wad' will fail
                one_minus_amp_inverse
            )?.as_u256()                            // Casting is safe, as 'pow_wad' result is always positive
        )?
    )?;

    update_limit_capacity(
        deps,
        env.block.time,
        mul_wad_down(U256::from(2u64), vault_token_equivalent)?
    )?;

    // Mint the vault tokens.
    let mint_response = execute_mint(
        deps.branch(),
        env.clone(),
        MessageInfo {
            sender: env.contract.address.clone(),   // This contract itself is the one 'sending' the mint operation
            funds: vec![],
        },
        to_account.clone(),  // NOTE: the address is validated by the 'execute_mint' call
        vault_tokens
    )?;

    // Build the calldata message.
    let calldata_message = match calldata_target {
        Some(target) => Some(create_on_catalyst_call_msg(
            target,
            vault_tokens,
            calldata.unwrap_or_default()
        )?),
        None => None
    };

    // Build and send the response.
    let mut response = Response::new()
        .set_data(to_binary(&vault_tokens)?);   // Return the vault tokens 'received'

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
        .add_event(
            cw20_response_to_standard_event(
                mint_response
            )
        )
    )
}



/// Compute the return of 'send_asset' (not including fees).
/// 
/// **NOTE**: This function reverts if 'from_asset' does not form part of the vault or
/// if `amount` and the vault's asset balance are both 0.
/// 
/// # Arguments:
/// * `from_asset` - The source asset.
/// * `amount` - The `from_asset` amount sold to the vault (excluding the vault fee).
/// 
pub fn calc_send_asset(
    deps: &Deps,
    env: Env,
    info: Option<&MessageInfo>,
    from_asset: &Asset,
    amount: Uint128
) -> Result<U256, ContractError> {

    let from_asset_weight = WEIGHTS.load(deps.storage, from_asset.get_asset_ref())
        .map_err(|_| ContractError::AssetNotFound {})?;

    let from_asset_balance: Uint128 = from_asset.query_prior_balance(deps, &env, info)?;

    let one_minus_amp = ONE_MINUS_AMP.load(deps.storage)?;

    let units = calc_price_curve_area(
        amount.into(),
        from_asset_balance.into(),
        U256::from(from_asset_weight),
        one_minus_amp
    )?;

    // If the swap amount is small with respect to the vault's asset balance, add an 
    // additional fee to cover up for mathematical errors of the implementation.
    if from_asset_balance / SMALL_SWAP_RATIO >= amount {
        return Ok(
            units
                .wrapping_mul(SMALL_SWAP_RETURN)    // 'wrapping_mul' is safe, as the 'units' value depends heavily on the swapped
                                                    // amount wrt to the vault's balance, and thus it will have a small value in
                                                    // this case. In any case, if it were to overflow less would be returned to the 
                                                    // user, which would not be critical from the vault's safety point of view.
                .div(WAD)
        )
    }

    Ok(units)
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
    info: Option<&MessageInfo>,
    to_asset: &Asset,
    u: U256
) -> Result<Uint128, ContractError> {

    let to_asset_weight = WEIGHTS.load(deps.storage, to_asset.get_asset_ref())
        .map_err(|_| ContractError::AssetNotFound {})?;

    // Subtract the escrowed balance from the vault's total balance to return a smaller output.
    let to_asset_escrowed_balance: Uint128 = TOTAL_ESCROWED_ASSETS.load(
        deps.storage,
        to_asset.get_asset_ref()
    )?;
    let to_asset_balance: Uint128 = to_asset
        .query_prior_balance(deps, &env, info)?
        .checked_sub(to_asset_escrowed_balance)?;
    
    let one_minus_amp = ONE_MINUS_AMP.load(deps.storage)?;

    calc_price_curve_limit(
        u,
        to_asset_balance.into(),
        U256::from(to_asset_weight),
        one_minus_amp
    ).and_then(
        |val| TryInto::<Uint128>::try_into(val).map_err(|err| err.into())
    )

}


/// Compute the return of 'local_swap' (not including fees).
/// 
/// **NOTE**: This function reverts if 'from_asset' or 'to_asset' do not form part 
/// of the vault or if `amount` and the vault's asset balance are both 0.
/// 
/// # Arguments:
/// * `from_asset` - The source asset.
/// * `to_asset` - The destination asset.
/// * `amount` - The `from_asset` amount sold to the vault (excluding fees).
/// 
pub fn calc_local_swap(
    deps: &Deps,
    env: Env,
    info: Option<&MessageInfo>,
    from_asset: &Asset,
    to_asset: &Asset,
    amount: Uint128
) -> Result<Uint128, ContractError> {

    let from_asset_weight = WEIGHTS.load(deps.storage, from_asset.get_asset_ref())
        .map_err(|_| ContractError::AssetNotFound {})?;

    let to_asset_weight = WEIGHTS.load(deps.storage, to_asset.get_asset_ref())
        .map_err(|_| ContractError::AssetNotFound {})?;

    let from_asset_balance: Uint128 = from_asset.query_prior_balance(deps, &env, info)?;

    // Subtract the 'to_asset' escrowed balance from the vault's total balance 
    // to return a smaller output.
    let to_asset_escrowed_balance: Uint128 = TOTAL_ESCROWED_ASSETS.load(
        deps.storage,
        to_asset.get_asset_ref()
    )?;
    let to_asset_balance: Uint128 = to_asset
        .query_prior_balance(deps, &env, info)?
        .checked_sub(to_asset_escrowed_balance)?;

    let one_minus_amp = ONE_MINUS_AMP.load(deps.storage)?;

    let output: Uint128 = calc_combined_price_curves(
        amount.into(),
        from_asset_balance.into(),
        to_asset_balance.into(),
        from_asset_weight.into(),
        to_asset_weight.into(),
        one_minus_amp
    )?.try_into()?;

    // If the swap amount is small with respect to the vault's asset balance, add an 
    // additional fee to cover up for mathematical errors of the implementation.
    if from_asset_balance / SMALL_SWAP_RATIO >= amount {
        return Ok(
            (
                U256::from(output)
                    .wrapping_mul(SMALL_SWAP_RETURN)  // 'wrapping_mul' is safe as U256::MAX > Uint128::MAX*~2^64
                    .div(WAD)
                ).as_uint128()     // Casting is safe, as the result is always <= 'output', and 'output' is <= Uint128::MAX
        )
    }

    Ok(output)
}



/// Compute the vault's balance0. This value allows for the derivation of the vault's assets balances
/// that would be required such that their pricing were 1:1.
/// 
/// **NOTE**: This function takes as argument `one_minus_amp`, as this value is most times readily 
/// available whenever this function is called, and thus it is avoided reading twice from the store
/// in these cases.
/// 
/// **NOTE**: This function also returns the vault's asset count, as it is always used in conjunction
/// with the balance0 value.
/// 
/// # Arguments:
/// * `one_minus_amp` - One minus the vault's amplification.
/// 
pub fn calc_balance_0(
    deps: Deps,
    env: Env,
    info: Option<&MessageInfo>,
    one_minus_amp: I256
) -> Result<(U256, usize), ContractError> {

    let (balance_0_ampped, asset_count) = calc_balance_0_ampped(
        deps,
        env,
        info,
        one_minus_amp
    )?;

    // Remove the power from 'balance_0_ampped'
    let balance_0 = pow_wad(
        balance_0_ampped.as_i256(),   // If casting overflows to a negative number 'pow_wad' will fail
        WADWAD / one_minus_amp
    )?.as_u256();

    Ok((balance_0, asset_count))
}


/// Compute the vault's balance0 to the power of `one_minus_amp`. This value allows for the derivation 
/// of the vault's assets balances that would be required such that their pricing were 1:1.
/// 
/// **NOTE**: This function takes as argument `one_minus_amp`, as this value is most times readily 
/// available whenever this function is called, and thus it is avoided reading twice from the store
/// in these cases.
/// 
/// **NOTE**: This function also returns the vault's asset count, as it is always used in conjunction
/// with the balance0_ampped value.
/// 
/// **NOTE-DEV**: This function is called `_computeBalance0` on the EVM implementation.
/// 
/// # Arguments:
/// * `one_minus_amp` - One minus the vault's amplification.
/// 
pub fn calc_balance_0_ampped(
    deps: Deps,
    env: Env,
    info: Option<&MessageInfo>,
    one_minus_amp: I256
) -> Result<(U256, usize), ContractError> {
    
    let assets = VaultAssets::load(&deps)?;
    let unit_tracker = UNIT_TRACKER.load(deps.storage)?;

    let weights = assets.get_assets()
        .iter()
        .map(|asset| {
            WEIGHTS.load(deps.storage, asset.get_asset_ref())
        })
        .collect::<StdResult<Vec<Uint128>>>()?;

    let asset_balances = assets.get_assets()
        .iter()
        .map(|asset| -> Result<Uint128, ContractError> {
            Ok(
                asset.query_prior_balance(&deps, &env, info)?
            )
        })
        .collect::<Result<Vec<Uint128>, ContractError>>()?;

    let assets_count = assets.get_assets().len();

    let weighted_alpha_0_ampped = calc_weighted_alpha_0_ampped(
        weights,
        asset_balances,
        one_minus_amp,
        unit_tracker
    )?;

    Ok((weighted_alpha_0_ampped, assets_count))
}



/// Amplified-specific handling of the confirmation of a successful asset swap.
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
/// * `asset_ref` - The swap source asset reference.
/// * `block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32).
/// 
pub fn on_send_asset_success_amplified(
    deps: &mut DepsMut,
    env: &Env,
    info: &MessageInfo,
    channel_id: String,
    to_account: Binary,
    u: U256,
    escrow_amount: Uint128,
    asset_ref: String,
    block_number_mod: u32
) -> Result<Response, ContractError> {

    // Execute the common 'success' logic
    let response = on_send_asset_success(
        deps,
        env,
        info,
        channel_id,
        to_account,
        u,
        escrow_amount,
        asset_ref.clone(),
        block_number_mod
    )?;

    // The outgoing 'flow' is subtracted from the used limit capacity to avoid having a fixed
    // one-sided maximum daily cross chain volume. If the router was fraudulent, no one would
    // execute an outgoing swap.

    let weight = WEIGHTS.load(deps.storage, asset_ref.as_ref())?;
    let limit_delta = U256::from(escrow_amount).wrapping_mul(weight.into());     // 'wrapping_mul' is safe, as U256.max >= Uint128.max * Uint128.max

    // Minor optimization: avoid storage write if the used capacity is already at zero
    let used_capacity = USED_LIMIT_CAPACITY.load(deps.storage)?;
    if !used_capacity.is_zero() {
        USED_LIMIT_CAPACITY.save(deps.storage, &used_capacity.saturating_sub(limit_delta))?;
    }

    // The 'max_limit_capacity' must also be updated, as for the amplified vault it depends 
    // on the vault's asset balances.

    MAX_LIMIT_CAPACITY.update(
        deps.storage,
        |max_limit_capacity| -> StdResult<_> {
            // The max capacity update calculation might overflow, yet it should never make 
            // the callback revert. Hence the capacity is set to the maximum allowed value 
            // without allowing it to overflow (saturating_add).
            Ok(
                max_limit_capacity.saturating_add(limit_delta)
            )
        }
    )?;

    Ok(response)
}


/// Amplified-specific handling of the confirmation of an unsuccessful asset swap.
/// 
/// This function adds unit tracker adjustment to the default implementation.
/// 
/// **DEV NOTE**: This function should never revert (for valid swap data).
/// 
/// # Arguments:
/// * `channel_id` - The swap's channel id.
/// * `to_account` - The recipient of the swap output.
/// * `u` - The units value of the swap.
/// * `escrow_amount` - The escrowed asset amount.
/// * `asset_ref` - The swap source asset reference.
/// * `block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32).
/// 
pub fn on_send_asset_failure_amplified(
    deps: &mut DepsMut,
    env: &Env,
    info: &MessageInfo,
    channel_id: String,
    to_account: Binary,
    u: U256,
    escrow_amount: Uint128,
    asset_ref: String,
    block_number_mod: u32
) -> Result<Response, ContractError> {

    // Execute the common 'failure' logic
    let response = on_send_asset_failure(
        deps,
        env,
        info,
        channel_id,
        to_account,
        u,
        escrow_amount,
        asset_ref,
        block_number_mod
    )?;

    // Remove the timed-out units from the unit tracker.
    UNIT_TRACKER.update(deps.storage, |unit_tracker| -> StdResult<_> {
        Ok(
            unit_tracker.checked_sub(   // Using 'checked_sub' as it would be extremely bad for the 'unit_tracker' to underflow.
                                        // It is extremely difficult for this to happen.
                u.as_i256()             // 'u' casting to i256 is safe, as this requirement has been checked on 'send_asset'
            )?
        )
    })?;

    Ok(response)
}


// 'on_send_liquidity_success' is not overwritten as it is very expensive to compute the update
// to the security limit. Realistically, this is only detrimental for the cases in which a large
// share of the vault assets are liquidity swapped. This is likely to only happen when the vault
// is low on liquidity, case in which liquidity swaps shouldn't be used. 


/// Amplified-specific handling of the confirmation of an unsuccessful liquidity swap.
/// 
/// This function adds unit tracker adjustment to the default implementation.
/// 
/// **DEV NOTE**: This function should never revert (for valid swap data).
/// 
/// # Arguments:
/// * `channel_id` - The swap's channel id.
/// * `to_account` - The recipient of the swap output.
/// * `u` - The units value of the swap.
/// * `escrow_amount` - The escrowed asset amount.
/// * `block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32).
/// 
pub fn on_send_liquidity_failure_amplified(
    deps: &mut DepsMut,
    env: &Env,
    info: &MessageInfo,
    channel_id: String,
    to_account: Binary,
    u: U256,
    escrow_amount: Uint128,
    block_number_mod: u32
) -> Result<Response, ContractError> {

    // Execute the common 'failure' logic
    let response = on_send_liquidity_failure(
        deps,
        env,
        info,
        channel_id,
        to_account,
        u,
        escrow_amount,
        block_number_mod
    )?;

    // Remove the timed-out units from the unit tracker.
    UNIT_TRACKER.update(deps.storage, |unit_tracker| -> StdResult<_> {
        Ok(
            unit_tracker.checked_sub(   // Using 'checked_sub' as it would be extremely bad for the 'unit_tracker' to underflow.
                                        // It is extremely difficult for this to happen.
                u.as_i256()             // 'u' casting to i256 is safe, as this requirement has been checked on 'send_liquidity'
            )?
        )
    })?;

    Ok(response)
}


/// Allow governance to modify the vault amplification.
/// 
/// **NOTE**: the amplification value has to be less than 1 (in WAD notation) and it cannot 
/// introduce a change larger than `MAX_AMP_ADJUSTMENT_FACTOR`.
/// 
/// **NOTE**: `target_timestamp` must be within `MIN_ADJUSTMENT_TIME_SECONDS` and
/// `MAX_ADJUSTMENT_TIME_SECONDS` from the current time.
/// 
/// # Arguments:
/// * `target_timestamp` - The time at which the amplification update must be completed.
/// * `target_amplification` - The new desired amplification.
/// 
pub fn set_amplification(
    deps: &mut DepsMut,
    env: &Env,
    info: MessageInfo,
    target_timestamp: Uint64,
    target_amplification: Uint64
) -> Result<Response, ContractError> {

    // Amplification changes are disabled for cross-chain vaults, as the 'unit_tracker' implementation
    // is incompatible with amplification changes.
    if CHAIN_INTERFACE.load(deps.storage)?.is_some() {
        return Err(
            ContractError::Error("Amplification adjustment is disabled for cross-chain vaults.".to_string())
        );
    }

    // Only allow amplification changes by the factory owner
    if info.sender != factory_owner(&deps.as_ref())? {
        return Err(ContractError::Unauthorized {});
    }
    
    // Check 'target_timestamp' is within the defined acceptable bounds
    let current_time = Uint64::new(env.block.time.seconds());
    if
        target_timestamp < current_time + MIN_ADJUSTMENT_TIME_SECONDS ||
        target_timestamp > current_time + MAX_ADJUSTMENT_TIME_SECONDS
    {
        return Err(ContractError::InvalidTargetTime {});
    }

    // Check that the specified 'target_amplification' is correct (set to < 1)
    if target_amplification >= WAD.as_u64().into() {        // Casting is safe as 'WAD' < u64.max
        return Err(ContractError::InvalidAmplification {})
    }
    
    // Limit the maximum allowed relative amplification change to a factor of 'MAX_AMP_ADJUSTMENT_FACTOR'.
    // Note that this effectively 'locks' the amplification if it gets intialized to 0. Similarly, the 
    // amplification will never be allowed to be set to 0 if it is initialized to any other value 
    // (note how 'target_amplification*MAX_AMP_ADJUSTMENT_FACTOR < current_amplification' is used
    // instead of 'target_amplification < current_amplification/MAX_AMP_ADJUSTMENT_FACTOR').
    let current_amplification: Uint64 = WAD
        .as_i256()                                          // Casting is safe as 'WAD' < I256.max
        .wrapping_sub(ONE_MINUS_AMP.load(deps.storage)?)    // 'wrapping_sub' is safe as 'ONE_MINUS_AMP' <= 'WAD'
        .as_u64().into();                                   // Casting is safe as 'AMP' <= u64.max

    if
        target_amplification > current_amplification.checked_mul(MAX_AMP_ADJUSTMENT_FACTOR)? ||
        target_amplification.checked_mul(MAX_AMP_ADJUSTMENT_FACTOR)? < current_amplification
    {
        return Err(ContractError::InvalidAmplification {});
    }

    // Save the target amplification
    TARGET_ONE_MINUS_AMP.save(
        deps.storage,
        &WAD
            .as_i256()                                         // Casting is safe, as WAD < I256.max
            .wrapping_sub(I256::from(target_amplification))    // 'wrapping_sub' is safe, as 'target_amplification' is always < WAD (checked shortly above)
    )?;

    // Set the amplification update time parameters
    AMP_UPDATE_FINISH_TIMESTAMP_SECONDS.save(deps.storage, &target_timestamp)?;
    AMP_UPDATE_TIMESTAMP_SECONDS.save(deps.storage, &current_time)?;

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


/// Perform an incremental amplification update.
/// 
/// **DEV-NOTE**: This function should be called at the beginning of amplification-dependent functions.
/// 
/// # Arguments:
/// * `current_timestamp` - The current time.
/// 
pub fn update_amplification(
    deps: &mut DepsMut,
    current_timestamp: Timestamp
) -> Result<(), ContractError> {

    // This algorithm incrementally adjusts the current amplification to the target amplification 
    // via linear interpolation.

    let current_timestamp = Uint64::new(current_timestamp.seconds());

    // Only run update logic if 'amp_update_finish_timestamp' is set
    let amp_update_finish_timestamp = AMP_UPDATE_FINISH_TIMESTAMP_SECONDS.load(deps.storage)?;
    if amp_update_finish_timestamp.is_zero() {
        return Ok(());
    }

    // Skip the update if the amplification has already been updated on the same block
    let amp_update_timestamp = AMP_UPDATE_TIMESTAMP_SECONDS.load(deps.storage)?;
    if current_timestamp == amp_update_timestamp {
        return Ok(());
    }

    let target_one_minus_amp = TARGET_ONE_MINUS_AMP.load(deps.storage)?;

    // If the 'amp_update_finish_timestamp' has been reached, finish the amplification update
    if current_timestamp >= amp_update_finish_timestamp {

        ONE_MINUS_AMP.save(deps.storage, &target_one_minus_amp)?;

        // Clear the 'amp_update_finish_timestamp' to disable the update logic
        AMP_UPDATE_FINISH_TIMESTAMP_SECONDS.save(
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
        //          => remaining_update_time > time_since_last_update, since amp_update_finish_timestamp > current_timestamp
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
    AMP_UPDATE_TIMESTAMP_SECONDS.save(
        deps.storage,
        &current_timestamp
    )?;

    Ok(())

}


/// Recompute the maximum security limit capacity.
pub fn update_max_limit_capacity(
    deps: &mut DepsMut,
    env: Env,
    info: &MessageInfo
) -> Result<Response, ContractError> {
    
    let assets = VaultAssets::load(&deps.as_ref())?;

    // Compute the sum of the vault's asset balance-weight products.
    let max_limit_capacity = assets.get_assets().iter()
        .try_fold(
            U256::zero(),
            |acc, asset| -> StdResult<_> {

                let vault_asset_balance = asset.query_prior_balance(
                    &deps.as_ref(),
                    &env,
                    Some(info)
                )?;

                let escrowed_balance = TOTAL_ESCROWED_ASSETS.load(deps.storage, asset.get_asset_ref())?;

                let effective_balance = vault_asset_balance.checked_sub(escrowed_balance)?;

                let weight = WEIGHTS.load(deps.storage, asset.get_asset_ref())?;

                acc.checked_add(
                    // 'wrapping_mul' is safe as U256.max >= Uint128.max * Uint128.max
                    U256::from(effective_balance).wrapping_mul(weight.into())
                ).map_err(|err| err.into())
            }
        )?;
    
    MAX_LIMIT_CAPACITY.save(deps.storage, &max_limit_capacity)?;

    Ok(
        Response::new()
    )
}



// Query helpers ****************************************************************************************************************

/// Query a 'send_asset' calculation (returned 'units' in WAD notation).
/// 
/// # Arguments:
/// * `from_asset_ref` - The source asset reference.
/// * `amount` - The `from_asset_ref` amount (excluding the vault fee).
/// 
pub fn query_calc_send_asset(
    deps: Deps,
    env: Env,
    from_asset_ref: &str,
    amount: Uint128
) -> StdResult<CalcSendAssetResponse> {

    Ok(
        CalcSendAssetResponse {
            u: calc_send_asset(
                &deps,
                env,
                None,
                &Asset::from_asset_ref(&deps, from_asset_ref)?,
                amount
            )?
        }
    )

}


/// Query a 'receive_asset' calculation.
/// 
/// # Arguments:
/// * `to_asset_ref` - The target asset reference.
/// * `u` - The incoming units (in WAD notation).
/// 
pub fn query_calc_receive_asset(
    deps: Deps,
    env: Env,
    to_asset_ref: &str,
    u: U256
) -> StdResult<CalcReceiveAssetResponse> {

    Ok(
        CalcReceiveAssetResponse {
            to_amount: calc_receive_asset(
                &deps,
                env,
                None,
                &Asset::from_asset_ref(&deps, to_asset_ref)?,
                u
            )?
        }
    )

}


/// Query a 'local_swap' calculation.
/// 
/// # Arguments:
/// * `from_asset_ref` - The source asset reference.
/// * `to_asset_ref` - The target asset reference.
/// * `amount` - The `from_asset_ref` amount (excluding the vault fee).
/// 
pub fn query_calc_local_swap(
    deps: Deps,
    env: Env,
    from_asset_ref: &str,
    to_asset_ref: &str,
    amount: Uint128
) -> StdResult<CalcLocalSwapResponse> {

    Ok(
        CalcLocalSwapResponse {
            to_amount: calc_local_swap(
                &deps,
                env,
                None,
                &Asset::from_asset_ref(&deps, from_asset_ref)?,
                &Asset::from_asset_ref(&deps, to_asset_ref)?,
                amount
            )?
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
            // NOTE: the common implementation of `get_limit_capacity` is divided by 2 because
            // of how the limit is implemented for amplified vaults.
            capacity: get_limit_capacity(&deps, env.block.time)? / u256!("2")
        }
    )

}


/// Query the vault amplification (in WAD notation).
pub fn query_amplification(
    deps: Deps
) -> StdResult<AmplificationResponse> {
    
    Ok(
        AmplificationResponse {
            amplification: WAD
                .as_i256()              // Casting is safe as WAD < I256.max
                .wrapping_sub(          // 'wrapping_sub' is safe as WAD >= 'one_minus_amp'
                    ONE_MINUS_AMP.load(deps.storage)?
                )
                .as_u64()               // Casting is safe as 'amplification' < u64.max
                .into()
        }
    )

}


/// Query the vault target amplification (in WAD notation).
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


/// Query the amplification update finish timestamp.
pub fn query_amplification_update_finish_timestamp(
    deps: Deps
) -> StdResult<AmplificationUpdateFinishTimestampResponse> {

    Ok(
        AmplificationUpdateFinishTimestampResponse {
            timestamp: AMP_UPDATE_FINISH_TIMESTAMP_SECONDS.load(deps.storage)?
        }
    )

}


// Query the vault's current balance0 (in WAD notation).
pub fn query_balance_0(
    deps: Deps,
    env: Env
) -> StdResult<Balance0Response> {

    Ok(
        Balance0Response {
            balance_0: calc_balance_0(
                deps,
                env,
                None,
                ONE_MINUS_AMP.load(deps.storage)?
            )?.0
        }
    )

}


// Query the vault's current unit tracker (in WAD notation).
pub fn query_unit_tracker(
    deps: Deps
) -> StdResult<UnitTrackerResponse> {

    Ok(
        UnitTrackerResponse {
            amount: UNIT_TRACKER.load(deps.storage)?
        }
    )

}
