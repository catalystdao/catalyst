use std::ops::{Div, Sub};

use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, Uint128, DepsMut, Env, Response, Event, MessageInfo, Deps, StdResult, CosmosMsg, to_binary, Timestamp};
use cw20::Cw20ExecuteMsg;
use cw_storage_plus::{Map, Item};
use cw20_base::{state::{MinterData, TokenInfo, TOKEN_INFO}, contract::execute_mint};
use ethnum::{U256, uint};
use fixed_point_math_lib::fixed_point_math::mul_wad_down;
use sha3::{Digest, Keccak256};

use crate::{ContractError, msg::{ChainInterfaceResponse, SetupMasterResponse, ReadyResponse, OnlyLocalResponse, AssetsResponse, WeightsResponse, PoolFeeResponse, GovernanceFeeShareResponse, FeeAdministratorResponse, TotalEscrowedAssetResponse, TotalEscrowedLiquidityResponse, AssetEscrowResponse, LiquidityEscrowResponse, PoolConnectionStateResponse}};


// Pool Constants
pub const MAX_ASSETS: usize = 3;

pub const DECIMALS: u8 = 18;
pub const INITIAL_MINT_AMOUNT: Uint128 = Uint128::new(1000000000000000000u128); // 1e18

pub const MAX_POOL_FEE_SHARE       : u64 = 1000000000000000000u64;              // 100%
pub const MAX_GOVERNANCE_FEE_SHARE : u64 = 75u64 * 10000000000000000u64;        // 75%    //TODO EVM mismatch (move to factory)

pub const DECAY_RATE: U256 = uint!("86400");    // 60*60*24


// Pool Storage
pub const SETUP_MASTER: Item<Option<Addr>> = Item::new("catalyst-pool-setup-master");
pub const CHAIN_INTERFACE: Item<Option<Addr>> = Item::new("catalyst-pool-chain-interface");

pub const ASSETS: Item<Vec<Addr>> = Item::new("catalyst-pool-assets");
pub const WEIGHTS: Item<Vec<u64>> = Item::new("catalyst-pool-weights");                                 //TODO use mapping instead?

pub const FEE_ADMINISTRATOR: Item<Addr> = Item::new("catalyst-pool-fee-administrator");
pub const POOL_FEE: Item<u64> = Item::new("catalyst-pool-pool-fee");
pub const GOVERNANCE_FEE_SHARE: Item<u64> = Item::new("catalyst-pool-governance-fee");

pub const POOL_CONNECTIONS: Map<(&str, Vec<u8>), bool> = Map::new("catalyst-pool-connections");         //TODO channelId and toPool types

pub const TOTAL_ESCROWED_ASSETS: Map<&str, Uint128> = Map::new("catalyst-pool-escrowed-assets");        //TODO use mapping instead?
pub const TOTAL_ESCROWED_LIQUIDITY: Item<Uint128> = Item::new("catalyst-pool-escrowed-pool-tokens");
pub const ASSET_ESCROWS: Map<&str, String> = Map::new("catalyst-pool-asset-escrows");                   //TODO use Addr instead of String (for fallback_account)
pub const LIQUIDITY_ESCROWS: Map<&str, String> = Map::new("catalyst-pool-liquidity-escrows");           //TODO use Addr instead of String (for fallback_account)

pub const MAX_LIMIT_CAPACITY: Item<U256> = Item::new("catalyst-pool-max-limit-capacity");
pub const USED_LIMIT_CAPACITY: Item<U256> = Item::new("catalyst-pool-used-limit-capacity");
pub const USED_LIMIT_CAPACITY_TIMESTAMP: Item<u64> = Item::new("catalyst-pool-used-limit-capacity-timestamp");


// TODO move to utils/similar?
fn calc_keccak256(message: Vec<u8>) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(message);
    format!("{:?}", hasher.finalize().to_vec())
}


// TODO replace with implementation
fn factory_owner() -> String {
    "factory_owner".to_string()
}


pub fn get_asset_index(assets: &Vec<Addr>, asset: &str) -> Result<usize, ContractError> {
    assets
        .iter()
        .enumerate()
        .find_map(|(index, a): (usize, &Addr)| if *a == asset { Some(index) } else { None })
        .ok_or(ContractError::InvalidAssets {})
}


