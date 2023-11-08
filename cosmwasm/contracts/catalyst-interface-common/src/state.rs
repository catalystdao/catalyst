use catalyst_types::U256;
use catalyst_vault_common::{msg::{CommonQueryMsg, AssetResponse, ReceiverExecuteMsg, ExecuteMsg as VaultExecuteMsg, VaultConnectionStateResponse}, bindings::{Asset, AssetTrait, CustomMsg, IntoCosmosCustomMsg}};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Deps, Addr, DepsMut, Event, MessageInfo, Empty, Response, Uint64, Uint128, Binary, Env, from_binary, StdError, Coin, CosmosMsg, to_binary, SubMsgResponse, WasmMsg, SubMsg, ReplyOn, StdResult, SubMsgResult, Reply};
use cw0::parse_execute_response_data;
use cw_controllers::Admin;
use cw_storage_plus::{Map, Item};
use sha3::{Keccak256, Digest};
use std::ops::Div;

use crate::{catalyst_payload::{CatalystV1SendAssetPayload, SendAssetVariablePayload, CatalystV1SendLiquidityPayload, SendLiquidityVariablePayload, CatalystEncodedAddress, CatalystCalldata, parse_calldata, CatalystV1Packet}, msg::UnderwriteIdentifierResponse, event::set_max_underwrite_duration_event};
use crate::error::ContractError;
use crate::event::{set_owner_event, underwrite_swap_event, fulfill_underwrite_event, expire_underwrite_event};
use crate::bindings::InterfaceResponse;




// Constants
// ************************************************************************************************

// Underwriting
pub const UNDERWRITING_COLLATERAL: Uint128 = Uint128::new(35);          // 3.5% collateral
pub const UNDERWRITING_COLLATERAL_BASE: Uint128 = Uint128::new(1000);

pub const UNDERWRITING_EXPIRE_REWARD: Uint128 = Uint128::new(350);      // 35% of the collateral
pub const UNDERWRITING_EXPIRE_REWARD_BASE: Uint128 = Uint128::new(1000);

pub const DEFAULT_MIN_UNDERWRITE_DURATION_ALLOWED_BLOCKS: Uint64 = Uint64::new(12 * 60 * 60);      // 12 hours at 1 block/s
pub const DEFAULT_MAX_UNDERWRITE_DURATION_ALLOWED_BLOCKS: Uint64 = Uint64::new(15 * 24 * 60 * 60); // 15 days at 1 block/s

pub const UNDERWRITE_BUFFER_BLOCKS: Uint64 = Uint64::new(4);




// State
// ************************************************************************************************

// Admin
const ADMIN: Admin = Admin::new("catalyst-interface-admin");

// Calldata
const REPLY_CALLDATA_PARAMS: Item<CatalystCalldata> = Item::new("catalyst-interface-calldata-params");

// Underwriting
pub const MAX_UNDERWRITE_DURATION_BLOCKS: Item<Uint64> = Item::new("catalyst-interface-max-underwrite-duration");
pub const UNDERWRITE_EVENTS: Map<Vec<u8>, UnderwriteEvent> = Map::new("catalyst-interface-underwrite-events");
pub const UNDERWRITE_EXPIRIES: Map<Vec<u8>, UnderwriteExpiry> = Map::new("catalyst-interface-underwrite-expiries");

const REPLY_UNDERWRITE_PARAMS: Item<UnderwriteParams> = Item::new("catalyst-interface-underwrite-params");




// Reply
// ************************************************************************************************

pub const SET_ACK_REPLY_ID       : u64 = 0x100;
pub const RUN_CALLDATA_REPLY_ID  : u64 = 0x200;
pub const UNDERWRITE_REPLY_ID    : u64 = 0x300;




// Acknowledgements
// ************************************************************************************************

pub const ACK_SUCCESS: u8 = 0x00;
pub const ACK_FAIL: u8 = 0x01;

/// Generate a 'success' ack data response.
pub fn ack_success() -> Binary {
    Into::<Binary>::into(vec![ACK_SUCCESS])
}

/// Generate a 'fail' ack data response.
pub fn ack_fail() -> Binary {
    Into::<Binary>::into(vec![ACK_FAIL])
}




// Instantiation Helpers
// ************************************************************************************************

/// Setup the interface on instantiation.
/// 
/// # Arguments:
/// * `max_underwrite_duration` - The initial maximum underwrite duration.
/// * `min_underwrite_duration_allowed` - The minimum underwrite duration allowed. If `None`, 
/// defaults to a hardcoded constant.
/// * `max_underwrite_duration_allowed` - The maximum underwrite duration allowed. If `None`, 
/// defaults to a hardcoded constant.
/// 
pub fn setup(
    mut deps: DepsMut,
    info: MessageInfo,
    max_underwrite_duration: Uint64,
    min_underwrite_duration_allowed: Option<Uint64>,
    max_underwrite_duration_allowed: Option<Uint64>
) -> Result<InterfaceResponse, ContractError> {

    //TODO event
    set_max_underwriting_duration_unchecked(
        &mut deps,
        max_underwrite_duration,
        min_underwrite_duration_allowed,
        max_underwrite_duration_allowed
    )?;

    let set_owner_event = set_owner_unchecked(deps, info.sender)?;

    Ok(
        Response::new()
            .add_event(set_owner_event)
    )

}




// Send Handlers
// ************************************************************************************************

/// Pack the arguments of a 'send_asset' transaction into a byte array following Catalyst's
/// payload definition.
/// 
/// # Arguments: 
/// * `to_vault` - The target vault on the target chain (Catalyst encoded).
/// * `to_account` - The recipient of the swap on the target chain (Catalyst encoded).
/// * `to_asset_index` - The destination asset index.
/// * `u` - The outgoing 'units'.
/// * `min_out` - The mininum `to_asset` output amount to get on the target vault.
/// * `from_amount` - The `from_asset` amount sold to the vault.
/// * `from_asset` - The source asset.
/// * `underwrite_incentive_x16` - The share of the swap return that is offered to an underwriter as incentive.
/// * `block_number` - The block number at which the transaction has been committed.
/// * `calldata` - Arbitrary data to be executed on the target chain upon successful execution of the swap.
/// 
pub fn encode_send_cross_chain_asset(
    info: MessageInfo,
    to_vault: Binary,
    to_account: Binary,
    to_asset_index: u8,
    u: U256,
    min_out: U256,
    from_amount: Uint128,
    from_asset: String,
    underwrite_incentive_x16: u16,
    block_number: u32,
    calldata: Binary
) -> Result<Binary, ContractError> {

    // Build the payload
    let payload = CatalystV1SendAssetPayload {
        from_vault: CatalystEncodedAddress::try_encode(info.sender.as_bytes())?,
        to_vault: CatalystEncodedAddress::try_from(to_vault)?,        // 'to_vault' should already be correctly encoded
        to_account: CatalystEncodedAddress::try_from(to_account)?,    // 'to_account' should already be correctly encoded
        u,
        variable_payload: SendAssetVariablePayload {
            to_asset_index,
            min_out,
            from_amount: U256::from(from_amount),
            from_asset: CatalystEncodedAddress::try_encode(from_asset.as_bytes())?,
            block_number,
            underwrite_incentive_x16,
            calldata,
        },
    };

    Ok(
        payload.try_encode()?.into()    // Encode the parameters into a byte vector
    )
}


/// Pack the arguments of a 'send_liquidity' transaction into a byte array following Catalyst's
/// payload definition.
/// 
/// # Arguments: 
/// * `to_vault` - The target vault on the target chain (Catalyst encoded).
/// * `to_account` - The recipient of the swap on the target chain (Catalyst encoded).
/// * `u` - The outgoing 'units'.
/// * `min_vault_tokens` - The mininum vault tokens output amount to get on the target vault.
/// * `min_reference_asset` - The mininum reference asset value on the target vault.
/// * `from_amount` - The `from_asset` amount sold to the vault.
/// * `block_number` - The block number at which the transaction has been committed.
/// * `calldata` - Arbitrary data to be executed on the target chain upon successful execution of the swap.
/// 
pub fn encode_send_cross_chain_liquidity(
    info: MessageInfo,
    to_vault: Binary,
    to_account: Binary,
    u: U256,
    min_vault_tokens: U256,
    min_reference_asset: U256,
    from_amount: Uint128,
    block_number: u32,
    calldata: Binary
) -> Result<Binary, ContractError> {

    let payload = CatalystV1SendLiquidityPayload {
        from_vault: CatalystEncodedAddress::try_encode(info.sender.as_bytes())?,
        to_vault: CatalystEncodedAddress::try_from(to_vault)?,        // 'to_vault' should already be correctly encoded
        to_account: CatalystEncodedAddress::try_from(to_account)?,    // 'to_account' should already be correctly encoded
        u,
        variable_payload: SendLiquidityVariablePayload {
            min_vault_tokens,
            min_reference_asset,
            from_amount: U256::from(from_amount),
            block_number,
            calldata,
        },
    };

    Ok(
        payload.try_encode()?.into()     // Encode the parameters into a byte vector
    )
}




// Receive Handlers
// ************************************************************************************************

