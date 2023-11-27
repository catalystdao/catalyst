use catalyst_interface_common::{bindings::InterfaceResponse, ContractError, state::only_owner, catalyst_payload::CatalystEncodedAddress};
use catalyst_types::Bytes32;
use cosmwasm_std::{Addr, DepsMut, Deps, Uint64, MessageInfo, StdResult, StdError, Binary, WasmMsg, to_json_binary, Uint128};
use cw_storage_plus::{Item, Map};
use generalised_incentives_common::{msg::{EstimateAddtionalCostResponse, QueryMsg as GIQueryMsg, ExecuteMsg as GIExecuteMsg}, bytes32::Bytes32 as GIBytes32, state::IncentiveDescription};

use crate::events::{set_min_gas_for_event, remote_implementation_set_event, set_min_ack_gas_price_event};



// Constants
// ************************************************************************************************

pub const MIN_GAS_FOR_ACK_ID : Bytes32 = Bytes32([0; 32]);




// State
// ************************************************************************************************

const GENERALISED_INCENTIVES: Item<Addr> = Item::new("catalyst-interface-gi-address");
const REMOTE_IMPLEMENTATIONS: Map<Bytes32, Vec<u8>> = Map::new("catalyst-interface-gi-implementations");
const MIN_GAS_FOR: Map<Bytes32, Uint64> = Map::new("catalyst-interface-gi-min-gas-for");
const MIN_ACK_GAS_PRICE: Item<Uint128> = Item::new("catalyst-interface-gi-min-ack-gas-price");




// Helpers
// ************************************************************************************************

pub fn connect_new_chain(
    mut deps: DepsMut,
    info: MessageInfo,
    channel_id: Bytes32,
    remote_interface: Binary,
    remote_gi: Binary
) -> Result<InterfaceResponse, ContractError> {
    
    only_owner(deps.as_ref(), &info)?;

    // Validate the `remote_interface` type
    let remote_interface = CatalystEncodedAddress::try_from(remote_interface)?;

    set_remote_interface(
        &mut deps,
        channel_id.clone(),
        remote_interface.clone()
    )?;

    let set_remote_gi_submsg = WasmMsg::Execute {
        contract_addr: get_generalised_incentives(&deps.as_ref())?.to_string(),
        msg: to_json_binary(&GIExecuteMsg::SetRemoteImplementation {
            remote_identifier: GIBytes32(channel_id.0),
            implementation: remote_gi.clone()
        })?,
        funds: vec![]
    };

    let event = remote_implementation_set_event(
        channel_id.clone(),
        remote_interface,
        remote_gi.clone()
    );
    
    Ok(
        InterfaceResponse::new()
            .add_message(set_remote_gi_submsg)
            .add_event(event)
    )

}


pub fn set_generalised_incentives(
    deps: &mut DepsMut,
    generalised_incentives: String
) -> Result<(), ContractError> {

    GENERALISED_INCENTIVES.save(
        deps.storage,
        &deps.api.addr_validate(&generalised_incentives)?
    )?;

    Ok(())
}

pub fn get_generalised_incentives(
    deps: &Deps,
) -> Result<Addr, ContractError> {

    let address = GENERALISED_INCENTIVES.load(deps.storage)?;

    Ok(address)
}

pub fn only_generalised_incentives(
    deps: &Deps,
    info: &MessageInfo
) -> Result<(), ContractError> {

    if info.sender != get_generalised_incentives(deps)? {
        Err(ContractError::Unauthorized {})
    }
    else {
        Ok(())
    }
}


pub fn set_min_gas_for(
    deps: DepsMut,
    info: MessageInfo,
    channel_id: Bytes32,
    min_gas: Uint64
) -> Result<InterfaceResponse, ContractError> {
    
    only_owner(deps.as_ref(), &info)?;

    MIN_GAS_FOR.save(
        deps.storage,
        channel_id.clone(),
        &min_gas
    )?;

    let event = set_min_gas_for_event(
        channel_id,
        min_gas
    );

    Ok(
        InterfaceResponse::new()
            .add_event(event)
    )
}