pub fn only_local(deps: &Deps) -> StdResult<bool> {

    Ok(CHAIN_INTERFACE.load(deps.storage)?.is_none())

}


pub fn ready(deps: &Deps) -> StdResult<bool> {

    let setup_master = SETUP_MASTER.load(deps.storage)?;
    let assets = ASSETS.load(deps.storage)?;

    Ok(setup_master.is_none() && assets.len() > 0)

}


pub fn calc_unit_capacity(
    deps: &Deps,
    time: Timestamp
) -> Result<U256, ContractError> {

    let max_limit_capacity = MAX_LIMIT_CAPACITY.load(deps.storage)?;
    let used_limit_capacity = USED_LIMIT_CAPACITY.load(deps.storage)?;
    let used_limit_capacity_timestamp = USED_LIMIT_CAPACITY_TIMESTAMP.load(deps.storage)?;

    let released_limit_capacity = max_limit_capacity
        .checked_mul(
            U256::from(time.minus_nanos(used_limit_capacity_timestamp).seconds())  //TODO use seconds instead of nanos (overflow wise)
        ).ok_or(ContractError::ArithmeticError {})?   //TODO error
        .div(DECAY_RATE);

        if used_limit_capacity <= released_limit_capacity {
            return Ok(max_limit_capacity);
        }

        if max_limit_capacity <= used_limit_capacity - released_limit_capacity {
            return Ok(U256::ZERO);
        }

        Ok(
            max_limit_capacity
                .checked_add(released_limit_capacity).ok_or(ContractError::ArithmeticError {})?
                .sub(used_limit_capacity)
        )

}


pub fn update_unit_capacity(
    deps: &mut DepsMut,
    current_time: Timestamp,
    units: U256
) -> Result<(), ContractError> {

    //TODO EVM mismatch
    let capacity = calc_unit_capacity(&deps.as_ref(), current_time)?;

    if units > capacity {
        return Err(ContractError::SecurityLimitExceeded { units, capacity });
    }

    let new_capacity = capacity - units;
    let timestamp = current_time.nanos();

    USED_LIMIT_CAPACITY.save(deps.storage, &new_capacity)?;
    USED_LIMIT_CAPACITY_TIMESTAMP.save(deps.storage, &timestamp)?;

    Ok(())
}


pub fn total_supply(deps: Deps) -> Result<Uint128, ContractError> {
    let info = TOKEN_INFO.load(deps.storage)?;
    Ok(info.total_supply)
}


pub fn finish_setup(
    deps: &mut DepsMut,
    info: MessageInfo
) -> Result<Response, ContractError> {

    let setup_master = SETUP_MASTER.load(deps.storage)?;

    if setup_master != Some(info.sender) {
        return Err(ContractError::Unauthorized {})
    }

    SETUP_MASTER.save(deps.storage, &None)?;

    Ok(Response::new())
}

    
pub fn set_connection(
    deps: &mut DepsMut,
    info: MessageInfo,
    channel_id: String,
    to_pool: Vec<u8>,
    state: bool
) -> Result<Response, ContractError> {

    let setup_master = SETUP_MASTER.load(deps.storage)?;

    if setup_master != Some(info.sender) {   // TODO check also for factory owner
        return Err(ContractError::Unauthorized {});
    }

    POOL_CONNECTIONS.save(deps.storage, (channel_id.as_str(), to_pool.clone()), &state)?;

    Ok(
        Response::new()
            .add_attribute("channel_id", channel_id)
            .add_attribute("to_pool",  format!("{:x?}", to_pool))
            .add_attribute("state", state.to_string())
    )
}


pub fn is_connected(
    deps: &Deps,
    channel_id: &str,
    to_pool: Vec<u8>
) -> bool {

    POOL_CONNECTIONS
        .load(deps.storage, (channel_id, to_pool))
        .unwrap_or(false)

}


pub fn set_fee_administrator_unchecked(
    deps: &mut DepsMut,
    administrator: &str
) -> Result<Event, ContractError> {

    FEE_ADMINISTRATOR.save(
        deps.storage,
        &deps.api.addr_validate(administrator)?
    )?;

    return Ok(
        Event::new(String::from("SetFeeAdministrator"))
            .add_attribute("administrator", administrator)
    )
}