// ! The Catalyst interface is designed to allow the return of a response upon reception of a cross
// ! chain message. If it is desired to return a response when a message reception causes an error,
// ! the error must be catched to prevent the transaction from failing (as if the transaction
// ! fails, the response is not emitted).
// !
// ! Because of this 'error catching', it is very important for the receive handlers' message 
// ! execution/reply/ack logic to adhere to the following rules to prevent committing partial state
// ! changes:
// !   - A success ack is only returned if **ALL** messages succeed.
// !   - A fail ack is only returned when only one message has been sent and it has failed.
// !
// ! In practice this translates to:
// !   - Receive asset/liquidity execution (1 message)
// !      -> Vault execution passes: return Ok(success-ack)
// !      -> Vault execution fails: return Ok(fail-ack)
// !  
// !   - Receive asset/liquidity execution WITH CALLDATA EXECUTED ON REPLY
// !     (2 messages, 'calldata' message sent on 'reply')
// !      -> Vault execution passes: execute calldata on reply
// !          -> Calldata execution passes: Ok(success-ack)
// !          -> Calldata execution fails: FAIL to revert vault execution (do not enable reply)
// !      -> Vault execution fails: Ok(fail-ack)
// !  
// !   - Receive asset WITH UNDERWRITE
// !     (2 messages, both triggered at the same time)
// !      -> Both pass: Ok(success-ack) (enable reply on success on the second message)
// !      -> Either fails: FAIL (do not enable reply). Note: returning a fail-ack on failure of the
// !         first message is not enough, as this would allow the second message to execute.
// !
// ! If an interface implementation always requires the 'receive' transaction **NOT** to fail,
// ! messages generated by the interface which may lead to FAIL cases must be wrapped within a
// ! further message. The interface implements a `WrapSubMsgs` message for this purpose. This is
// ! not implemented by default, as the message wrapping results in an increased gas cost for a
// ! behavior which might not be needed by the implementing interface.


/// Handle the execution of the given Catalyst message.
/// 
/// # Arguments:
/// * `channel_id` - The source chain identifier.
/// * `data` - The Catalyst payload bytes.
/// 
pub fn handle_message_reception(
    deps: &mut DepsMut,
    env: &Env,
    channel_id: String,
    data: Binary
) -> Result<InterfaceResponse, ContractError> {

    let catalyst_packet = CatalystV1Packet::try_decode(data)?;

    match catalyst_packet {

        CatalystV1Packet::SendAsset(payload) => {

            handle_receive_asset(
                deps,
                &env,
                channel_id,
                payload.to_vault.try_decode_as_string()?,
                payload.variable_payload.to_asset_index,
                payload.to_account.try_decode_as_string()?,
                payload.u,
                payload.variable_payload.min_out,
                payload.variable_payload.underwrite_incentive_x16,
                payload.from_vault.to_binary(),
                payload.variable_payload.from_amount,
                payload.variable_payload.from_asset.to_binary(),
                payload.variable_payload.block_number,
                payload.variable_payload.calldata
            )
        },

        CatalystV1Packet::SendLiquidity(payload) => {

            handle_receive_liquidity(
                deps,
                channel_id,
                payload.to_vault.try_decode_as_string()?,
                payload.to_account.try_decode_as_string()?,
                payload.u,
                payload.variable_payload.min_vault_tokens,
                payload.variable_payload.min_reference_asset,
                payload.from_vault.to_binary(),
                payload.variable_payload.from_amount,
                payload.variable_payload.block_number,
                payload.variable_payload.calldata
            )
        }
    }
}


/// Handle the response of the given Catalyst message.
/// 
/// # Arguments:
/// * `channel_id` - The source chain identifier.
/// * `data` - The Catalyst payload bytes.
/// * `response` - The response bytes. 'None' if there has been no response.
/// 
pub fn handle_message_response(
    channel_id: String,
    data: Binary,
    response: Option<Binary>
) -> Result<InterfaceResponse, ContractError> {

    let catalyst_packet = CatalystV1Packet::try_decode(data)?;

    match catalyst_packet {

        CatalystV1Packet::SendAsset(payload) => {

            handle_send_asset_response(
                channel_id,
                payload.to_account.to_binary(),
                payload.u,
                payload.from_vault.try_decode_as_string()?,
                payload.variable_payload.from_amount,
                payload.variable_payload.from_asset.try_decode_as_string()?,
                payload.variable_payload.block_number,
                response
            )
        },

        CatalystV1Packet::SendLiquidity(payload) => {

            handle_send_liquidity_response(
                channel_id,
                payload.to_account.to_binary(),
                payload.u,
                payload.from_vault.try_decode_as_string()?,
                payload.variable_payload.from_amount,
                payload.variable_payload.block_number,
                response
            )
        }
    }
}


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
    min_out: U256,
    underwrite_incentive_x16: u16,
    from_vault: Binary,
    from_amount: U256,
    from_asset: Binary,
    from_block_number_mod: u32,
    calldata: Binary
) -> Result<InterfaceResponse, ContractError> {

    // Convert min_out into Uint128
    let min_out: Uint128 = min_out.try_into()
        .map_err(|_| ContractError::PayloadDecodingError {})?;

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
                    SubMsg::reply_always(wasm_msg, RUN_CALLDATA_REPLY_ID)   // ! Always 'reply' to execute calldata/set failed ack
                },
                None => SubMsg::reply_always(wasm_msg, SET_ACK_REPLY_ID),   // ! Always 'reply' to set ack
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
    min_vault_tokens: U256,
    min_reference_asset: U256,
    from_vault: Binary,
    from_amount: U256,
    from_block_number_mod: u32,
    calldata: Binary
) -> Result<InterfaceResponse, ContractError> {

    // Convert the minimum outputs into Uint128
    let min_vault_tokens: Uint128 = min_vault_tokens.try_into()
        .map_err(|_| ContractError::PayloadDecodingError {})?;
    let min_reference_asset: Uint128 = min_reference_asset.try_into()
        .map_err(|_| ContractError::PayloadDecodingError {})?;

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
            SubMsg::reply_always(wasm_msg, RUN_CALLDATA_REPLY_ID)   // ! Always 'reply' to execute calldata/set failed ack
        },
        None => SubMsg::reply_always(wasm_msg, SET_ACK_REPLY_ID),   // ! Always 'reply' to set ack
    };

    Ok(
        Response::new()
            .add_submessage(sub_message)
    )

}


/// Handle the response of a 'send asset' swap.
/// 
/// # Arguments: 
/// * `channel_id` - The target chain identifier.
/// * `to_account` - The recipient of the swap on the target chain (Catalyst encoded).
/// * `u` - The swapped 'units'.
/// * `from_vault` - The source vault.
/// * `from_amount` - The `from_asset` amount sold to the vault (excl. fee).
/// * `from_asset` - The source asset.
/// * `from_block_number_mod` - The block number at which the transaction was committed.
/// * `response` - The response bytes ('None' if no response).
/// 
pub fn handle_send_asset_response(
    channel_id: String,
    to_account: Binary,
    u: U256,
    from_vault: String,
    from_amount: U256,
    from_asset: String,
    from_block_number_mod: u32,
    response: Option<Binary>
) -> Result<InterfaceResponse, ContractError> {

    // NOTE: Only the first byte of the 'ack' response is checked. This allows future 'ack' implementations to
    // extend the 'ack' format.
    let success = response.is_some_and(|response| {
        response.get(0).is_some_and(|byte| byte == &ACK_SUCCESS)
    });

    // Convert 'from_amount' into Uint128
    let from_amount: Uint128 = from_amount.try_into()
        .map_err(|_| ContractError::PayloadDecodingError {})?;

    // Build the message to execute the success/fail call.
    // NOTE: none of the fields are validated, these must be correctly handled by the vault.
    let msg = match success {
        true => VaultExecuteMsg::<()>::OnSendAssetSuccess {
            channel_id,
            to_account,
            u,
            escrow_amount: from_amount,
            asset_ref: from_asset,
            block_number_mod: from_block_number_mod
        },
        false => VaultExecuteMsg::<()>::OnSendAssetFailure {
            channel_id,
            to_account,
            u,
            escrow_amount: from_amount,
            asset_ref: from_asset,
            block_number_mod: from_block_number_mod
        },
    };

    let response_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: from_vault,    // No need to validate, 'Execute' will fail for an invalid address.
        msg: to_binary(&msg)?,
        funds: vec![]
    });

    Ok(
        Response::new().add_message(response_msg)
    )
}


