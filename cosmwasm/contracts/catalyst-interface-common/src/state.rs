use catalyst_types::U256;
use catalyst_vault_common::{msg::{CommonQueryMsg, AssetResponse, ReceiverExecuteMsg, ExecuteMsg as VaultExecuteMsg}, bindings::{Asset, AssetTrait, CustomMsg, VaultResponse, IntoCosmosCustomMsg}};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{IbcEndpoint, Deps, Addr, DepsMut, Event, MessageInfo, Empty, Response, Uint64, Uint128, Binary, Env, from_binary, StdError, Coin, CosmosMsg, to_binary, SubMsgResponse, WasmMsg, SubMsg};
use cw_controllers::Admin;
use cw_storage_plus::{Map, Item};
use sha3::{Keccak256, Digest};
use std::ops::Div;

use crate::{ContractError, event::{set_owner_event, underwrite_swap_event, fulfill_underwrite_event}, catalyst_ibc_payload::{CatalystCalldata, parse_calldata}};

// Interface storage
pub const OPEN_CHANNELS: Map<&str, IbcChannelInfo> = Map::new("catalyst-interface-open-channels");

const ADMIN: Admin = Admin::new("catalyst-interface-admin");



// State
// ************************************************************************************************

const REPLY_CALLDATA_PARAMS: Item<CatalystCalldata> = Item::new("catalyst-interface-calldata-params");



// Reply
// ************************************************************************************************

pub const RECEIVE_ASSET_REPLY_ID     : u64 = 0x10;
pub const RECEIVE_LIQUIDITY_REPLY_ID : u64 = 0x20;
pub const UNDERWRITE_REPLY_ID        : u64 = 0x30;




//TODO move to 'ibc.rs'?
// IBC
// ************************************************************************************************

// Use a stripped down version of cosmwasm_std::IbcChannel to store the information of the
// interface's open channels.
#[cw_serde]
pub struct IbcChannelInfo {
    pub endpoint: IbcEndpoint,
    pub counterparty_endpoint: IbcEndpoint,
    pub connection_id: String
}




// Receive Helpers
// ************************************************************************************************

/// Handle the reception of a cross-chain asset swap.
/// 
/// # Arguments:
/// * `channel_id` - The source chain identifier.
/// * `to_vault` - The target vault.
/// * `to_asset_index` - The index of the purchased asset.
/// * `to_account` - The recipient of the swap.
/// * `u` - The incoming units.
/// * `min_out` - The mininum output amount.
/// * `underwrite_incentive_x16` - The share of the swap return that is offered to an underwriter as incentive.
/// * `from_vault` - The source vault on the source chain.
/// * `from_amount` - The `from_asset` amount sold to the source vault.
/// * `from_asset` - The source asset.
/// * `from_block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32).
/// * `calldata` - Arbitrary data to be executed upon successful execution of the swap.
/// 
pub fn handle_receive_asset(
    deps: &mut DepsMut,
    env: &Env,
    channel_id: String,
    to_vault: String,
    to_asset_index: u8,
    to_account: String,
    u: U256,
    min_out: Uint128,
    underwrite_incentive_x16: u16,
    from_vault: Binary,
    from_amount: U256,
    from_asset: Binary,
    from_block_number_mod: u32,
    calldata: Binary
) -> Result<VaultResponse, ContractError> {

    let to_asset = deps.querier.query_wasm_smart::<AssetResponse<Asset>>(
        to_vault.clone(),
        &CommonQueryMsg::AssetByIndex { asset_index: to_asset_index }
    )?.asset;

    let match_underwrite_response = match_underwrite(
        deps,
        env,
        &channel_id,
        &from_vault,
        &to_vault,
        &to_asset,
        &u,
        &min_out,
        &to_account,
        underwrite_incentive_x16,
        &calldata
    )?;

    match match_underwrite_response {
        Some(response) => Ok(response),
        None => {

            // Build the message to execute the reception of the swap.
            // NOTE: none of the fields are validated, these must be correctly handled by the vault.
            let wasm_msg = WasmMsg::Execute {
                contract_addr: to_vault,
                msg: to_binary(&VaultExecuteMsg::<()>::ReceiveAsset {
                    channel_id,
                    from_vault,
                    to_asset_index,
                    to_account,
                    u,
                    min_out,
                    from_amount,
                    from_asset,
                    from_block_number_mod
                })?,
                funds: vec![]
            };

            // If calldata exists, enable the 'reply' on the vault message to trigger the calldata
            // execution once the vault's 'receive' handler completes.
            let parsed_calldata = parse_calldata(
                deps.as_ref(),
                calldata
            )?;
            let sub_message = match parsed_calldata {
                Some(calldata) => {
                    calldata.save(deps)?;
                    SubMsg::reply_on_success(wasm_msg, RECEIVE_ASSET_REPLY_ID)
                },
                None => SubMsg::new(wasm_msg),
            };

            Ok(
                Response::new()
                    .add_submessage(sub_message)
            )

        },
    }
    
}