pub fn set_pool_fee_unchecked(
    deps: &mut DepsMut,
    fee: u64
) -> Result<Event, ContractError> {

    if fee > MAX_POOL_FEE_SHARE {
        return Err(
            ContractError::InvalidPoolFee { requested_fee: fee, max_fee: MAX_POOL_FEE_SHARE }
        )
    }

    POOL_FEE.save(deps.storage, &fee)?;

    return Ok(
        Event::new(String::from("SetPoolFee"))
            .add_attribute("fee", fee.to_string())
    )
}


pub fn set_pool_fee(
    deps: &mut DepsMut,
    info: MessageInfo,
    fee: u64
) -> Result<Response, ContractError> {

    let fee_administrator = FEE_ADMINISTRATOR.load(deps.storage)?;

    if info.sender != fee_administrator {
        return Err(ContractError::Unauthorized {})
    }

    let event = set_pool_fee_unchecked(deps, fee)?;

    Ok(Response::new().add_event(event))
}


pub fn set_governance_fee_share_unchecked(
    deps: &mut DepsMut,
    fee: u64
) -> Result<Event, ContractError> {

    if fee > MAX_GOVERNANCE_FEE_SHARE {
        return Err(
            ContractError::InvalidGovernanceFee { requested_fee: fee, max_fee: MAX_GOVERNANCE_FEE_SHARE }
        )
    }

    GOVERNANCE_FEE_SHARE.save(deps.storage, &fee)?;

    return Ok(
        Event::new(String::from("SetGovernanceFeeShare"))
            .add_attribute("fee", fee.to_string())
    )
}


pub fn set_governance_fee_share(
    deps: &mut DepsMut,
    info: MessageInfo,
    fee: u64
) -> Result<Response, ContractError> {

    let fee_administrator = FEE_ADMINISTRATOR.load(deps.storage)?;

    if info.sender != fee_administrator {
        return Err(ContractError::Unauthorized {})
    }

    let event = set_governance_fee_share_unchecked(deps, fee)?;

    Ok(Response::new().add_event(event))
}


pub fn collect_governance_fee_message(
    deps: &Deps,
    env: Env,
    asset: String,
    pool_fee_amount: Uint128
) -> Result<Option<CosmosMsg>, ContractError> {

    let gov_fee_amount: Uint128 = mul_wad_down(
        U256::from(pool_fee_amount.u128()),
        U256::from(GOVERNANCE_FEE_SHARE.load(deps.storage)?)
    )?.as_u128().into();     //TODO unsafe as_u128 casting

    if gov_fee_amount.is_zero() {
        return Ok(None)
    }

    Ok(Some(CosmosMsg::Wasm(
        cosmwasm_std::WasmMsg::Execute {
            contract_addr: asset,
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: factory_owner().to_string(),
                amount: gov_fee_amount
            })?,
            funds: vec![]
        }
    )))
    
}


pub fn set_fee_administrator(
    deps: &mut DepsMut,
    _info: MessageInfo,
    administrator: String
) -> Result<Response, ContractError> {

    //TODO verify sender is factory owner

    let event = set_fee_administrator_unchecked(deps, administrator.as_str())?;

    Ok(Response::new().add_event(event))
}


//TODO merge setup and initializeSwapCurves?
pub fn setup(
    deps: &mut DepsMut,
    env: &Env,
    name: String,
    symbol: String,
    chain_interface: Option<String>,
    pool_fee: u64,
    governance_fee: u64,
    fee_administrator: String,
    setup_master: String,
) -> Result<Response, ContractError> {

    SETUP_MASTER.save(
        deps.storage,
        &Some(deps.api.addr_validate(&setup_master)?)
    )?;
    
    CHAIN_INTERFACE.save(
        deps.storage,
        &match chain_interface {
            Some(chain_interface) => Some(deps.api.addr_validate(&chain_interface)?),
            None => None
        }
    )?;


    let admin_fee_event = set_fee_administrator_unchecked(deps, fee_administrator.as_str())?;
    let pool_fee_event = set_pool_fee_unchecked(deps, pool_fee)?;
    let gov_fee_event = set_governance_fee_share_unchecked(deps, governance_fee)?;

    // Setup the Pool Token (store token info using cw20-base format)
    let data = TokenInfo {
        name,
        symbol,
        decimals: DECIMALS,
        total_supply: Uint128::zero(),
        mint: Some(MinterData {
            minter: env.contract.address.clone(),  // Set self as minter
            cap: None
        })
    };
    TOKEN_INFO.save(deps.storage, &data)?;

    Ok(
        Response::new()
            .add_event(admin_fee_event)
            .add_event(pool_fee_event)
            .add_event(gov_fee_event)
    )
}