/// Handle the response of a 'send liquidity' swap.
/// 
/// # Arguments: 
/// * `channel_id` - The target chain identifier.
/// * `to_account` - The recipient of the swap on the target chain (Catalyst encoded).
/// * `u` - The swapped 'units'.
/// * `from_vault` - The source vault.
/// * `from_amount` - The `from_asset` amount sold to the vault.
/// * `from_block_number_mod` - The block number at which the transaction was committed.
/// * `response` - The response bytes ('None' if no response).
/// 
pub fn handle_send_liquidity_response(
    channel_id: String,
    to_account: Binary,
    u: U256,
    from_vault: String,
    from_amount: U256,
    from_block_number_mod: u32,
    response: Option<Binary>
) -> Result<InterfaceResponse, ContractError> {

    // NOTE: Only the first byte of the 'ack' response is checked. This allows future 'ack' implementations to
    // extend the 'ack' format.
    let success = response.is_some_and(|response| {
        response.get(0).is_some_and(|byte| byte == &ACK_SUCCESS)
    });

    // Convert 'from_amount' into Uint128
    let from_amount: Uint128 = from_amount.try_into()
        .map_err(|_| ContractError::PayloadDecodingError {})?;

    // Build the message to execute the success/fail call.
    // NOTE: none of the fields are validated, these must be correctly handled by the vault.
    let msg = match success {
        true => VaultExecuteMsg::<()>::OnSendLiquiditySuccess {
            channel_id,
            to_account,
            u,
            escrow_amount: from_amount,
            block_number_mod: from_block_number_mod
        },
        false => VaultExecuteMsg::<()>::OnSendLiquidityFailure {
            channel_id,
            to_account,
            u,
            escrow_amount: from_amount,
            block_number_mod: from_block_number_mod
        },
    };

    let response_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: from_vault,    // No need to validate, 'Execute' will fail for an invalid address.
        msg: to_binary(&msg)?,
        funds: vec![]
    });

    Ok(
        Response::new().add_message(response_msg)
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
    /// **NOTE**: Returns None if there is no data saved on the store.
    /// 
    pub fn remove(
        deps: &mut DepsMut
    ) -> Result<Option<Self>, ContractError> {

        let params = REPLY_CALLDATA_PARAMS.may_load(deps.storage)?;

        if params.is_some() {
            REPLY_CALLDATA_PARAMS.remove(deps.storage);
        }

        Ok(params)
    }
}


/// Handle the replies for 'common' messages (i.e. messages that are generated by the common
/// interface code/messages that use the common 'reply ids').
/// 
/// **NOTE**: Returns `None` for non-common reply ids.
/// 
/// # Arguments:
/// * `reply` - The message reply.
/// 
pub fn handle_reply(
    mut deps: DepsMut,
    env: Env,
    reply: Reply
) -> Result<Option<InterfaceResponse>, ContractError> {

    match reply.id {

        SET_ACK_REPLY_ID => match reply.result {
            SubMsgResult::Ok(_) => {
                // Set the custom 'success-ack' for successful executions.
                Ok(Some(Response::new().set_data(ack_success())))
            },
            SubMsgResult::Err(_) => {
                // Set the custom 'failed-ack' for unsuccessful executions.
                Ok(Some(Response::new().set_data(ack_fail())))
            }
        },

        RUN_CALLDATA_REPLY_ID => match reply.result {
            SubMsgResult::Ok(response) => {
                // Check if there is calldata to execute
                match CatalystCalldata::remove(&mut deps)? {
                    Some(calldata) => {
                        handle_calldata_on_reply(response, calldata).map(Some)
                    },
                    None => unreachable!("'RUN_CALLDATA_REPLY_ID' set without calldata saved.")
                }
            },
            SubMsgResult::Err(_) => {
                // The vault 'ReceiveAsset'/'ReceiveLiquidity' invocation is **always** the first
                // message. If it errors, a non-error response may be returned (i.e. an ack with
                // information on the error), as it is guaranteed that no prior submessage has 
                // committed any state to the store.
                Ok(Some(Response::new().set_data(ack_fail())))
            }
        },

        UNDERWRITE_REPLY_ID => match reply.result {
            SubMsgResult::Ok(response) => {
                handle_underwrite_reply(deps, env, response).map(Some)
                
            },
            SubMsgResult::Err(_) => {
                unreachable!(
                    "Underwrite reply should never be an error (ReplyOn::Success set)."
                )
            }
        },

        _ => Ok(None)
    }

}


/// Handle the calldata execution after the execution of a swap.
/// 
/// # Arguments:
/// * `swap_return` - The swap return.
/// 
pub fn handle_calldata_on_reply(
    response: SubMsgResponse,
    calldata: CatalystCalldata
) -> Result<InterfaceResponse, ContractError> {

    // Build the 'onCatalystCall' message using the swap return.

    let response_data = response.data.ok_or_else(|| {
        StdError::GenericErr { msg: "No data in the vault's `ReceiveAsset`/`ReceiveLiquidity` response.".to_string() }
    })?;
    let parsed_response_bytes = parse_execute_response_data(&response_data)
        .map_err(|err| {ContractError::Std(StdError::generic_err(err.to_string()))})?
        .data
        .ok_or(StdError::generic_err("No data in the vault's `ReceiveAsset`/`ReceiveLiquidity` response."))?;
    let swap_return: Uint128 = from_binary(&parsed_response_bytes)?;

    let calldata_message = create_on_catalyst_call_msg(
        calldata.target.to_string(),
        swap_return,
        calldata.bytes
    )?;

    // ! ONLY reply on success: DO NOT return an ack-fail, rather error the entire tx
    // ! See the `Receive Handlers` section above for more information.
    let submessage = SubMsg::reply_on_success(
        calldata_message,
        SET_ACK_REPLY_ID
    );

    Ok(
        Response::new()
            .add_submessage(submessage)
    )

}



// Underwriting
// ************************************************************************************************

/// Record of an underwrite event. Used to finish the underwriting logic upon reception of the
/// underwritten swap or expiry of the underwrite.
#[cw_serde]
pub struct UnderwriteEvent {
    pub amount: Uint128,
    pub underwriter: Addr
}


/// Expiry of an underwrite event. This is separate from the 'UnderwriteEvent' class, as the
/// 'UnderwriteExpiry' annotations are never removed from the storage. This is done to prevent
/// two underwrites targetting the same underwrite id on the same (or adjecent) blocks from
/// affecting each other. 
#[cw_serde]
pub struct UnderwriteExpiry {
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


impl UnderwriteExpiry {

    /// Save the underwrite expiry to the store under the given identifier.
    /// 
    /// **NOTE**: This call will always be successful, even if there is data already saved on the
    /// store under the given identifier.
    /// 
    /// # Arguments:
    /// * `identifier` - The underwrite identifier under which to save the event.
    /// 
    pub fn update(
        &self,
        deps: &mut DepsMut,
        identifier: Binary
    ) -> Result<(), ContractError> {

        UNDERWRITE_EXPIRIES.save(deps.storage, identifier.0, self)
            .map_err(|err| err.into())

    }