/// Handle the reception of a cross-chain liquidity swap.
/// 
/// # Arguments:
/// * `channel_id` - The source chain identifier.
/// * `to_vault` - The target vault.
/// * `to_account` - The recipient of the swap.
/// * `u` - The incoming units.
/// * `min_out` - The mininum output amount.
/// * `min_vault_tokens` - The mininum vault tokens output amount.
/// * `min_reference_asset` - The output amount's mininum reference asset value.
/// * `from_vault` - The source vault on the source chain.
/// * `from_amount` - The `from_asset` amount sold to the source vault.
/// * `from_block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32).
/// * `calldata` - Arbitrary data to be executed upon successful execution of the swap.
/// 
pub fn handle_receive_liquidity(
    deps: &mut DepsMut,
    channel_id: String,
    to_vault: String,
    to_account: String,
    u: U256,
    min_vault_tokens: Uint128,
    min_reference_asset: Uint128,
    from_vault: Binary,
    from_amount: U256,
    from_block_number_mod: u32,
    calldata: Binary
) -> Result<VaultResponse, ContractError> {

    // Build the message to execute the reception of the swap.
    // NOTE: none of the fields are validated, these must be correctly handled by the vault.
    let wasm_msg = WasmMsg::Execute {
        contract_addr: to_vault,    // No need to validate, 'Execute' will fail for an invalid address.
        msg: to_binary(&VaultExecuteMsg::<()>::ReceiveLiquidity {
            channel_id,
            from_vault,
            to_account,
            u,
            min_vault_tokens,
            min_reference_asset,
            from_amount,
            from_block_number_mod
        })?,
        funds: vec![]
    };

    // If calldata exists, enable the 'reply' on the vault message to trigger the calldata
    // execution once the vault's 'receive' handler completes.
    let parsed_calldata = parse_calldata(
        deps.as_ref(),
        calldata
    )?;
    let sub_message = match parsed_calldata {
        Some(calldata) => {
            calldata.save(deps)?;
            SubMsg::reply_on_success(wasm_msg, RECEIVE_LIQUIDITY_REPLY_ID)
        },
        None => SubMsg::new(wasm_msg),
    };

    Ok(
        Response::new()
            .add_submessage(sub_message)
    )

}



impl CatalystCalldata {

    /// Save the calldata parameters to the store.
    /// 
    /// **NOTE**: The call will only be successful if there is **no** data already saved on the
    /// store.
    /// 
    pub fn save(
        &self,
        deps: &mut DepsMut
    ) -> Result<(), ContractError> {

        if REPLY_CALLDATA_PARAMS.exists(deps.storage) {
            return Err(ContractError::Unauthorized {});
        }

        REPLY_CALLDATA_PARAMS.save(deps.storage, self)
            .map_err(|err| err.into())
    }

    /// Retrieve and remove the calldata parameters from the store.
    /// 
    /// **NOTE**: The call will fail if there is no data saved on the store.
    /// 
    pub fn remove(
        deps: &mut DepsMut
    ) -> Result<Self, ContractError> {

        let params = REPLY_CALLDATA_PARAMS.load(deps.storage)?;
        REPLY_CALLDATA_PARAMS.remove(deps.storage);

        Ok(params)
    }
}