//TODO rename
pub fn get_unit_capacity(
    deps: &Deps,
    env: Env
) -> Result<U256, ContractError> {

    calc_unit_capacity(deps, env.block.time)

}


pub fn create_asset_escrow(
    deps: &mut DepsMut,
    send_asset_hash: &str,
    amount: Uint128,
    asset: &str,
    fallback_account: String
) -> Result<(), ContractError> {

    if ASSET_ESCROWS.has(deps.storage, send_asset_hash) {
        return Err(ContractError::Unauthorized {});
    }

    ASSET_ESCROWS.save(deps.storage, send_asset_hash, &fallback_account)?;

    let escrowed_assets = TOTAL_ESCROWED_ASSETS.load(deps.storage, asset)?;
    TOTAL_ESCROWED_ASSETS.save(deps.storage, asset, &escrowed_assets.checked_add(amount)?)?;

    Ok(())
}


pub fn create_liquidity_escrow(
    deps: &mut DepsMut,
    send_liquidity_hash: &str,
    amount: Uint128,
    fallback_account: String
) -> Result<(), ContractError> {

    if LIQUIDITY_ESCROWS.has(deps.storage, send_liquidity_hash) {
        return Err(ContractError::Unauthorized {});
    }

    LIQUIDITY_ESCROWS.save(deps.storage, send_liquidity_hash, &fallback_account)?;

    let escrowed_pool_tokens = TOTAL_ESCROWED_LIQUIDITY.load(deps.storage)?;
    TOTAL_ESCROWED_LIQUIDITY.save(deps.storage, &escrowed_pool_tokens.checked_add(amount)?)?;

    Ok(())
}


pub fn release_asset_escrow(
    deps: &mut DepsMut,
    send_asset_hash: &str,
    amount: Uint128,
    asset: &str
) -> Result<String, ContractError> {

    let fallback_account = ASSET_ESCROWS.load(deps.storage, send_asset_hash)?;
    ASSET_ESCROWS.remove(deps.storage, send_asset_hash);

    let escrowed_assets = TOTAL_ESCROWED_ASSETS.load(deps.storage, asset)?;
    TOTAL_ESCROWED_ASSETS.save(deps.storage, asset, &(escrowed_assets - amount))?;        // Safe, as 'amount' is always contained in 'escrowed_assets'

    Ok(fallback_account)
}


pub fn release_liquidity_escrow(
    deps: &mut DepsMut,
    send_liquidity_hash: &str,
    amount: Uint128
) -> Result<String, ContractError> {

    let fallback_account = LIQUIDITY_ESCROWS.load(deps.storage, send_liquidity_hash)?;
    LIQUIDITY_ESCROWS.remove(deps.storage, send_liquidity_hash);

    let escrowed_pool_tokens = TOTAL_ESCROWED_LIQUIDITY.load(deps.storage)?;
    TOTAL_ESCROWED_LIQUIDITY.save(deps.storage, &(escrowed_pool_tokens - amount))?;     // Safe, as 'amount' is always contained in 'escrowed_assets'

    Ok(fallback_account)
}


pub fn on_send_asset_ack(
    deps: &mut DepsMut,
    info: MessageInfo,
    to_account: Vec<u8>,
    u: U256,
    amount: Uint128,
    asset: String,
    block_number_mod: u32
) -> Result<Response, ContractError> {

    if Some(info.sender) != CHAIN_INTERFACE.load(deps.storage)? {
        return Err(ContractError::Unauthorized {})
    }

    let send_asset_hash = compute_send_asset_hash(
        to_account.as_slice(),
        u,
        amount,
        asset.as_str(),
        block_number_mod
    );

    release_asset_escrow(deps, &send_asset_hash, amount, &asset)?;

    Ok(
        Response::new()
            .add_attribute("swap_hash", send_asset_hash)
    )
}


pub fn send_asset_ack(
    deps: &mut DepsMut,
    info: MessageInfo,
    to_account: Vec<u8>,
    u: U256,
    amount: Uint128,
    asset: String,
    block_number_mod: u32
) -> Result<Response, ContractError> {

    on_send_asset_ack(
        deps,
        info,
        to_account,
        u,
        amount,
        asset,
        block_number_mod
    )
}