    /// Get the underwrite expiry from the store of the given identifier.
    /// 
    /// **NOTE**: Will return `0` if there is no data saved on the store with the given
    /// identifier.
    /// 
    /// # Arguments:
    /// * `identifier` - The underwrite identifier of which to get the expiry.
    /// 
    pub fn get(
        deps: &Deps,
        identifier: Binary
    ) -> Result<Self, ContractError> {

        let expiry = UNDERWRITE_EXPIRIES
            .may_load(deps.storage, identifier.0)?
            .unwrap_or(UnderwriteExpiry { expiry: Uint64::zero() });

        Ok(expiry)

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


/// Underwrite an asset swap.
/// 
/// **NOTE**: All the arguments passed to this function must **exactly match** those of the
/// desired swap to be underwritten.
/// 
/// **NOTE**: This method does not take into account any source vault parameters.
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
pub fn underwrite(
    deps: &mut DepsMut,
    env: &Env,
    info: &MessageInfo,
    to_vault: String,
    to_asset_ref: String,
    u: U256,
    min_out: Uint128,
    to_account: String,
    underwrite_incentive_x16: u16,
    calldata: Binary
) -> Result<InterfaceResponse, ContractError> {

    let identifier = get_underwrite_identifier(
        &to_vault,
        &to_asset_ref,
        &u,
        &min_out,
        &to_account,
        underwrite_incentive_x16,
        &calldata
    );

    // Check if another underwriter has already fulfilled the underwrite, and update the expiry to
    // the current block.
    let expiry = UnderwriteExpiry::get(&deps.as_ref(), identifier.clone())?.expiry;
    if (expiry + UNDERWRITE_BUFFER_BLOCKS).u64() >= env.block.height {
        return Err(ContractError::SwapRecentlyUnderwritten{});
    }

    // Parse the calldata now to avoid executing all the underwrite logic should the calldata be
    // wrongly formatted.
    let parsed_calldata = parse_calldata(deps.as_ref(), calldata)?;

    // Save the underwrite parameters to the store so that they can be recovered on the 'reply'
    // handler to finish the 'underwrite' logic.
    let underwrite_params = UnderwriteParams {
        identifier: identifier.clone(),
        underwriter: info.sender.clone(),
        to_vault: to_vault.clone(),
        to_asset_ref: to_asset_ref.clone(),
        to_account,
        underwrite_incentive_x16,
        calldata: parsed_calldata,
        funds: info.funds.clone()
    };
    underwrite_params.save(deps)?;

    // The swap `min_out` must be increased to take into account the underwriter's incentive
    let min_out = min_out
        .checked_mul(Uint128::new(2u128.pow(16)))?
        .div(Uint128::new(2u128.pow(16).wrapping_sub(underwrite_incentive_x16 as u128)));   //'wrapping_sub' safe as `underwrite_incentive_x16` < 2**16
    
    // Invoke the vault
    let underwrite_message = WasmMsg::Execute {
        contract_addr: to_vault,
        msg: to_binary(&VaultExecuteMsg::<()>::UnderwriteAsset {
            identifier,
            asset_ref: to_asset_ref,
            u,
            min_out
        })?,
        funds: vec![]
    };

    Ok(Response::new()
        .add_submessage(
            SubMsg {
                id: UNDERWRITE_REPLY_ID,
                msg: CosmosMsg::Wasm(underwrite_message),
                gas_limit: None,
                reply_on: ReplyOn::Success,
            }
        )
    )
}


/// Check the existance of a connection between the destination and the source vault, and perform
/// an asset underwrite.
/// 
/// **NOTE**: All the arguments passed to this function must **exactly match** those of the
/// desired swap to be underwritten.
/// 
/// # Arguments: 
/// * `channel_id` - The incoming message channel identifier.
/// * `from_vault` - The source vault on the source chain.
/// * `to_vault` - The target vault.
/// * `to_asset_ref` - The destination asset.
/// * `u` - The underwritten units.
/// * `min_out` - The mininum `to_asset_ref` output amount to get on the target vault.
/// * `to_account` - The recipient of the swap.
/// * `underwrite_incentive_x16` - The underwriting incentive.
/// * `calldata` - The swap calldata.
/// 
pub fn underwrite_and_check_connection(
    deps: &mut DepsMut,
    env: &Env,
    info: &MessageInfo,
    channel_id: String,
    from_vault: Binary,
    to_vault: String,
    to_asset_ref: String,
    u: U256,
    min_out: Uint128,
    to_account: String,
    underwrite_incentive_x16: u16,
    calldata: Binary
) -> Result<InterfaceResponse, ContractError> {

    let is_source_vault_connected = deps.querier.query_wasm_smart::<VaultConnectionStateResponse>(
        to_vault.clone(),
        &CommonQueryMsg::VaultConnectionState {
            channel_id: channel_id.clone(),
            vault: from_vault.clone()
        }
    )?.state;

    if !is_source_vault_connected {
        return Err(ContractError::VaultNotConnected { channel_id, vault: from_vault })
    }
    
    underwrite(
        deps,
        env,
        info,
        to_vault,
        to_asset_ref,
        u,
        min_out,
        to_account,
        underwrite_incentive_x16,
        calldata
    )
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
) -> Result<InterfaceResponse, ContractError> {

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
    let parsed_response_bytes = parse_execute_response_data(&response_data)
        .map_err(|err| {ContractError::Std(StdError::generic_err(err.to_string()))})?
        .data
        .ok_or(StdError::generic_err("No data in the vault's `UnderwriteAsset` response."))?;
    let swap_return: Uint128 = from_binary(&parsed_response_bytes)?;
        
    let expiry = Uint64::new(env.block.height) + get_max_underwrite_duration(&deps.as_ref())?;

    let underwrite_event = UnderwriteEvent {
        amount: swap_return,
        underwriter: underwriter.clone()
    };
    underwrite_event.save(&mut deps, identifier.clone())?;

    let underwrite_expiry = UnderwriteExpiry {
        expiry
    };
    underwrite_expiry.update(&mut deps, identifier.clone())?;


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


/// Expire an underwrite and free the escrowed assets.
/// 
/// **NOTE**: The underwriter may expire the underwrite at any time. Any other account must wait
/// until after the expiry block.
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
pub fn expire_underwrite(
    deps: &mut DepsMut,
    env: &Env,
    info: &MessageInfo,
    to_vault: String,
    to_asset_ref: String,
    u: U256,
    min_out: Uint128,
    to_account: String,
    underwrite_incentive_x16: u16,
    calldata: Binary
) -> Result<InterfaceResponse, ContractError> {

    let identifier = get_underwrite_identifier(
        &to_vault,
        &to_asset_ref,
        &u,
        &min_out,
        &to_account,
        underwrite_incentive_x16,
        &calldata
    );

    // ! Remove the underwrite event to prevent 'expire' being called multiple times
    let underwrite_event = UnderwriteEvent::remove(deps, identifier.clone())?
        .ok_or(ContractError::UnderwriteDoesNotExist{ id: identifier.clone() })?;

    // Verify that the underwrite has expired. Note that the underwriter may 'expire' the
    // underwrite at any time.
    if underwrite_event.underwriter != info.sender {
        let current_block = env.block.height;
        let expiry_block = UnderwriteExpiry::get(&deps.as_ref(), identifier.clone())?.expiry.u64();
        if current_block < expiry_block {
            return Err(ContractError::UnderwriteNotExpired{
                // 'wrapping_sub' safe, 'current_block < expiry_block' checked above 
                blocks_remaining: expiry_block.wrapping_sub(current_block).into()
            })
        }
    }

    // Clear the underwrite expiry (do not set it to the current block, as the 'double' underwrite
    // protection is not needed since the underwrite did not happen.)
    UnderwriteExpiry{ expiry: Uint64::zero() }.update(deps, identifier.clone())?;

    // Build the message to invoke the vault's `DeleteUnderwriteAsset`.
    let underwrite_amount = underwrite_event.amount;
    let delete_underwrite_msg = WasmMsg::Execute {
        contract_addr: to_vault.clone(),
        msg: to_binary(&VaultExecuteMsg::<()>::DeleteUnderwriteAsset {
            identifier: identifier.clone(),
            asset_ref: to_asset_ref.clone(),
            u,
            escrow_amount: underwrite_amount
        })?,
        funds: vec![]
    };

    // Build the messages to transfer the escrowed assets.
    //   -> refund = collateral + incentive
    let underwrite_incentive_x16 = Uint128::new(underwrite_incentive_x16 as u128);
    let underwrite_incentive = (underwrite_amount.checked_mul(underwrite_incentive_x16)?) >> 16;
    let underwrite_collateral = underwrite_amount
        .checked_mul(UNDERWRITING_COLLATERAL)?
        .div(UNDERWRITING_COLLATERAL_BASE);
    let refund_amount = underwrite_incentive
        .wrapping_add(underwrite_collateral);  // 'wrapping_add` safe: a larger computation has already
                                                // been done on the initial 'underwrite' call.

    // Compute the share of the refund amount that goes to the caller and to the vault
    // NOTE: Use U256 to make sure the following calculation never overflowS
    let expire_reward = U256::from(refund_amount)
        .wrapping_mul(U256::from_uint128(UNDERWRITING_EXPIRE_REWARD))   // 'wrapping_mul' safe: U256.max > Uint128.max * Uint128.max
        .div(U256::from_uint128(UNDERWRITING_EXPIRE_REWARD_BASE))
        .as_uint128();  // Casting safe, as UNDERWRITING_EXPIRE_REWARD < UNDERWRITING_EXPIRE_REWARD_BASE

    let vault_reward = refund_amount.wrapping_sub(expire_reward);   // 'wrapping_sub' safe: expire_reward < refund_amount

    let asset = deps.querier.query_wasm_smart::<AssetResponse<Asset>>(
        to_vault.clone(),
        &CommonQueryMsg::Asset { asset_ref: to_asset_ref }
    )?.asset;

    let expire_reward_msg = asset.send_asset(
        env,
        expire_reward,
        info.sender.to_string()
    )?;

    let vault_reward_transfer_msg = asset.send_asset(
        env,
        vault_reward,
        to_vault
    )?;

    // Build the response
    let mut response = InterfaceResponse::new()
        .add_message(delete_underwrite_msg);

    if let Some(msg) = expire_reward_msg {
        response = response.add_message(msg.into_cosmos_vault_msg());
    }

    if let Some(msg) = vault_reward_transfer_msg {
        response = response.add_message(msg.into_cosmos_vault_msg());
    }

    Ok(
        response
            .add_event(
                expire_underwrite_event(
                    identifier,
                    info.sender.to_string(),
                    expire_reward
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
) -> Result<Option<InterfaceResponse>, ContractError> {

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
        underwriter
    } = underwrite_event.unwrap();  // Unwrap safe: if the event is `None`, the function returns on
                                    // the previous statement.

    // Set the expiry to the current block to prevent new underwrites for the same underwrite id in
    // the next UNDERWRITE_BUFFER_BLOCKS blocks from happening (prevent accidental loss of funds).
    UnderwriteExpiry {
        expiry: Uint64::new(env.block.height)
    }.update(deps, identifier.clone())?;

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

    // Make sure the **last** message triggers the 'set ack' logic on **success**
    // ! ONLY reply on success: DO NOT return an ack-fail, rather error the entire tx.
    // ! See the 'Receive Handlers' section above for more information.
    let message_count = response.messages.len();
    let last_sub_msg = response.messages
        .get_mut(message_count-1)
        .unwrap();  // Unwrap safe, as there is always at least 1 message
    last_sub_msg.reply_on = ReplyOn::Success;
    last_sub_msg.id = SET_ACK_REPLY_ID;

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
/// ! **IMPORTANT**: This function **DOES NOT** check the authority of the sender of the transaction.
/// 
/// # Arguments:
/// * `new_max_duration` - The new desired maximum underwriting duration.
/// * `min_duration_allowed` - The minimum allowed underwriting duration. If `None`, defaults to a hardcoded constant.
/// * `max_duration_allowed` - The maximum allowed underwriting duration. If `None`, defaults to a hardcoded constant.
/// 
pub fn set_max_underwriting_duration_unchecked(
    deps: &mut DepsMut,
    new_max_duration: Uint64,
    min_duration_allowed: Option<Uint64>,
    max_duration_allowed: Option<Uint64>
) -> Result<Event, ContractError> {

    let min_duration_allowed = min_duration_allowed.unwrap_or(
        DEFAULT_MIN_UNDERWRITE_DURATION_ALLOWED_BLOCKS
    );
    if new_max_duration < min_duration_allowed {
        return Err(ContractError::MaxUnderwriteDurationTooShort {
            set_duration: new_max_duration,
            min_duration: min_duration_allowed,
        })
    }

    let max_duration_allowed = max_duration_allowed.unwrap_or(
        DEFAULT_MAX_UNDERWRITE_DURATION_ALLOWED_BLOCKS
    );
    if new_max_duration > max_duration_allowed {
        return Err(ContractError::MaxUnderwriteDurationTooLong {
            set_duration: new_max_duration,
            max_duration: max_duration_allowed,
        })
    }

    MAX_UNDERWRITE_DURATION_BLOCKS.save(deps.storage, &new_max_duration)?;

    Ok(
        set_max_underwrite_duration_event(new_max_duration)
    )

}


/// Set the maximum underwriting duration (only applies to new underwrite orders).
/// 
/// **NOTE**: This function checks that the sender of the transaction is the current interface owner.
/// 
/// # Arguments:
/// * `new_max_duration` - The new desired maximum underwriting duration.
/// * `min_duration_allowed` - The minimum allowed underwriting duration. If `None`, defaults to a hardcoded constant.
/// * `max_duration_allowed` - The maximum allowed underwriting duration. If `None`, defaults to a hardcoded constant.
/// 
pub fn set_max_underwriting_duration(
    deps: &mut DepsMut,
    info: &MessageInfo,
    new_max_duration: Uint64,
    min_duration_allowed: Option<Uint64>,
    max_duration_allowed: Option<Uint64>
) -> Result<InterfaceResponse, ContractError> {

    only_owner(deps.as_ref(), info)?;

    let event = set_max_underwriting_duration_unchecked(
        deps,
        new_max_duration,
        min_duration_allowed,
        max_duration_allowed
    )?;

    Ok(
        Response::new()
            .add_event(event)
    )
}


/// Get the maximum underwriting duration.
pub fn get_max_underwrite_duration(
    deps: &Deps
) -> Result<Uint64, ContractError> {

    MAX_UNDERWRITE_DURATION_BLOCKS
        .load(deps.storage)
        .map_err(|err| err.into())

}



/// Wrap multiple submessages within a single submessage.
/// 
/// **NOTE**: This method can only be invoked by the interface itself.
/// 
/// # Arguments:
/// *sub_msgs* - The submessages to wrap into a single submessage.
/// 
pub fn wrap_sub_msgs(
    info: &MessageInfo,
    env: &Env,
    sub_msgs: Vec<SubMsg<CustomMsg>>
) -> Result<InterfaceResponse, ContractError> {

    // ! Only the interface itself may invoke this function
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized {});
    }

    Ok(
        InterfaceResponse::new()
            .add_submessages(sub_msgs)
    )
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



// Queries
// ************************************************************************************************

/// Query the identifier of the provided underwrite parameters.
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
pub fn query_underwrite_identifier(
    to_vault: String,
    to_asset_ref: String,
    u: U256,
    min_out: Uint128,
    to_account: String,
    underwrite_incentive_x16: u16,
    calldata: Binary
) -> StdResult<UnderwriteIdentifierResponse> {
    Ok(
        UnderwriteIdentifierResponse {
            identifier: get_underwrite_identifier(
                &to_vault,
                &to_asset_ref,
                &u,
                &min_out,
                &to_account,
                underwrite_incentive_x16,
                &calldata
            )
        }
    )
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
pub fn update_owner<T>(
    deps: DepsMut,
    info: MessageInfo,
    account: String
) -> Result<Response<T>, ContractError> {

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




// Tests
// ************************************************************************************************
#[cfg(test)]
mod test_catalyst_interface_common {

    use std::marker::PhantomData;

    use catalyst_types::{U256, u256};
    use catalyst_vault_common::{bindings::{CustomMsg, Asset}, msg::{ExecuteMsg as VaultExecuteMsg, CommonQueryMsg, AssetResponse}};
    use cosmwasm_std::{Uint128, Binary, testing::{mock_info, mock_dependencies, mock_env, MockStorage, MockApi, MockQuerier}, SubMsg, to_binary, OwnedDeps, SystemResult, ContractResult, from_binary, Empty, Reply, SubMsgResult, SubMsgResponse};

    use crate::{catalyst_payload::CatalystEncodedAddress, ContractError, state::{encode_send_cross_chain_asset, encode_send_cross_chain_liquidity, handle_receive_liquidity, SET_ACK_REPLY_ID, handle_send_asset_response, handle_send_liquidity_response, handle_receive_asset, handle_reply}};




    const TEST_CHANNEL_ID           : &str      = "mock-channel-1";
    const TEST_FROM_VAULT           : &str      = "from_vault";
    const TEST_TO_VAULT             : &str      = "to_vault";
    const TEST_TO_ACCOUNT           : &str      = "to_account";
    const TEST_TO_ASSET_INDEX       : u8        = 1;
    const TEST_UNITS                : U256      = u256!("78456988731590487483448276103933454935747871349630657124267302091643025406701");
    const TEST_MIN_OUT              : Uint128   = Uint128::new(323476719582585693194107115743132847255u128);
    const TEST_MIN_VAULT_TOKENS     : Uint128   = Uint128::new(323476719582585693194107115743132847255u128);
    const TEST_MIN_REFERENCE_ASSET  : Uint128   = Uint128::new(1385371954613879816514345798135479u128);
    const TEST_FROM_AMOUNT          : Uint128   = Uint128::new(4920222095670429824873974121747892731u128);
    const TEST_FROM_AMOUNT_U256     : U256      = u256!("67845877620589376372337165092822343824636760238529546013156291980532914395690");
    const TEST_FROM_ASSET           : &str      = "from_asset";
    const TEST_UNDERWRITE_INCENTIVE : u16       = 1001;
    const TEST_FROM_BLOCK_NUMBER    : u32       = 1356;
    const TEST_CALLDATA             : Binary    = Binary(vec![]);    //TODO




    // Send asset helpers

    fn mock_vault_receive_asset_msg() -> VaultExecuteMsg<(), CustomMsg> {
        VaultExecuteMsg::ReceiveAsset {
            channel_id: TEST_CHANNEL_ID.to_string(),
            from_vault: CatalystEncodedAddress::try_encode(TEST_FROM_VAULT.as_bytes()).unwrap().to_binary(),
            to_asset_index: TEST_TO_ASSET_INDEX,
            to_account: TEST_TO_ACCOUNT.to_string(),
            u: TEST_UNITS,
            min_out: TEST_MIN_OUT,
            from_asset: CatalystEncodedAddress::try_encode(TEST_FROM_ASSET.as_bytes()).unwrap().to_binary(),
            from_amount: TEST_FROM_AMOUNT_U256,
            from_block_number_mod: TEST_FROM_BLOCK_NUMBER
        }
    }

    fn mock_vault_send_asset_success_msg() -> VaultExecuteMsg<(), CustomMsg> {
        VaultExecuteMsg::OnSendAssetSuccess {
            channel_id: TEST_CHANNEL_ID.to_string(),
            to_account: CatalystEncodedAddress::try_encode(TEST_TO_ACCOUNT.as_bytes()).unwrap().to_binary(),
            u: TEST_UNITS,
            escrow_amount: TEST_FROM_AMOUNT,
            asset_ref: TEST_FROM_ASSET.to_string(),
            block_number_mod: TEST_FROM_BLOCK_NUMBER
        }
    }

    fn mock_vault_send_asset_failure_msg() -> VaultExecuteMsg<(), CustomMsg> {
        VaultExecuteMsg::OnSendAssetFailure {
            channel_id: TEST_CHANNEL_ID.to_string(),
            to_account: CatalystEncodedAddress::try_encode(TEST_TO_ACCOUNT.as_bytes()).unwrap().to_binary(),
            u: TEST_UNITS,
            escrow_amount: TEST_FROM_AMOUNT,
            asset_ref: TEST_FROM_ASSET.to_string(),
            block_number_mod: TEST_FROM_BLOCK_NUMBER
        }
    }



    // Send liquidity helpers

    fn mock_vault_receive_liquidity_msg() -> VaultExecuteMsg<()> {
        VaultExecuteMsg::ReceiveLiquidity {
            channel_id: TEST_CHANNEL_ID.to_string(),
            from_vault: CatalystEncodedAddress::try_encode(TEST_FROM_VAULT.as_bytes()).unwrap().to_binary(),
            to_account: TEST_TO_ACCOUNT.to_string(),
            u: TEST_UNITS,
            min_vault_tokens: TEST_MIN_VAULT_TOKENS,
            min_reference_asset: TEST_MIN_REFERENCE_ASSET,
            from_amount: TEST_FROM_AMOUNT_U256,
            from_block_number_mod: TEST_FROM_BLOCK_NUMBER
        }
    }

    fn mock_vault_send_liquidity_success_msg() -> VaultExecuteMsg<()> {
        VaultExecuteMsg::OnSendLiquiditySuccess {
            channel_id: TEST_CHANNEL_ID.to_string(),
            to_account: CatalystEncodedAddress::try_encode(TEST_TO_ACCOUNT.as_bytes()).unwrap().to_binary(),
            u: TEST_UNITS,
            escrow_amount: TEST_FROM_AMOUNT,
            block_number_mod: TEST_FROM_BLOCK_NUMBER
        }
    }

    fn mock_vault_send_liquidity_failure_msg() -> VaultExecuteMsg<()> {
        VaultExecuteMsg::OnSendLiquidityFailure {
            channel_id: TEST_CHANNEL_ID.to_string(),
            to_account: CatalystEncodedAddress::try_encode(TEST_TO_ACCOUNT.as_bytes()).unwrap().to_binary(),
            u: TEST_UNITS,
            escrow_amount: TEST_FROM_AMOUNT,
            block_number_mod: TEST_FROM_BLOCK_NUMBER
        }
    }


    
    // Custom Deps Helpers
    // Some of the interface methods interact with the vaults. The following helpers implement
    // mocks for these interactions.

    fn mock_vault_assets() -> Vec<Asset> {

        #[cfg(feature="asset_native")]
        let assets = vec![
            Asset { denom: "asset_a".to_string(), alias: "a".to_string() },
            Asset { denom: "asset_b".to_string(), alias: "b".to_string() },
            Asset { denom: "asset_c".to_string(), alias: "c".to_string() }
        ];

        #[cfg(feature="asset_cw20")]
        let assets = vec![
            Asset ("asset_a".to_string()),
            Asset ("asset_b".to_string()),
            Asset ("asset_c".to_string())
        ];

        assets
    }

    // Create mock dependencies with the vault queries that are required by the interface.
    fn mock_deps_with_vault_queries() -> OwnedDeps<MockStorage, MockApi, MockQuerier, Empty> {

        let mut querier = MockQuerier::default();
        querier.update_wasm(|request| -> SystemResult<ContractResult<Binary>> {
            match request {
                cosmwasm_std::WasmQuery::Smart {
                    contract_addr,
                    msg
                } => {

                    if contract_addr != TEST_TO_VAULT {
                        unimplemented!("Mock query not implemented for contract '{:?}'", contract_addr)
                    }
                    
                    match from_binary::<CommonQueryMsg>(msg).unwrap() {

                        CommonQueryMsg::AssetByIndex { asset_index } => {
                            let response = AssetResponse {
                                asset: mock_vault_assets()[asset_index as usize].clone()
                            };
                            SystemResult::Ok(ContractResult::Ok(to_binary(&response).unwrap()))
                        },
                        query => unimplemented!("Mock query not implemented: {:?}", query)
                    }
                },
                _ => unimplemented!(),
            }
        });

        OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier,
            custom_query_type: PhantomData,
        }
    }



    // Send Asset Tests
    // ********************************************************************************************

    #[test]
    fn test_send_asset_encoding() {

        let info = mock_info(TEST_FROM_VAULT, &[]);



        // Tested action: send asset encoding
        let encoded_payload = encode_send_cross_chain_asset(
            info,
            CatalystEncodedAddress::try_encode(TEST_TO_VAULT.as_bytes()).unwrap().to_binary(),
            CatalystEncodedAddress::try_encode(TEST_TO_ACCOUNT.as_bytes()).unwrap().to_binary(),
            TEST_TO_ASSET_INDEX,
            TEST_UNITS,
            U256::from_uint128(TEST_MIN_OUT),
            TEST_FROM_AMOUNT,
            TEST_FROM_ASSET.to_string(),
            TEST_UNDERWRITE_INCENTIVE,
            TEST_FROM_BLOCK_NUMBER,
            TEST_CALLDATA
        ).unwrap();



        let expected_payload = "AAoAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABmcm9tX3ZhdWx0CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAdG9fdmF1bHQKAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAdG9fYWNjb3VudK11FPutIXAiE147GgKdkGKPHGCQYwjyY+0HHEN78LrtAQAAAAAAAAAAAAAAAAAAAADzW1mdPtcyQ6KnbCoFMQyXAAAAAAAAAAAAAAAAAAAAAAOzma2zUZuQzXUnhx+qyfsKAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAZnJvbV9hc3NldAAABUwD6QAA";

        assert_eq!(
            encoded_payload,
            Binary::from_base64(expected_payload).unwrap()
        );

    }


    #[test]
    fn test_receive_asset() {

        // Have to use custom mock_deps, as the 'handle_receive_asset' will query the vault for the
        // asset that corresponds to `TEST_TO_ASSET_INDEX`.
        let mut deps = mock_deps_with_vault_queries();
        let env = mock_env();



        // Tested action: receive asset
        let response_result = handle_receive_asset(
            &mut deps.as_mut(),
            &env,
            TEST_CHANNEL_ID.to_string(),
            TEST_TO_VAULT.to_string(),
            TEST_TO_ASSET_INDEX,
            TEST_TO_ACCOUNT.to_string(),
            TEST_UNITS,
            U256::from_uint128(TEST_MIN_OUT),
            TEST_UNDERWRITE_INCENTIVE,
            CatalystEncodedAddress::try_encode(TEST_FROM_VAULT.as_bytes()).unwrap().to_binary(),
            TEST_FROM_AMOUNT_U256,
            CatalystEncodedAddress::try_encode(TEST_FROM_ASSET.as_bytes()).unwrap().to_binary(),
            TEST_FROM_BLOCK_NUMBER,
            TEST_CALLDATA
        );



        // Check the transaction passes
        let response = response_result.unwrap();
    
        // Check vault is invoked
        assert_eq!(response.messages.len(), 1);

        assert_eq!(
            response.messages[0],
            SubMsg {
                id: SET_ACK_REPLY_ID,
                msg: cosmwasm_std::WasmMsg::Execute {
                    contract_addr: TEST_TO_VAULT.to_string(),
                    msg: to_binary(&mock_vault_receive_asset_msg()).unwrap(),
                    funds: vec![]
                }.into(),
                reply_on: cosmwasm_std::ReplyOn::Always,
                gas_limit: None

            }
        )

    }


    #[test]
    fn test_receive_asset_invalid_min_out() {

        let mut deps = mock_dependencies();
        let env = mock_env();



        // Tested action: receive asset
        let response_result = handle_receive_asset(
            &mut deps.as_mut(),
            &env,
            TEST_CHANNEL_ID.to_string(),
            TEST_TO_VAULT.to_string(),
            TEST_TO_ASSET_INDEX,
            TEST_TO_ACCOUNT.to_string(),
            TEST_UNITS,
            U256::MAX,      // ! Specify a min_out that is larger than Uint128
            TEST_UNDERWRITE_INCENTIVE,
            CatalystEncodedAddress::try_encode(TEST_FROM_VAULT.as_bytes()).unwrap().to_binary(),
            TEST_FROM_AMOUNT_U256,
            CatalystEncodedAddress::try_encode(TEST_FROM_ASSET.as_bytes()).unwrap().to_binary(),
            TEST_FROM_BLOCK_NUMBER,
            TEST_CALLDATA
        );



        // Check the transaction errors
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::PayloadDecodingError {}
        ));

    }


    #[test]
    fn test_send_asset_ack() {


        // Tested action: send asset ack SUCCESSFUL
        let response_result = handle_send_asset_response(
            TEST_CHANNEL_ID.to_string(),
            CatalystEncodedAddress::try_encode(TEST_TO_ACCOUNT.as_bytes()).unwrap().to_binary(),
            TEST_UNITS,
            TEST_FROM_VAULT.to_string(),
            U256::from_uint128(TEST_FROM_AMOUNT),
            TEST_FROM_ASSET.to_string(),
            TEST_FROM_BLOCK_NUMBER,
            Some(Binary(vec![0u8])),         // ! Test for success
        );

        // Check the transaction passes
        let response = response_result.unwrap();
    
        // Check vault ack is invoked
        assert_eq!(response.messages.len(), 1);
        assert_eq!(
            response.messages[0],
            SubMsg::new(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: TEST_FROM_VAULT.to_string(),
                    msg: to_binary(&mock_vault_send_asset_success_msg()).unwrap(),
                    funds: vec![]
                }
            )
        );



        // Tested action: send asset ack UNSUCCESSFUL
        let response_result = handle_send_asset_response(
            TEST_CHANNEL_ID.to_string(),
            CatalystEncodedAddress::try_encode(TEST_TO_ACCOUNT.as_bytes()).unwrap().to_binary(),
            TEST_UNITS,
            TEST_FROM_VAULT.to_string(),
            U256::from_uint128(TEST_FROM_AMOUNT),
            TEST_FROM_ASSET.to_string(),
            TEST_FROM_BLOCK_NUMBER,
            Some(Binary(vec![1u8])),         // ! Test for failure
        );

        // Check the transaction passes
        let response = response_result.unwrap();
    
        // Check vault ack is invoked
        assert_eq!(response.messages.len(), 1);
        assert_eq!(
            response.messages[0],
            SubMsg::new(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: TEST_FROM_VAULT.to_string(),
                    msg: to_binary(&mock_vault_send_asset_failure_msg()).unwrap(),
                    funds: vec![]
                }
            )
        );



        // Tested action: send asset ack INVALID
        let response_result = handle_send_asset_response(
            TEST_CHANNEL_ID.to_string(),
            CatalystEncodedAddress::try_encode(TEST_TO_ACCOUNT.as_bytes()).unwrap().to_binary(),
            TEST_UNITS,
            TEST_FROM_VAULT.to_string(),
            U256::from_uint128(TEST_FROM_AMOUNT),
            TEST_FROM_ASSET.to_string(),
            TEST_FROM_BLOCK_NUMBER,
            Some(Binary(vec![9u8])),         // ! Some invalid response
        );

        // Check the transaction passes
        let response = response_result.unwrap();
    
        // Check vault ack is invoked
        assert_eq!(response.messages.len(), 1);
        assert_eq!(
            response.messages[0],
            SubMsg::new(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: TEST_FROM_VAULT.to_string(),
                    msg: to_binary(&mock_vault_send_asset_failure_msg()).unwrap(),    // Invalid responses are treated as failures.
                    funds: vec![]
                }
            )
        );

    }


    #[test]
    fn test_send_asset_timeout() {


        // Tested action: send asset timeout
        let response_result = handle_send_asset_response(
            TEST_CHANNEL_ID.to_string(),
            CatalystEncodedAddress::try_encode(TEST_TO_ACCOUNT.as_bytes()).unwrap().to_binary(),
            TEST_UNITS,
            TEST_FROM_VAULT.to_string(),
            U256::from_uint128(TEST_FROM_AMOUNT),
            TEST_FROM_ASSET.to_string(),
            TEST_FROM_BLOCK_NUMBER,
            None,         // ! No response (i.e. timeout)
        );


        // Check the transaction passes
        let response = response_result.unwrap();
    
        // Check vault timeout is invoked
        assert_eq!(response.messages.len(), 1);
        assert_eq!(
            response.messages[0],
            SubMsg::new(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: TEST_FROM_VAULT.to_string(),
                    msg: to_binary(&mock_vault_send_asset_failure_msg()).unwrap(),
                    funds: vec![]
                }
            )
        )

    }