/// Handle the calldata execution after the execution of a swap.
/// 
/// # Arguments:
/// * `swap_return` - The swap return.
/// 
pub fn handle_calldata_on_reply(
    mut deps: DepsMut,
    response: SubMsgResponse
) -> Result<VaultResponse, ContractError> {

    // Build the 'onCatalystCall' message using the swap return.

    let response_data = response.data.ok_or_else(|| {
        StdError::GenericErr { msg: "No data in the vault's `ReceiveAsset`/`ReceiveLiquidity` response.".to_string() }
    })?;
    let swap_return: Uint128 = from_binary(&response_data)?;

    let CatalystCalldata {
        target,
        bytes
    } = CatalystCalldata::remove(&mut deps)?;

    let calldata_message = create_on_catalyst_call_msg(
        target.to_string(),
        swap_return,
        bytes
    )?;

    Ok(
        Response::new()
            .add_message(calldata_message)
    )

}


// Underwriting
// ************************************************************************************************

pub const UNDERWRITING_COLLATERAL: Uint128 = Uint128::new(35);          // 3.5% collateral
pub const UNDERWRITING_COLLATERAL_BASE: Uint128 = Uint128::new(1000);

pub const UNDERWRITING_EXPIRE_REWARD: Uint128 = Uint128::new(350);      // 35% of the collateral
pub const UNDERWRITING_EXPIRE_REWARD_BASE: Uint128 = Uint128::new(1000);

pub const MAX_UNDERWRITE_DURATION_ALLOWED_SECONDS: Uint64 = Uint64::new(15 * 24 * 60 * 60); // 15 days

pub const MAX_UNDERWRITE_DURATION_SECONDS: Item<Uint64> = Item::new("catalyst-interface-max-underwrite-duration");
pub const UNDERWRITE_EVENTS: Map<Vec<u8>, UnderwriteEvent> = Map::new("catalyst-interface-underwrite-events");

const REPLY_UNDERWRITE_PARAMS: Item<UnderwriteParams> = Item::new("catalyst-interface-underwrite-params");


/// Record of an underwrite event. Used to finish the underwriting logic upon reception of the
/// underwritten swap or expiry of the underwrite.
#[cw_serde]
pub struct UnderwriteEvent {
    pub amount: Uint128,
    pub underwriter: Addr,
    pub expiry: Uint64
}

impl UnderwriteEvent {

    /// Save the event to the store under the given identifier.
    /// 
    /// **NOTE**: The call will only be successful if there is **no** data already saved on the
    /// store under the given identifier.
    /// 
    /// # Arguments:
    /// * `identifier` - The underwrite identifier under which to save the event.
    /// 
    pub fn save(
        &self,
        deps: &mut DepsMut,
        identifier: Binary
    ) -> Result<(), ContractError> {

        let key = UNDERWRITE_EVENTS.key(identifier.0);

        if key.may_load(deps.storage)?.is_some() {
            Err(ContractError::Unauthorized {})
        }
        else {
            key.save(deps.storage, self)?;
            Ok(())
        }
    }

    /// Retrieve and remove the event from the store of the given identifier.
    /// 
    /// **NOTE**: Will return `None` if there is no data saved on the store with the given
    /// identifier.
    /// 
    /// # Arguments:
    /// * `identifier` - The underwrite identifier of which to retrieve and remove the event.
    /// 
    pub fn remove(
        deps: &mut DepsMut,
        identifier: Binary
    ) -> Result<Option<Self>, ContractError> {

        let key = UNDERWRITE_EVENTS.key(identifier.0);

        let event = key.may_load(deps.storage)?;
        if event.is_some() {
            key.remove(deps.storage);
        }

        Ok(event)
    }
}


#[cw_serde]
pub struct UnderwriteParams {
    pub identifier: Binary,
    pub underwriter: Addr,
    pub to_vault: String,
    pub to_asset_ref: String,
    pub to_account: String,
    pub underwrite_incentive_x16: u16,
    pub calldata: Option<CatalystCalldata>,
    pub funds: Vec<Coin>
}

impl UnderwriteParams {

    /// Save the underwrite parameters to the store.
    /// 
    /// **NOTE**: The call will only be successful if there is **no** data already saved on the
    /// store.
    /// 
    pub fn save(
        &self,
        deps: &mut DepsMut
    ) -> Result<(), ContractError> {

        if REPLY_UNDERWRITE_PARAMS.exists(deps.storage) {
            return Err(ContractError::Unauthorized {});
        }

        REPLY_UNDERWRITE_PARAMS.save(deps.storage, self)
            .map_err(|err| err.into())
    }