pub fn get_min_gas_for(
    deps: &Deps,
    channel_id: Bytes32
) -> Result<Uint64, ContractError> {

    MIN_GAS_FOR
        .may_load(deps.storage, channel_id.clone())?
        .ok_or_else(|| {
            let msg = format!(
                "Unable to load the minimum gas for the chain identifier {}",
                channel_id.to_base64()
            );
            StdError::generic_err(msg).into()
        })
}


pub fn set_min_ack_gas_price(
    deps: DepsMut,
    info: MessageInfo,
    min_gas_price: Uint128
) -> Result<InterfaceResponse, ContractError> {
    
    only_owner(deps.as_ref(), &info)?;

    MIN_ACK_GAS_PRICE.save(
        deps.storage,
        &min_gas_price
    )?;

    let event = set_min_ack_gas_price_event(
        min_gas_price
    );

    Ok(
        InterfaceResponse::new()
            .add_event(event)
    )
}

pub fn get_min_ack_gas_price(
    deps: &Deps
) -> Result<Uint128, ContractError> {

    MIN_ACK_GAS_PRICE
        .may_load(deps.storage)?
        .ok_or_else(|| {
            let msg = "Unable to load the minimum ack gas price";
            StdError::generic_err(msg).into()
        })
}


pub fn set_remote_interface(
    deps: &mut DepsMut,
    channel_id: Bytes32,
    remote_interface: CatalystEncodedAddress
) -> Result<(), ContractError> {

    let key = REMOTE_IMPLEMENTATIONS.key(channel_id);

    if key.may_load(deps.storage)?.is_some() {
        return Err(ContractError::Unauthorized {});
    }

    key.save(
        deps.storage,
        &remote_interface.to_vec()
    )?;

    Ok(())
}

pub fn get_remote_interface(
    deps: &Deps,
    channel_id: Bytes32
) -> Result<CatalystEncodedAddress, ContractError> {

    let implementation = REMOTE_IMPLEMENTATIONS.load(
        deps.storage,
        channel_id
    ).map_err(|_| ContractError::NoSourceInterfaceSet {})?;

    Ok(implementation.try_into()?)
}


pub fn check_route_description(
    deps: &Deps,
    channel_id: Bytes32,
    to_account: Binary,
    to_vault: Binary,
    incentive: &IncentiveDescription
) -> Result<(), ContractError> {

    // Check the specified gas configuration

    let min_gas_for_delivery = get_min_gas_for(deps, channel_id)?;
    if incentive.max_gas_delivery < min_gas_for_delivery {
        return Err(ContractError::NotEnoughIncentives {
            description: "delivery gas".to_string(),
            minimum: min_gas_for_delivery.into(),
            actual: incentive.max_gas_delivery.into()
        });
    }

    let min_gas_for_ack = get_min_gas_for(deps, MIN_GAS_FOR_ACK_ID)?;
    if incentive.max_gas_ack < min_gas_for_ack {
        return Err(ContractError::NotEnoughIncentives {
            description: "ack gas".to_string(),
            minimum: min_gas_for_ack.into(),
            actual: incentive.max_gas_ack.into()
        });
    }

    let min_gas_price_for_ack = get_min_ack_gas_price(deps)?;
    if incentive.price_of_ack_gas < min_gas_price_for_ack {
        return Err(ContractError::NotEnoughIncentives {
            description: "price of ack gas".to_string(),
            minimum: min_gas_price_for_ack.into(),
            actual: incentive.price_of_ack_gas
        });
    }

    CatalystEncodedAddress::try_from(to_account)?;
    CatalystEncodedAddress::try_from(to_vault)?;

    Ok(())
}


pub fn query_estimate_additional_cost(
    deps: &Deps
) -> StdResult<EstimateAddtionalCostResponse> {

    deps.querier.query_wasm_smart(
        get_generalised_incentives(deps).map_err(Into::<StdError>::into)?,
        &GIQueryMsg::EstimateAdditionalCost {}
    )
}