pub fn on_send_asset_timeout(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    to_account: Vec<u8>,
    u: U256,
    amount: Uint128,
    asset: String,
    block_number_mod: u32
) -> Result<Response, ContractError> {

    if Some(info.sender) != CHAIN_INTERFACE.load(deps.storage)? {
        return Err(ContractError::Unauthorized {})
    }

    let send_asset_hash = compute_send_asset_hash(
        to_account.as_slice(),
        u,
        amount,
        asset.as_str(),
        block_number_mod
    );

    let fallback_address = release_asset_escrow(deps, &send_asset_hash, amount, &asset)?;

    // Transfer escrowed asset to fallback user
    let transfer_msg: CosmosMsg = CosmosMsg::Wasm(
        cosmwasm_std::WasmMsg::Execute {
            contract_addr: asset.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                owner: env.contract.address.to_string(),
                recipient: fallback_address,
                amount
            })?,
            funds: vec![]
        }
    );

    Ok(
        Response::new()
            .add_message(transfer_msg)
            .add_attribute("swap_hash", send_asset_hash)
    )
}


pub fn send_asset_timeout(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    to_account: Vec<u8>,
    u: U256,
    amount: Uint128,
    asset: String,
    block_number_mod: u32
) -> Result<Response, ContractError> {

    on_send_asset_timeout(
        deps,
        env,
        info,
        to_account,
        u,
        amount,
        asset,
        block_number_mod
    )

}


pub fn on_send_liquidity_ack(
    deps: &mut DepsMut,
    info: MessageInfo,
    to_account: Vec<u8>,
    u: U256,
    amount: Uint128,
    block_number_mod: u32
) -> Result<Response, ContractError> {

    if Some(info.sender) != CHAIN_INTERFACE.load(deps.storage)? {
        return Err(ContractError::Unauthorized {})
    }

    let send_liquidity_hash = compute_send_liquidity_hash(
        to_account.as_slice(),
        u,
        amount,
        block_number_mod
    );

    release_liquidity_escrow(deps, &send_liquidity_hash, amount)?;

    Ok(
        Response::new()
            .add_attribute("swap_hash", send_liquidity_hash)
    )
}


pub fn send_liquidity_ack(
    deps: &mut DepsMut,
    info: MessageInfo,
    to_account: Vec<u8>,
    u: U256,
    amount: Uint128,
    block_number_mod: u32
) -> Result<Response, ContractError> {

    on_send_liquidity_ack(
        deps,
        info,
        to_account,
        u,
        amount,
        block_number_mod
    )

}


pub fn on_send_liquidity_timeout(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    to_account: Vec<u8>,
    u: U256,
    amount: Uint128,
    block_number_mod: u32
) -> Result<Response, ContractError> {

    if Some(info.sender) != CHAIN_INTERFACE.load(deps.storage)? {
        return Err(ContractError::Unauthorized {})
    }

    let send_liquidity_hash = compute_send_liquidity_hash(
        to_account.as_slice(),
        u,
        amount,
        block_number_mod
    );

    let fallback_address = release_liquidity_escrow(deps, &send_liquidity_hash, amount)?;

    // Mint pool tokens for the fallbackAccount
    let execute_mint_info = MessageInfo {
        sender: env.contract.address.clone(),
        funds: vec![],
    };
    let mint_response = execute_mint(
        deps.branch(),
        env,
        execute_mint_info,
        fallback_address,
        amount
    )?;

    Ok(
        Response::new()
            .add_attribute("swap_hash", send_liquidity_hash)
            .add_attributes(mint_response.attributes)   //TODO better way to do this?
    )
}

pub fn send_liquidity_timeout(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    to_account: Vec<u8>,
    u: U256,
    amount: Uint128,
    block_number_mod: u32
) -> Result<Response, ContractError> {

    on_send_liquidity_timeout(
        deps,
        env,
        info,
        to_account,
        u,
        amount,
        block_number_mod
    )
}