    #[test]
    fn test_send_asset_ack_timeout_invalid_from_amount() {

        let invalid_from_amount = U256::from(Uint128::MAX) + U256::one();



        // Tested action: send asset ACK SUCCESSFUL with invalid packet (from_amount)
        let response_result = handle_send_asset_response(
            TEST_CHANNEL_ID.to_string(),
            CatalystEncodedAddress::try_encode(TEST_TO_ACCOUNT.as_bytes()).unwrap().to_binary(),
            TEST_UNITS,
            TEST_FROM_VAULT.to_string(),
            invalid_from_amount,        // ! Invalid from_amount
            TEST_FROM_ASSET.to_string(),
            TEST_FROM_BLOCK_NUMBER,
            Some(Binary(vec![0u8])),    // ! Test ack-success
        );

        // Check the transaction does not pass
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::PayloadDecodingError {}
        ));



        // Tested action: send asset ACK UNSUCCESSFUL with invalid packet
        let response_result = handle_send_asset_response(
            TEST_CHANNEL_ID.to_string(),
            CatalystEncodedAddress::try_encode(TEST_TO_ACCOUNT.as_bytes()).unwrap().to_binary(),
            TEST_UNITS,
            TEST_FROM_VAULT.to_string(),
            invalid_from_amount,        // ! Invalid from_amount
            TEST_FROM_ASSET.to_string(),
            TEST_FROM_BLOCK_NUMBER,
            Some(Binary(vec![1u8])),    // ! Test ack-failure
        );