    /// Retrieve and remove the underwrite parameters from the store.
    /// 
    /// **NOTE**: The call will fail if there is no data saved on the store.
    /// 
    pub fn remove(
        deps: &mut DepsMut
    ) -> Result<Self, ContractError> {

        let params = REPLY_UNDERWRITE_PARAMS.load(deps.storage)?;
        REPLY_UNDERWRITE_PARAMS.remove(deps.storage);

        Ok(params)
    }
}


/// Compute the underwriting identifier of the provided underwrite parameters.
/// 
/// # Arguments:
/// * `to_vault` - The target vault.
/// * `to_asset_ref` - The destination asset.
/// * `u` - The underwritten units.
/// * `min_out` - The mininum `to_asset_ref` output amount to get on the target vault.
/// * `to_account` - The recipient of the swap.
/// * `underwrite_incentive_x16` - The underwriting incentive.
/// * `calldata` - The swap calldata.
/// 
pub fn get_underwrite_identifier(
    to_vault: &str,
    to_asset_ref: &str,
    u: &U256,
    min_out: &Uint128,
    to_account: &str,
    underwrite_incentive_x16: u16,
    calldata: &Binary
) -> Binary {

    // Initialize vec with the specified capacity to avoid reallocations
    let mut identifier_data: Vec<u8> = Vec::with_capacity(
        to_vault.len()
            + to_asset_ref.len()
            + 32
            + 16
            + to_account.len()
            + 2
            + calldata.len()
    );

    identifier_data.extend_from_slice(to_vault.as_bytes());
    identifier_data.extend_from_slice(to_asset_ref.as_bytes());
    identifier_data.extend_from_slice(&u.to_be_bytes());
    identifier_data.extend_from_slice(&min_out.to_be_bytes());
    identifier_data.extend_from_slice(to_account.as_bytes());
    identifier_data.extend_from_slice(&underwrite_incentive_x16.to_be_bytes());
    identifier_data.extend_from_slice(&calldata.0);

    let mut hasher = Keccak256::new();
    hasher.update(identifier_data);
    Binary(hasher.finalize().to_vec())
}


/// Resume the underwriting logic after the successful execution of the `UnderwriteAsset` call on
/// the destination vault.
/// 
/// # Arguments:
/// * `response` - The response of the `UnderwriteAsset` message.
/// 
pub fn handle_underwrite_reply(
    mut deps: DepsMut,
    env: Env,
    response: SubMsgResponse
) -> Result<VaultResponse, ContractError> {

    let UnderwriteParams {
        identifier,
        underwriter,
        to_vault,
        to_asset_ref,
        to_account,
        underwrite_incentive_x16,
        calldata,
        funds,
    } = UnderwriteParams::remove(&mut deps)?;


    // Store the underwrite 'event' (i.e. store that an underwrite is active)
    // ! This must only be successful if the `indentifier` is **not** already in use.
    let response_data = response.data.ok_or_else(|| {
        StdError::GenericErr { msg: "No data in the vault's `UnderwriteAsset` response.".to_string() }
    })?;
    let swap_return: Uint128 = from_binary(&response_data)?;
    let expiry = Uint64::new(env.block.time.seconds()) + get_max_underwrite_duration(&deps.as_ref())?;

    let underwrite_event = UnderwriteEvent {
        amount: swap_return,
        underwriter: underwriter.clone(),
        expiry,
    };
    underwrite_event.save(&mut deps, identifier.clone())?;


    // Query the asset information from the destination vault
    let asset = deps.querier.query_wasm_smart::<AssetResponse<Asset>>(
        to_vault,
        &CommonQueryMsg::Asset { asset_ref: to_asset_ref }
    )?.asset;


    // Transfer the corresponding asset amounts
    //   -> receive_underwriter_amount = swap_return + collateral 
    //   -> send_recipient_amount      = swap_return - underwrite_incentive

    let receive_underwriter_amount = swap_return
        .checked_mul(UNDERWRITING_COLLATERAL_BASE + UNDERWRITING_COLLATERAL)?
        .div(UNDERWRITING_COLLATERAL_BASE);

    let underwrite_incentive_x16 = Uint128::new(underwrite_incentive_x16 as u128);
    let underwrite_incentive = (swap_return.checked_mul(underwrite_incentive_x16)?) >> 16;
    let send_recipient_amount = swap_return
        .wrapping_sub(underwrite_incentive);  // 'wrapping_sub' safe, as `underwrite_incentive` is always < `swap_return`

    let receive_asset_msg = asset.receive_asset_with_refund(
        &env,
        &MessageInfo { sender: underwriter.clone(), funds },
        receive_underwriter_amount,
        None
    ).map_err(|err| StdError::from(err))?;

    let send_asset_msg = asset.send_asset(
        &env,
        send_recipient_amount,
        to_account
    ).map_err(|err| StdError::from(err))?;


    let calldata_message = match calldata {
        Some(calldata) => Some(
            create_on_catalyst_call_msg(
                calldata.target.to_string(),
                send_recipient_amount,
                calldata.bytes
            )?
        ),
        None => None,
    };


    // Build the response
    let mut response = Response::new();

    if let Some(msg) = receive_asset_msg {
        response = response.add_message(msg.into_cosmos_vault_msg());
    }
    
    if let Some(msg) = send_asset_msg {
        response = response.add_message(msg.into_cosmos_vault_msg());
    }

    if let Some(msg) = calldata_message {
        response = response.add_message(msg);
    }

    Ok(response
        .add_event(
            underwrite_swap_event(
                identifier,
                underwriter,
                expiry
            )
        )
    )

}