pub fn compute_send_asset_hash(
    to_account: &[u8],
    u: U256,
    amount: Uint128,
    asset: &str,
    block_number_mod: u32        
) -> String {

    let asset_bytes = asset.as_bytes();

    let mut hash_data: Vec<u8> = Vec::with_capacity(    // Initialize vec with the specified capacity (avoid reallocations)
        to_account.len()
            + 32
            + 16
            + asset_bytes.len()
            + 4
    );

    hash_data.extend_from_slice(to_account);
    hash_data.extend_from_slice(&u.to_be_bytes());
    hash_data.extend_from_slice(&amount.to_be_bytes());
    hash_data.extend_from_slice(asset_bytes);
    hash_data.extend_from_slice(&block_number_mod.to_be_bytes());
    
    calc_keccak256(hash_data)
}


pub fn compute_send_liquidity_hash(
    to_account: &[u8],
    u: U256,
    amount: Uint128,
    block_number_mod: u32        
) -> String {

    let mut hash_data: Vec<u8> = Vec::with_capacity(    // Initialize vec with the specified capacity (avoid reallocations)
        to_account.len()
            + 32
            + 16
            + 4
    );

    hash_data.extend_from_slice(to_account);
    hash_data.extend_from_slice(&u.to_be_bytes());
    hash_data.extend_from_slice(&amount.to_be_bytes());
    hash_data.extend_from_slice(&block_number_mod.to_be_bytes());
    
    calc_keccak256(hash_data)
}



#[cw_serde]
pub struct Escrow {
    pub fallback_address: Addr
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

pub fn query_ready(deps: Deps) -> StdResult<ReadyResponse> {
    Ok(
        ReadyResponse {
            ready: ready(&deps)?
        }
    )
}

pub fn query_only_local(deps: Deps) -> StdResult<OnlyLocalResponse> {
    Ok(
        OnlyLocalResponse {
            only_local: only_local(&deps)?
        }
    )
}

pub fn query_assets(deps: Deps) -> StdResult<AssetsResponse> {
    Ok(
        AssetsResponse {
            assets: ASSETS.load(deps.storage)?
        }
    )
}

pub fn query_weights(deps: Deps) -> StdResult<WeightsResponse> {
    Ok(
        WeightsResponse {
            weights: WEIGHTS.load(deps.storage)?
        }
    )
}

pub fn query_pool_fee(deps: Deps) -> StdResult<PoolFeeResponse> {
    Ok(
        PoolFeeResponse {
            fee: POOL_FEE.load(deps.storage)?
        }
    )
}

pub fn query_governance_fee_share(deps: Deps) -> StdResult<GovernanceFeeShareResponse> {
    Ok(
        GovernanceFeeShareResponse {
            fee: GOVERNANCE_FEE_SHARE.load(deps.storage)?
        }
    )
}

pub fn query_fee_administrator(deps: Deps) -> StdResult<FeeAdministratorResponse> {
    Ok(
        FeeAdministratorResponse {
            administrator: FEE_ADMINISTRATOR.load(deps.storage)?
        }
    )
}

pub fn query_total_escrowed_asset(deps: Deps, asset: &str) -> StdResult<TotalEscrowedAssetResponse> {
    Ok(
        TotalEscrowedAssetResponse {
            amount: TOTAL_ESCROWED_ASSETS.load(deps.storage, asset)?
        }
    )
}

pub fn query_total_escrowed_liquidity(deps: Deps) -> StdResult<TotalEscrowedLiquidityResponse> {
    Ok(
        TotalEscrowedLiquidityResponse {
            amount: TOTAL_ESCROWED_LIQUIDITY.load(deps.storage)?
        }
    )
}

pub fn query_asset_escrow(deps: Deps, hash: &str) -> StdResult<AssetEscrowResponse> {
    Ok(
        AssetEscrowResponse {
            fallback_account: ASSET_ESCROWS.load(deps.storage, hash)?
        }
    )
}

pub fn query_liquidity_escrow(deps: Deps, hash: &str) -> StdResult<LiquidityEscrowResponse> {
    Ok(
        LiquidityEscrowResponse {
            fallback_account: LIQUIDITY_ESCROWS.load(deps.storage, hash)?
        }
    )
}

pub fn query_pool_connection_state(deps: Deps, channel_id: &str, pool: Vec<u8>) -> StdResult<PoolConnectionStateResponse> {
    Ok(
        PoolConnectionStateResponse {
            state: POOL_CONNECTIONS.load(deps.storage, (channel_id, pool))?
        }
    )
}