        // Check the transaction does not pass
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::PayloadDecodingError {}
        ));



        // Tested action: send asset TIMEOUT with invalid packet
        let response_result = handle_send_asset_response(
            TEST_CHANNEL_ID.to_string(),
            CatalystEncodedAddress::try_encode(TEST_TO_ACCOUNT.as_bytes()).unwrap().to_binary(),
            TEST_UNITS,
            TEST_FROM_VAULT.to_string(),
            invalid_from_amount,        // ! Invalid from_amount
            TEST_FROM_ASSET.to_string(),
            TEST_FROM_BLOCK_NUMBER,
            None,                       // ! Test timeout
        );

        // Check the transaction does not pass
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::PayloadDecodingError {}
        ));

    }



    // Send Liquidity Tests
    // ********************************************************************************************

    #[test]
    fn test_send_liquidity_encoding() {

        let info = mock_info(TEST_FROM_VAULT, &[]);



        // Tested action: send liquidity encoding
        let encoded_payload = encode_send_cross_chain_liquidity(
            info,
            CatalystEncodedAddress::try_encode(TEST_TO_VAULT.as_bytes()).unwrap().to_binary(),
            CatalystEncodedAddress::try_encode(TEST_TO_ACCOUNT.as_bytes()).unwrap().to_binary(),
            TEST_UNITS,
            U256::from_uint128(TEST_MIN_VAULT_TOKENS),
            U256::from_uint128(TEST_MIN_REFERENCE_ASSET),
            TEST_FROM_AMOUNT,
            TEST_FROM_BLOCK_NUMBER,
            TEST_CALLDATA
        ).unwrap();



        let expected_payload = "AQoAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABmcm9tX3ZhdWx0CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAdG9fdmF1bHQKAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAdG9fYWNjb3VudK11FPutIXAiE147GgKdkGKPHGCQYwjyY+0HHEN78LrtAAAAAAAAAAAAAAAAAAAAAPNbWZ0+1zJDoqdsKgUxDJcAAAAAAAAAAAAAAAAAAAAAAABETdo2CALoQfNE9srutwAAAAAAAAAAAAAAAAAAAAADs5mts1GbkM11J4cfqsn7AAAFTAAA";

        assert_eq!(
            encoded_payload,
            Binary::from_base64(expected_payload).unwrap()
        );

    }


    #[test]
    fn test_receive_liquidity() {

        let mut deps = mock_dependencies();



        // Tested action: receive liquidity
        let response_result = handle_receive_liquidity(
            &mut deps.as_mut(),
            TEST_CHANNEL_ID.to_string(),
            TEST_TO_VAULT.to_string(),
            TEST_TO_ACCOUNT.to_string(),
            TEST_UNITS,
            U256::from_uint128(TEST_MIN_VAULT_TOKENS),
            U256::from_uint128(TEST_MIN_REFERENCE_ASSET),
            CatalystEncodedAddress::try_encode(TEST_FROM_VAULT.as_bytes()).unwrap().to_binary(),
            TEST_FROM_AMOUNT_U256,
            TEST_FROM_BLOCK_NUMBER,
            TEST_CALLDATA
        );



        // Check the transaction passes
        let response = response_result.unwrap();
    
        // Check vault is invoked
        assert_eq!(response.messages.len(), 1);

        assert_eq!(
            response.messages[0],
            SubMsg {
                id: SET_ACK_REPLY_ID,
                msg: cosmwasm_std::WasmMsg::Execute {
                    contract_addr: TEST_TO_VAULT.to_string(),
                    msg: to_binary(
                        &mock_vault_receive_liquidity_msg()
                    ).unwrap(),
                    funds: vec![]
                }.into(),
                reply_on: cosmwasm_std::ReplyOn::Always,
                gas_limit: None

            }
        )

    }


    #[test]
    fn test_receive_liquidity_invalid_min_vault_tokens() {

        let mut deps = mock_dependencies();



        // Tested action: receive liquidity
        let response_result = handle_receive_liquidity(
            &mut deps.as_mut(),
            TEST_CHANNEL_ID.to_string(),
            TEST_TO_VAULT.to_string(),
            TEST_TO_ACCOUNT.to_string(),
            TEST_UNITS,
            U256::MAX,      // ! Specify a min_vault_token that is larger than Uint128
            U256::from_uint128(TEST_MIN_REFERENCE_ASSET),
            CatalystEncodedAddress::try_encode(TEST_FROM_VAULT.as_bytes()).unwrap().to_binary(),
            TEST_FROM_AMOUNT_U256,
            TEST_FROM_BLOCK_NUMBER,
            TEST_CALLDATA
        );



        // Check the transaction errors
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::PayloadDecodingError {}
        ));

    }


    #[test]
    fn test_receive_liquidity_invalid_min_reference_asset() {

        let mut deps = mock_dependencies();



        // Tested action: receive liquidity
        let response_result = handle_receive_liquidity(
            &mut deps.as_mut(),
            TEST_CHANNEL_ID.to_string(),
            TEST_TO_VAULT.to_string(),
            TEST_TO_ACCOUNT.to_string(),
            TEST_UNITS,
            U256::from_uint128(TEST_MIN_VAULT_TOKENS),
            U256::MAX,      // ! Specify a min_vault_token that is larger than Uint128
            CatalystEncodedAddress::try_encode(TEST_FROM_VAULT.as_bytes()).unwrap().to_binary(),
            TEST_FROM_AMOUNT_U256,
            TEST_FROM_BLOCK_NUMBER,
            TEST_CALLDATA
        );



        // Check the transaction errors
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::PayloadDecodingError {}
        ));

    }


    #[test]
    fn test_send_liquidity_ack() {


        // Tested action: send liquidity ack SUCCESSFUL
        let response_result = handle_send_liquidity_response(
            TEST_CHANNEL_ID.to_string(),
            CatalystEncodedAddress::try_encode(TEST_TO_ACCOUNT.as_bytes()).unwrap().to_binary(),
            TEST_UNITS,
            TEST_FROM_VAULT.to_string(),
            U256::from_uint128(TEST_FROM_AMOUNT),
            TEST_FROM_BLOCK_NUMBER,
            Some(Binary(vec![0u8])),         // ! Test for success
        );

        // Check the transaction passes
        let response = response_result.unwrap();
    
        // Check vault ack is invoked
        assert_eq!(response.messages.len(), 1);
        assert_eq!(
            response.messages[0],
            SubMsg::new(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: TEST_FROM_VAULT.to_string(),
                    msg: to_binary(&mock_vault_send_liquidity_success_msg()).unwrap(),
                    funds: vec![]
                }
            )
        );



        // Tested action: send liquidity ack UNSUCCESSFUL
        let response_result = handle_send_liquidity_response(
            TEST_CHANNEL_ID.to_string(),
            CatalystEncodedAddress::try_encode(TEST_TO_ACCOUNT.as_bytes()).unwrap().to_binary(),
            TEST_UNITS,
            TEST_FROM_VAULT.to_string(),
            U256::from_uint128(TEST_FROM_AMOUNT),
            TEST_FROM_BLOCK_NUMBER,
            Some(Binary(vec![1u8])),         // ! Test for failure
        );

        // Check the transaction passes
        let response = response_result.unwrap();
    
        // Check vault ack is invoked
        assert_eq!(response.messages.len(), 1);
        assert_eq!(
            response.messages[0],
            SubMsg::new(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: TEST_FROM_VAULT.to_string(),
                    msg: to_binary(&mock_vault_send_liquidity_failure_msg()).unwrap(),
                    funds: vec![]
                }
            )
        );



        // Tested action: send liquidity ack INVALID
        let response_result = handle_send_liquidity_response(
            TEST_CHANNEL_ID.to_string(),
            CatalystEncodedAddress::try_encode(TEST_TO_ACCOUNT.as_bytes()).unwrap().to_binary(),
            TEST_UNITS,
            TEST_FROM_VAULT.to_string(),
            U256::from_uint128(TEST_FROM_AMOUNT),
            TEST_FROM_BLOCK_NUMBER,
            Some(Binary(vec![9u8])),         // ! Some invalid response
        );

        // Check the transaction passes
        let response = response_result.unwrap();
    
        // Check vault ack is invoked
        assert_eq!(response.messages.len(), 1);
        assert_eq!(
            response.messages[0],
            SubMsg::new(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: TEST_FROM_VAULT.to_string(),
                    msg: to_binary(&mock_vault_send_liquidity_failure_msg()).unwrap(),    // Invalid responses are treated as failures.
                    funds: vec![]
                }
            )
        );

    }


    #[test]
    fn test_send_liquidity_timeout() {


        // Tested action: send liquidity timeout
        let response_result = handle_send_liquidity_response(
            TEST_CHANNEL_ID.to_string(),
            CatalystEncodedAddress::try_encode(TEST_TO_ACCOUNT.as_bytes()).unwrap().to_binary(),
            TEST_UNITS,
            TEST_FROM_VAULT.to_string(),
            U256::from_uint128(TEST_FROM_AMOUNT),
            TEST_FROM_BLOCK_NUMBER,
            None,         // ! No response (i.e. timeout)
        );



        // Check the transaction passes
        let response = response_result.unwrap();
    
        // Check vault timeout is invoked
        assert_eq!(response.messages.len(), 1);
        assert_eq!(
            response.messages[0],
            SubMsg::new(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: TEST_FROM_VAULT.to_string(),
                    msg: to_binary(&mock_vault_send_liquidity_failure_msg()).unwrap(),
                    funds: vec![]
                }
            )
        )

    }


    #[test]
    fn test_send_liquidity_ack_timeout_invalid_from_amount() {

        let invalid_from_amount = U256::from(Uint128::MAX) + U256::one();



        // Tested action: send liquidity ACK SUCCESSFUL with invalid packet (from_amount)
        let response_result = handle_send_liquidity_response(
            TEST_CHANNEL_ID.to_string(),
            CatalystEncodedAddress::try_encode(TEST_TO_ACCOUNT.as_bytes()).unwrap().to_binary(),
            TEST_UNITS,
            TEST_FROM_VAULT.to_string(),
            invalid_from_amount,        // ! Invalid from_amount
            TEST_FROM_BLOCK_NUMBER,
            Some(Binary(vec![0u8])),    // ! Test ack-success
        );

        // Check the transaction does not pass
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::PayloadDecodingError {}
        ));



        // Tested action: send liquidity ACK UNSUCCESSFUL with invalid packet
        let response_result = handle_send_liquidity_response(
            TEST_CHANNEL_ID.to_string(),
            CatalystEncodedAddress::try_encode(TEST_TO_ACCOUNT.as_bytes()).unwrap().to_binary(),
            TEST_UNITS,
            TEST_FROM_VAULT.to_string(),
            invalid_from_amount,        // ! Invalid from_amount
            TEST_FROM_BLOCK_NUMBER,
            Some(Binary(vec![1u8])),    // ! Test ack-failure
        );

        // Check the transaction does not pass
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::PayloadDecodingError {}
        ));



        // Tested action: send liquidity TIMEOUT with invalid packet
        let response_result = handle_send_liquidity_response(
            TEST_CHANNEL_ID.to_string(),
            CatalystEncodedAddress::try_encode(TEST_TO_ACCOUNT.as_bytes()).unwrap().to_binary(),
            TEST_UNITS,
            TEST_FROM_VAULT.to_string(),
            invalid_from_amount,        // ! Invalid from_amount
            TEST_FROM_BLOCK_NUMBER,
            None,                       // ! Test timeout
        );

        // Check the transaction does not pass
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::PayloadDecodingError {}
        ));

    }



    // Common Tests
    // ********************************************************************************************

    #[test]
    fn test_reply_set_ack() {

        let mut deps = mock_dependencies();
        let env = mock_env();



        // Tested action 1: reply ok
        let result = handle_reply(
            deps.as_mut(),
            env.clone(),
            Reply {
                id: SET_ACK_REPLY_ID,
                result: SubMsgResult::Ok(
                    SubMsgResponse { events: vec![], data: None }       // SubMsgResponse contents do not matter
                )
            }
        ).unwrap(); // Make sure the call passes

        // Make sure the reply handler matches the `reply_id` (i.e. the return value is not None)
        let response = result.unwrap(); 

        // Check the response
        assert_eq!(response.messages.len(), 0);
        assert_eq!(
            response.data,
            Some(Binary(vec![0]))     // ! Verify the response 'data' field is a success ack.
        );



        // Tested action 2: reply error
        let result = handle_reply(
            deps.as_mut(),
            env,
            Reply {
                id: SET_ACK_REPLY_ID,
                result: SubMsgResult::Err("Some error".to_string())
            }
        ).unwrap(); // Make sure the call passes

        // Make sure the reply handler matches the `reply_id` (i.e. the return value is not None)
        let response = result.unwrap(); 

        // Check the response
        assert_eq!(response.messages.len(), 0);
        assert_eq!(
            response.data,
            Some(Binary(vec![1]))     // ! Verify the response 'data' field is a fail ack.
        );

    }
    

    #[test]
    fn test_reply_unknown_id() {

        let mut deps = mock_dependencies();
        let env = mock_env();



        // Tested action 1: reply ok with unknown id
        let result = handle_reply(
            deps.as_mut(),
            env.clone(),
            Reply {
                id: 90909,  // Some random id
                result: SubMsgResult::Ok(
                    SubMsgResponse { events: vec![], data: None }       // SubMsgResponse contents do not matter
                )
            }
        ).unwrap(); // Make sure the call passes

        // Make sure the reply handler returns None
        assert!(result.is_none());



        // Tested action 2: reply err with unknown id
        let result = handle_reply(
            deps.as_mut(),
            env.clone(),
            Reply {
                id: 90909,  // Some random id
                result: SubMsgResult::Err("Some error".to_string())
            }
        ).unwrap(); // Make sure the call passes

        // Make sure the reply handler returns None
        assert!(result.is_none())
    }

}