/// Match the an incoming asset swap with an underwrite event. Returns `None` if no underwrite
/// is found.
/// 
/// # Arguments:
/// * `to_vault` - The target vault.
/// * `to_asset` - The destination asset.
/// * `u` - The underwritten units.
/// * `min_out` - The mininum `to_asset_ref` output amount to get on the target vault.
/// * `to_account` - The recipient of the swap.
/// * `underwrite_incentive_x16` - The underwriting incentive share.
/// * `calldata` - The swap calldata.
/// 
pub fn match_underwrite(
    deps: &mut DepsMut,
    env: &Env,
    channel_id: &String,
    from_vault: &Binary,
    to_vault: &str,
    to_asset: &Asset,
    u: &U256,
    min_out: &Uint128,
    to_account: &str,
    underwrite_incentive_x16: u16,
    calldata: &Binary
) -> Result<Option<VaultResponse>, ContractError> {

    let identifier = get_underwrite_identifier(
        to_vault,
        to_asset.get_asset_ref(),
        u,
        min_out,
        to_account,
        underwrite_incentive_x16,
        calldata
    );

    // Get and delete the underwrite event
    let underwrite_event = UnderwriteEvent::remove(
        deps,
        identifier.clone()
    )?;


    if underwrite_event.is_none() {
        return Ok(None);
    }


    let UnderwriteEvent {
        amount: underwritten_amount,
        underwriter,
        expiry: _,
    } = underwrite_event.unwrap();  // Unwrap safe: if the event is `None`, the function returns on
                                    // the previous statement.

    // Call the vault to release the underwrite escrow
    let release_underwrite_messsage = WasmMsg::Execute {
        contract_addr: to_vault.to_owned(),
        msg: to_binary(&VaultExecuteMsg::<()>::ReleaseUnderwriteAsset {
            channel_id: channel_id.to_owned(),
            from_vault: from_vault.to_owned(),
            identifier: identifier.clone(),
            asset_ref: to_asset.get_asset_ref().to_owned(),
            escrow_amount: underwritten_amount,
            recipient: underwriter.to_string()
        })?,
        funds: vec![]
    };

    //  Send the underwrite collateral plus the incentive to the underwriter.
    let underwriter_collateral = underwritten_amount
        .checked_mul(UNDERWRITING_COLLATERAL)?
        .div(UNDERWRITING_COLLATERAL_BASE);

    let underwrite_incentive_x16 = Uint128::new(underwrite_incentive_x16 as u128);
    let underwrite_incentive = (underwritten_amount.checked_mul(underwrite_incentive_x16)?) >> 16;
    
    let underwriter_payment = underwriter_collateral.checked_add(underwrite_incentive)?;

    let underwriter_payment_msg = to_asset.send_asset(
        env,
        underwriter_payment,
        underwriter.to_string()
    ).map_err(|err| StdError::from(err))?;

    // Build the response
    let mut response = Response::new()
        .add_message(release_underwrite_messsage);
    
    if let Some(msg) = underwriter_payment_msg {
        response = response.add_message(msg.into_cosmos_vault_msg());
    }
    
    Ok(Some(response
        .add_event(
            fulfill_underwrite_event(
                identifier
            )
        )
    ))

}


/// Set the maximum underwriting duration (only applies to new underwrite orders).
/// 
/// NOTE: This function checks that the sender of the transaction is the current interface owner.
/// 
/// # Arguments:
/// * `new_max_duration` - The new desired maximum underwriting duration.
/// 
pub fn set_max_underwriting_duration(
    deps: &mut DepsMut,
    info: &MessageInfo,
    new_max_duration: Uint64
) -> Result<VaultResponse, ContractError> {

    only_owner(deps.as_ref(), info)?;

    if new_max_duration > MAX_UNDERWRITE_DURATION_ALLOWED_SECONDS {
        return Err(ContractError::MaxUnderwriteDurationTooLong {
            set_duration: new_max_duration,
            max_duration: MAX_UNDERWRITE_DURATION_ALLOWED_SECONDS,
        })
    }

    MAX_UNDERWRITE_DURATION_SECONDS.save(deps.storage, &new_max_duration)?;

    Ok(Response::new())

}


/// Get the maximum underwriting duration.
pub fn get_max_underwrite_duration(
    deps: &Deps
) -> Result<Uint64, ContractError> {

    MAX_UNDERWRITE_DURATION_SECONDS
        .load(deps.storage)
        .map_err(|err| err.into())

}



// OnCatalystCall
// ************************************************************************************************

/// Create the 'OnCatalystCall' execution message.
/// 
/// # Arguments:
/// * `calldata_target` - The contract address to invoke.
/// * `purchased_tokens` - The swap return.
/// * `data` - Arbitrary data to be passed onto the `calldata_target`.
/// 
pub fn create_on_catalyst_call_msg(
    calldata_target: String,
    purchased_tokens: Uint128,
    data: Binary
) -> Result<CosmosMsg<CustomMsg>, ContractError> {

    Ok(CosmosMsg::Wasm(
        cosmwasm_std::WasmMsg::Execute {
            contract_addr: calldata_target,
            msg: to_binary(&ReceiverExecuteMsg::OnCatalystCall {
                purchased_tokens,
                data
            })?,
            funds: vec![]
        }
    ))

}



// Admin
// ************************************************************************************************

/// Get the current interface owner.
pub fn owner(
    deps: Deps
) -> Result<Option<Addr>, ContractError> {

    ADMIN.get(deps)
        .map_err(|err| err.into())

}

/// Assert that the message sender is the interface owner.
pub fn only_owner(
    deps: Deps,
    info: &MessageInfo
) -> Result<(), ContractError> {
 
    match is_owner(deps, &info.sender)? {
        true => Ok(()),
        false => Err(ContractError::Unauthorized {})
    }
}

/// Check if an address is the interface owner.
/// 
/// Arguments:
/// 
/// * `account` - The address of the account to check whether it is the interface owner.
/// 
pub fn is_owner(
    deps: Deps,
    account: &Addr,
) -> Result<bool, ContractError> {

    ADMIN.is_admin(deps, account)
        .map_err(|err| err.into())

}

/// Set the interface owner.
/// 
/// !IMPORTANT: This function DOES NOT check the sender of the transaction.
/// 
/// # Arguments
/// 
/// * `account` - The new interface owner.
/// 
pub fn set_owner_unchecked(
    deps: DepsMut,
    account: Addr
) -> Result<Event, ContractError> {
    
    ADMIN.set(deps, Some(account.clone()))?;
    
    Ok(
        set_owner_event(account.to_string())
    )
}

/// Update the interface owner.
/// 
/// NOTE: This function checks that the sender of the transaction is the current interface owner.
/// 
/// # Arguments
/// 
/// * `account` - The new interface owner.
/// 
pub fn update_owner(
    deps: DepsMut,
    info: MessageInfo,
    account: String
) -> Result<VaultResponse, ContractError> {

    // Validate the new owner account
    let account = deps.api.addr_validate(account.as_str())?;

    // ! The 'update' call also verifies whether the caller of the transaction is the current interface owner
    ADMIN.execute_update_admin::<Empty, Empty>(deps, info, Some(account.clone()))
        .map_err(|err| {
            match err {
                cw_controllers::AdminError::Std(err) => err.into(),
                cw_controllers::AdminError::NotAdmin {} => ContractError::Unauthorized {},
            }
        })?;

    Ok(
        Response::new()
            .add_event(set_owner_event(account.to_string()))
    )

}