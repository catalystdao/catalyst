use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128, DepsMut, Env, Response, Event, MessageInfo, Deps, StdResult, to_json_binary, Timestamp, StdError, Binary, Uint64, CosmosMsg};
use cw_storage_plus::{Map, Item};
use sha3::{Digest, Keccak256};
use std::ops::Div;

use catalyst_types::{U256, u256, Bytes32};
use fixed_point_math::mul_wad_down;

use crate::{ContractError, bindings::{VaultAssets, Asset, VaultAssetsTrait, AssetTrait, VaultToken, VaultTokenTrait, VaultResponse, IntoCosmosCustomMsg, CustomMsg}, msg::{ChainInterfaceResponse, SetupMasterResponse, ReadyResponse, OnlyLocalResponse, AssetsResponse, WeightResponse, VaultFeeResponse, GovernanceFeeShareResponse, FeeAdministratorResponse, TotalEscrowedAssetResponse, TotalEscrowedLiquidityResponse, AssetEscrowResponse, LiquidityEscrowResponse, VaultConnectionStateResponse, FactoryResponse, FactoryOwnerResponse, ReceiverExecuteMsg, TotalSupplyResponse, BalanceResponse, AssetResponse}, event::{send_asset_success_event, send_asset_failure_event, send_liquidity_success_event, send_liquidity_failure_event, finish_setup_event, set_fee_administrator_event, set_vault_fee_event, set_governance_fee_share_event, set_connection_event}};

#[cfg(feature="asset_native")]
use crate::msg::VaultTokenDenomResponse;


// Vault Constants **************************************************************************************************************

pub const MAX_ASSETS: usize = 3;

pub const DECIMALS: u8 = 18;
pub const INITIAL_MINT_AMOUNT: Uint128 = Uint128::new(10u128.pow(DECIMALS as u32));

pub const MAX_VAULT_FEE             : Uint64 = Uint64::new(1000000000000000000u64);       // 100%
pub const MAX_GOVERNANCE_FEE_SHARE  : Uint64 = Uint64::new(75u64 * 10000000000000000u64); // 75%

pub const DECAY_RATE: U256 = u256!("86400");    // 60*60*24

pub const CATALYST_ENCODED_ADDRESS_LENGTH: usize = 65usize;

pub const UNDERWRITER_ESCROW_ADDRESS_ID: &str = "underwriter";




// Vault Storage ****************************************************************************************************************

pub const FACTORY: Item<Addr> = Item::new("catalyst-vault-factory");
pub const SETUP_MASTER: Item<Option<Addr>> = Item::new("catalyst-vault-setup-master");
pub const CHAIN_INTERFACE: Item<Option<Addr>> = Item::new("catalyst-vault-chain-interface");

pub const WEIGHTS: Map<&str, Uint128> = Map::new("catalyst-vault-weights");

pub const FEE_ADMINISTRATOR: Item<Addr> = Item::new("catalyst-vault-fee-administrator");
pub const VAULT_FEE: Item<Uint64> = Item::new("catalyst-vault-vault-fee");
pub const GOVERNANCE_FEE_SHARE: Item<Uint64> = Item::new("catalyst-vault-governance-fee");

pub const VAULT_CONNECTIONS: Map<(&[u8], Vec<u8>), bool> = Map::new("catalyst-vault-connections");

pub const TOTAL_ESCROWED_ASSETS: Map<&str, Uint128> = Map::new("catalyst-vault-escrowed-assets");
pub const TOTAL_ESCROWED_LIQUIDITY: Item<Uint128> = Item::new("catalyst-vault-escrowed-vault-tokens");
pub const ASSET_ESCROWS: Map<Vec<u8>, Addr> = Map::new("catalyst-vault-asset-escrows");
pub const LIQUIDITY_ESCROWS: Map<Vec<u8>, Addr> = Map::new("catalyst-vault-liquidity-escrows");

pub const MAX_LIMIT_CAPACITY: Item<U256> = Item::new("catalyst-vault-max-limit-capacity");
pub const USED_LIMIT_CAPACITY: Item<U256> = Item::new("catalyst-vault-used-limit-capacity");
pub const USED_LIMIT_CAPACITY_TIMESTAMP_SECONDS: Item<Uint64> = Item::new("catalyst-vault-used-limit-capacity-timestamp");




// State Helpers ****************************************************************************************************************

/// Check if the vault is 'only local' (i.e. does not have a cross chain interface).
pub fn only_local(deps: &Deps) -> StdResult<bool> {

    Ok(CHAIN_INTERFACE.load(deps.storage)?.is_none())

}


/// Check if the vault is ready. This means that 'finish_setup' has been called and
/// that the vault has got at least one asset.
pub fn ready(deps: &Deps) -> StdResult<bool> {

    let setup_master = SETUP_MASTER.load(deps.storage)?;
    let assets = VaultAssets::load_refs(deps)?;

    Ok(setup_master.is_none() && assets.len() > 0)

}


// Redefine the types used by the 'factory' for queries (the factory contract cannot be imported by this contract, 
// as it would create a cyclic dependency)
#[cw_serde]
enum FactoryContractQueryMsg {
    Owner {}
}

#[cw_serde]
struct FactoryContractOwnerResponse {
    pub owner: Option<Addr>
}

/// Query the factory owner directly from the factory contract.
pub fn factory_owner(deps: &Deps) -> Result<Addr, ContractError> {

    let response = deps.querier.query_wasm_smart::<FactoryContractOwnerResponse>(
        FACTORY.load(deps.storage)?,
        &FactoryContractQueryMsg::Owner {}
    )?;

    response.owner.ok_or(ContractError::Error("Factory has no owner.".to_string()))
}




// Vault Setup ******************************************************************************************************************

/// Setup the vault configuration
/// 
/// # Arguments:
/// * `name` - The name of the vault token.
/// * `symbol` - The symbol of the vault token.
/// * `chain_interface` - The interface used for cross-chain swaps. It can be set to None to disable cross-chain swaps.
/// * `vault_fee` - The vault fee (18 decimals).
/// * `governance_fee_share` - The governance fee share (18 decimals).
/// * `fee_administrator` - The account which has the authority to modify the vault fee.
/// * `setup_master` - The account which has the authority to continue setting up the vault (until `finish_setup` is called).
/// 
pub fn setup(
    deps: &mut DepsMut,
    env: &Env,
    info: MessageInfo,
    name: String,
    symbol: String,
    chain_interface: Option<String>,
    vault_fee: Uint64,
    governance_fee_share: Uint64,
    fee_administrator: String,
    setup_master: String
) -> Result<VaultResponse, ContractError> {

    // Save accounts
    FACTORY.save(
        deps.storage,
        &info.sender        // Set the 'factory' as the sender of the transaction
    )?;

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

    // Setup fees
    let admin_fee_event = set_fee_administrator_unchecked(deps, fee_administrator.as_str())?;
    let vault_fee_event = set_vault_fee_unchecked(deps, vault_fee)?;
    let gov_fee_event = set_governance_fee_share_unchecked(deps, governance_fee_share)?;

    // Setup the Vault Token
    let create_vault_token_msg = VaultToken::create(
        deps,
        env,
        name,
        symbol,
        DECIMALS
    )?;

    let mut response = Response::new();

    if let Some(msg) = create_vault_token_msg {
        response = response.add_message(msg.into_cosmos_vault_msg());
    }

    Ok(
        response
            .add_event(admin_fee_event)
            .add_event(vault_fee_event)
            .add_event(gov_fee_event)
    )
}


/// Initialize the escrow totals storage variables.
/// 
/// # Arguments:
/// * `assets_refs` - The vault assets references.
/// 
pub fn initialize_escrow_totals(
    deps: &mut DepsMut,
    assets_refs: Vec<&str>
) -> Result<(), ContractError> {

    assets_refs
        .iter()
        .map(|asset_ref| {
            TOTAL_ESCROWED_ASSETS.save(deps.storage, asset_ref, &Uint128::zero())
        })
        .collect::<StdResult<Vec<_>>>()?;

    TOTAL_ESCROWED_LIQUIDITY.save(deps.storage, &Uint128::zero())?;

    Ok(())
}


/// Initialize the security limit capacity storage variables.
/// 
/// # Arguments:
/// * `max_limit_capacity` - The maximum limit capacity.
/// 
pub fn initialize_limit_capacity(
    deps: &mut DepsMut,
    max_limit_capacity: U256
) -> Result<(), ContractError> {

    MAX_LIMIT_CAPACITY.save(deps.storage, &max_limit_capacity)?;
    USED_LIMIT_CAPACITY.save(deps.storage, &U256::zero())?;
    USED_LIMIT_CAPACITY_TIMESTAMP_SECONDS.save(deps.storage, &Uint64::zero())?;

    Ok(())
}


/// Finish the vault setup. This revokes the 'setup_master' authority.
/// 
/// **NOTE**: This function checks whether the sender of the transaction is the setup master.
pub fn finish_setup(
    deps: &mut DepsMut,
    info: MessageInfo
) -> Result<VaultResponse, ContractError> {

    let setup_master = SETUP_MASTER.load(deps.storage)?;

    if setup_master != Some(info.sender) {
        return Err(ContractError::Unauthorized {})
    }

    SETUP_MASTER.save(deps.storage, &None)?;

    Ok(
        Response::new()
            .add_event(finish_setup_event())
    )
}




// Vault Administration *********************************************************************************************************

/// Setup a vault connection.
/// 
/// **NOTE**: This function checks whether the sender of the transaction is the setup master.
/// 
/// # Arguments:
/// * `channel_id` - The channel id that connects with the remoute vault.
/// * `vault` - The remote vault address to be connected to this vault.
/// * `state` - Whether the connection is enabled.
/// 
pub fn set_connection(
    deps: &mut DepsMut,
    info: MessageInfo,
    channel_id: Bytes32,
    vault: Binary,
    state: bool
) -> Result<VaultResponse, ContractError> {

    // Only allow a connection setup if the caller is the setup master
    let setup_master = SETUP_MASTER.load(deps.storage)?;
    if Some(info.sender.clone()) != setup_master {
        return Err(ContractError::Unauthorized {});
    }

    if vault.len() != CATALYST_ENCODED_ADDRESS_LENGTH {
        return Err(
            ContractError::Error("'vault' address is of invalid length (Catalyst specific address encoding expected).".to_string())
        );
    }

    VAULT_CONNECTIONS.save(deps.storage, (channel_id.as_slice(), vault.0.clone()), &state)?;

    Ok(
        Response::new()
            .add_event(
                set_connection_event(
                    channel_id,
                    vault,
                    state
                )
            )
    )
}


/// Get whether a remote vault is connected.
/// 
/// # Arguments:
/// * `channel_id` - The channel id that connects with the remoute vault.
/// * `vault` - The remote vault address.
/// 
pub fn is_connected(
    deps: &Deps,
    channel_id: &Bytes32,
    vault: Binary
) -> bool {

    VAULT_CONNECTIONS
        .load(deps.storage, (channel_id.as_slice(), vault.0))
        .unwrap_or(false)

}


/// Set the fee administrator (unchecked).
/// 
/// !IMPORTANT: This function DOES NOT check the sender of the transaction.
/// 
/// # Arguments:
/// * `administrator` - The account of the new addministrator.
/// 
pub fn set_fee_administrator_unchecked(
    deps: &mut DepsMut,
    administrator: &str
) -> Result<Event, ContractError> {

    FEE_ADMINISTRATOR.save(
        deps.storage,
        &deps.api.addr_validate(administrator)?
    )?;

    return Ok(
        set_fee_administrator_event(administrator.to_string())
    )
}


/// Set the fee administrator.
/// 
/// **NOTE**: This function checks whether the sender of the transaction is the factory owner.
/// 
/// # Arguments:
/// * `administrator` - The new administrator account.
/// 
pub fn set_fee_administrator(
    deps: &mut DepsMut,
    info: MessageInfo,
    administrator: String
) -> Result<VaultResponse, ContractError> {

    if info.sender != factory_owner(&deps.as_ref())? {
        return Err(ContractError::Unauthorized {})
    }

    let event = set_fee_administrator_unchecked(deps, administrator.as_str())?;

    Ok(Response::new().add_event(event))
}


/// Set the vault fee (unchecked).
/// 
/// !IMPORTANT: This function DOES NOT check the sender of the transaction.
/// 
/// # Arguments:
/// * `fee` - The new vault fee (18 decimals).
/// 
pub fn set_vault_fee_unchecked(
    deps: &mut DepsMut,
    fee: Uint64
) -> Result<Event, ContractError> {

    if fee > MAX_VAULT_FEE {
        return Err(
            ContractError::InvalidVaultFee { requested_fee: fee, max_fee: MAX_VAULT_FEE }
        )
    }

    VAULT_FEE.save(deps.storage, &fee)?;

    return Ok(
        set_vault_fee_event(fee)
    )
}


/// Set the vault fee.
/// 
/// **NOTE**: This function checks whether the sender of the transaction is the fee administrator.
/// 
/// # Arguments:
/// * `fee` - The new vault fee (18 decimals).
/// 
pub fn set_vault_fee(
    deps: &mut DepsMut,
    info: MessageInfo,
    fee: Uint64
) -> Result<VaultResponse, ContractError> {

    let fee_administrator = FEE_ADMINISTRATOR.load(deps.storage)?;

    if info.sender != fee_administrator {
        return Err(ContractError::Unauthorized {})
    }

    let event = set_vault_fee_unchecked(deps, fee)?;

    Ok(Response::new().add_event(event))
}


/// Set the governance fee share (unchecked).
/// 
/// !IMPORTANT: This function DOES NOT check the sender of the transaction.
/// 
/// # Arguments:
/// * `fee` - The new governance fee share (18 decimals).
/// 
pub fn set_governance_fee_share_unchecked(
    deps: &mut DepsMut,
    fee: Uint64
) -> Result<Event, ContractError> {

    if fee > MAX_GOVERNANCE_FEE_SHARE {
        return Err(
            ContractError::InvalidGovernanceFee { requested_fee: fee, max_fee: MAX_GOVERNANCE_FEE_SHARE }
        )
    }

    GOVERNANCE_FEE_SHARE.save(deps.storage, &fee)?;

    return Ok(
        set_governance_fee_share_event(fee)
    )
}


/// Set the governance fee share.
/// 
/// **NOTE**: This function checks whether the sender of the transaction is the factory owner.
/// 
/// # Arguments:
/// * `fee` - The new governance fee share (18 decimals).
/// 
pub fn set_governance_fee_share(
    deps: &mut DepsMut,
    info: MessageInfo,
    fee: Uint64
) -> Result<VaultResponse, ContractError> {

    if info.sender != factory_owner(&deps.as_ref())? {
        return Err(ContractError::Unauthorized {})
    }

    let event = set_governance_fee_share_unchecked(deps, fee)?;

    Ok(Response::new().add_event(event))
}


/// Build a message to transfer the governance fee share from the vault fee to the factory owner.
/// 
/// # Arguments:
/// * `asset` - The asset from which the fee is taken.
/// * `vault_fee_amount` - The vault fee amount.
/// 
pub fn collect_governance_fee_message(
    deps: &Deps,
    env: &Env,
    asset: &Asset,
    vault_fee_amount: Uint128
) -> Result<Option<CosmosMsg<CustomMsg>>, ContractError> {

    // Compute the governance fee as the GOVERNANCE_FEE_SHARE percentage of the vault_fee_amount.
    let gov_fee_amount: Uint128 = mul_wad_down(
        U256::from(vault_fee_amount),
        U256::from(GOVERNANCE_FEE_SHARE.load(deps.storage)?)
    )?.try_into()?;

    Ok(
        asset.send_asset(
            env,
            gov_fee_amount,
            factory_owner(deps)?.to_string()
        )?
        .and_then(|msg| Some(msg.into_cosmos_vault_msg()))
    )
    
}




// Security limit ***************************************************************************************************************

/// Compute the security limit capacity at some time 'timestamp'.
/// 
/// # Arguments:
/// 
/// * `timestamp` - Time at which to compute the limit capacity (usually this is the current timestamp).
/// 
pub fn get_limit_capacity(
    deps: &Deps,
    timestamp: Timestamp
) -> Result<U256, ContractError> {

    let max_limit_capacity = MAX_LIMIT_CAPACITY.load(deps.storage)?;
    let used_limit_capacity = USED_LIMIT_CAPACITY.load(deps.storage)?;
    let used_limit_capacity_timestamp = USED_LIMIT_CAPACITY_TIMESTAMP_SECONDS.load(deps.storage)?;


    // Compute the 'released' limit capacity using the following linear decay:
    //      released_limit_capacity = max_limit_capacity * time_elapsed / decay_rate
    let time_elapsed = U256::from(
        timestamp.seconds()
            .checked_sub(used_limit_capacity_timestamp.into())      // Using 'checked_sub' in case the provided 'time' is less than the saved timestamp (implementation errors) 
            .ok_or(ContractError::ArithmeticError {})?
    );

    let released_limit_capacity = max_limit_capacity
        .checked_mul(time_elapsed)?
        .div(DECAY_RATE);


    // Return the *current* 'limit_capacity'
    if used_limit_capacity <= released_limit_capacity {
        return Ok(max_limit_capacity);
    }

    if max_limit_capacity <= used_limit_capacity.wrapping_sub(released_limit_capacity) {    // 'wrapping_sub' is safe because of the previous 'if' statement
        return Ok(U256::zero());
    }

    Ok(
        max_limit_capacity.wrapping_sub(                                // 'wrapping_sub' is safe because of the previous 'if' statement
            used_limit_capacity.wrapping_sub(released_limit_capacity)   // 'wrapping_sub' is safe because of the previous 'if' statement
        )
    )

}


/// Verify that the security limit allows for the requested amount and update it accordingly.
/// 
/// # Arguments:
/// 
/// * `current_timestamp` - The current time.
/// * `amount` - The amount by which to decrease the limit capacity.
/// 
pub fn update_limit_capacity(
    deps: &mut DepsMut,
    current_timestamp: Timestamp,
    amount: U256
) -> Result<(), ContractError> {

    // EVM-MISMATCH: For performance reasons, the EVM implementation does not make use of
    // the 'get_limit_capacity' function, but rather duplicates most of its logic. It has been
    // decided not to implement the optimization on this implementation.
    let capacity = get_limit_capacity(&deps.as_ref(), current_timestamp)?;

    // Verify that the security limit has capacity for the requested 'amount'.
    if amount > capacity {
        return Err(
            ContractError::SecurityLimitExceeded {
                overflow: amount.wrapping_sub(capacity)     // 'wrapping_sub' safe, as 'amount' > 'capacity'
            }
        );
    }

    let timestamp = current_timestamp.seconds();

    USED_LIMIT_CAPACITY.update(
        deps.storage,
        |used_capacity| -> StdResult<_> {
            used_capacity
                .checked_add(amount)
                .map_err(|err| err.into())
        }
    )?;
    USED_LIMIT_CAPACITY_TIMESTAMP_SECONDS.save(deps.storage, &timestamp.into())?;

    Ok(())
}




// Swap Helpers *****************************************************************************************************************

/// Create an asset escrow.
/// 
/// # Arguments:
/// * `send_asset_hash` - The id under which to create the escrow.
/// * `amount` - The escrow amount.
/// * `asset_ref` - The escrowed asset reference.
/// * `fallback_account` - The account which to return the escrowed assets in the case of an unsuccessful swap.
/// 
pub fn create_asset_escrow(
    deps: &mut DepsMut,
    send_asset_hash: Vec<u8>,
    amount: Uint128,
    asset_ref: &str,
    fallback_account: String
) -> Result<(), ContractError> {

    // ! IMPORTANT: Only create the escrow if the `send_asset_hash` is NOT already in use.
    if ASSET_ESCROWS.has(deps.storage, send_asset_hash.clone()) {
        return Err(ContractError::Unauthorized {});
    }

    // Verify the fallback account before saving it
    let fallback_account = deps.api.addr_validate(&fallback_account)?;
    ASSET_ESCROWS.save(deps.storage, send_asset_hash, &fallback_account)?;

    let escrowed_assets = TOTAL_ESCROWED_ASSETS.load(deps.storage, asset_ref)?;
    TOTAL_ESCROWED_ASSETS.save(deps.storage, asset_ref, &escrowed_assets.checked_add(amount)?)?;

    Ok(())
}


/// Create a liquidity escrow.
/// 
/// # Arguments:
/// * `send_liquidity_hash` - The id under which to create the escrow.
/// * `amount` - The escrow amount.
/// * `fallback_account` - The account which to return the escrowed liquidity in the case of an unsuccessful swap.
/// 
pub fn create_liquidity_escrow(
    deps: &mut DepsMut,
    send_liquidity_hash: Vec<u8>,
    amount: Uint128,
    fallback_account: String
) -> Result<(), ContractError> {

    // ! IMPORTANT: Only create the escrow if the `send_liquidity_hash` is NOT already in use.
    if LIQUIDITY_ESCROWS.has(deps.storage, send_liquidity_hash.clone()) {
        return Err(ContractError::Unauthorized {});
    }

    // Verify the fallback account before saving
    let fallback_account = deps.api.addr_validate(&fallback_account)?;
    LIQUIDITY_ESCROWS.save(deps.storage, send_liquidity_hash, &fallback_account)?;

    let escrowed_vault_tokens = TOTAL_ESCROWED_LIQUIDITY.load(deps.storage)?;
    TOTAL_ESCROWED_LIQUIDITY.save(deps.storage, &escrowed_vault_tokens.checked_add(amount)?)?;

    Ok(())
}


/// Create an underwrite escrow.
/// 
/// # Arguments:
/// * `underwrite_identifier` - The underwrite id.
/// * `amount` - The escrow amount.
/// * `asset_ref` - The escrowed asset reference.
/// 
pub fn create_underwrite_escrow(
    deps: &mut DepsMut,
    underwrite_identifier: Vec<u8>,
    amount: Uint128,
    asset_ref: &str
) -> Result<(), ContractError> {

    create_asset_escrow(
        deps,
        underwrite_identifier,
        amount,
        asset_ref,
        UNDERWRITER_ESCROW_ADDRESS_ID.to_string()
    )
}


/// Release an asset escrow and return the escrow's fallback account.
/// 
/// ! IMPORTANT: This function has no means of verifying the correctness of the escrow `amount`. 
/// The caller of this function should make sure that the `amount` is correct.
/// 
/// # Arguments:
/// * `send_asset_hash` - The id of the escrow to be released.
/// * `amount` - The escrow amount.
/// * `asset_ref` - The escrowed asset reference.
/// * `fallback_account` - The account which to return the escrowed assets in the case of an unsuccessful swap.
/// 
/// 
pub fn release_asset_escrow(
    deps: &mut DepsMut,
    send_asset_hash: Vec<u8>,
    amount: Uint128,
    asset_ref: &str
) -> Result<Addr, ContractError> {

    // Get the escrow information and delete the escrow
    let fallback_account = ASSET_ESCROWS.load(deps.storage, send_asset_hash.clone())?;
    ASSET_ESCROWS.remove(deps.storage, send_asset_hash);

    // Decrease the `total_escrowed_assets` tracker.
    let escrowed_assets = TOTAL_ESCROWED_ASSETS.load(deps.storage, asset_ref)?;
    TOTAL_ESCROWED_ASSETS.save(
        deps.storage,
        asset_ref,
        &(escrowed_assets.wrapping_sub(amount))     // 'wrapping_sub' is safe, as 'amount' is always contained in 'escrowed_assets'
                                                    // ! This is only the case if the 'amount' value is correct. But this a safe assumption
                                                    // ! as the 'amount' value should ALWAYS be verified before calling this function.
    )?;

    Ok(fallback_account)
}


/// Release a liquidity escrow and return the escrow's fallback account.
/// 
/// ! IMPORTANT: This function has no means of verifying the correctness of the escrow `amount`. 
/// The caller of this function should make sure that the `amount` is correct.
/// 
/// # Arguments:
/// * `send_liquidity_hash` - The id of the escrow to be released.
/// * `amount` - The escrow amount.
/// * `fallback_account` - The account which to return the escrowed liquidity in the case of an unsuccessful swap.
/// 
pub fn release_liquidity_escrow(
    deps: &mut DepsMut,
    send_liquidity_hash: Vec<u8>,
    amount: Uint128
) -> Result<Addr, ContractError> {

    // Get the escrow information and delete the escrow
    let fallback_account = LIQUIDITY_ESCROWS.load(deps.storage, send_liquidity_hash.clone())?;
    LIQUIDITY_ESCROWS.remove(deps.storage, send_liquidity_hash);

    // Decrease the `total_escrowed_liquidity` tracker.
    let escrowed_vault_tokens = TOTAL_ESCROWED_LIQUIDITY.load(deps.storage)?;
    TOTAL_ESCROWED_LIQUIDITY.save(
        deps.storage,
        &(escrowed_vault_tokens.wrapping_sub(amount))   // 'wrapping_sub' is safe, as 'amount' is always contained in 'escrowed_liquidity'
                                                        // ! This is only the case if the 'amount' value is correct. But this a safe assumption
                                                        // ! as the 'amount' value should ALWAYS be verified before calling this function.
    )?;

    Ok(fallback_account)
}


/// Release an underwrite escrow.
/// 
/// ! IMPORTANT: This function has no means of verifying the correctness of the escrow `amount`. 
/// The caller of this function should make sure that the `amount` is correct.
/// 
/// # Arguments:
/// * `underwrite_identifier` - The underwrite id.
/// * `amount` - The escrow amount.
/// * `asset_ref` - The escrowed asset reference.
/// 
pub fn release_underwrite_escrow(
    deps: &mut DepsMut,
    underwrite_identifier: Vec<u8>,
    amount: Uint128,
    asset_ref: &str
) -> Result<(), ContractError> {

    release_asset_escrow(
        deps,
        underwrite_identifier,
        amount,
        asset_ref
    )?;

    Ok(())
}


/// Handle the confirmation of a successful asset swap. This function deletes the swap escrow
/// and releases the escrowed assets into the vault.
/// 
/// **NOTE**: Only the chain interface may invoke this function.
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
pub fn on_send_asset_success(
    deps: &mut DepsMut,
    _env: &Env,
    info: &MessageInfo,
    channel_id: Bytes32,
    to_account: Binary,
    u: U256,
    escrow_amount: Uint128,
    asset_ref: String,
    block_number_mod: u32
) -> Result<VaultResponse, ContractError> {

    if Some(info.sender.clone()) != CHAIN_INTERFACE.load(deps.storage)? {
        return Err(ContractError::Unauthorized {})
    }

    // Hash the swap parameters to recover and release the swap escrow. If any of the values 
    // are tampered with this will fail.
    let send_asset_hash = compute_send_asset_hash(
        to_account.as_slice(),
        u,
        escrow_amount,
        asset_ref.as_str(),
        block_number_mod
    );

    release_asset_escrow(deps, send_asset_hash.clone(), escrow_amount, &asset_ref)?;

    Ok(
        Response::new()
            .add_event(
                send_asset_success_event(
                    channel_id,
                    to_account,
                    u,
                    escrow_amount,
                    asset_ref,
                    block_number_mod
                )
            )
    )
}


/// Handle the confirmation of an unsuccessful asset swap. This function deletes the swap escrow
/// and returns the escrowed assets to the fallback account.
/// 
/// **NOTE**: Only the chain interface may invoke this function.
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
pub fn on_send_asset_failure(
    deps: &mut DepsMut,
    env: &Env,
    info: &MessageInfo,
    channel_id: Bytes32,
    to_account: Binary,
    u: U256,
    escrow_amount: Uint128,
    asset_ref: String,
    block_number_mod: u32
) -> Result<VaultResponse, ContractError> {

    if Some(info.sender.clone()) != CHAIN_INTERFACE.load(deps.storage)? {
        return Err(ContractError::Unauthorized {})
    }

    // Hash the swap parameters to recover and release the swap escrow. If any of the values 
    // are tampered with this will fail.
    let send_asset_hash = compute_send_asset_hash(
        to_account.as_slice(),
        u,
        escrow_amount,
        asset_ref.as_str(),
        block_number_mod
    );

    let fallback_address = release_asset_escrow(deps, send_asset_hash.clone(), escrow_amount, &asset_ref)?;

    // Transfer the escrowed assets to the fallback user.
    let transfer_msg = Asset::from_asset_ref(&deps.as_ref(), &asset_ref)?
        .send_asset(env, escrow_amount, fallback_address.to_string())?;

    let response = match transfer_msg {
        Some(msg) => Response::<CustomMsg>::new().add_message(msg.into_cosmos_vault_msg()),
        None => Response::<CustomMsg>::new()
    };

    Ok(
        response
            .add_event(
                send_asset_failure_event(
                    channel_id,
                    to_account,
                    u,
                    escrow_amount,
                    asset_ref,
                    block_number_mod
                )
            )
    )
}


/// Handle the confirmation of a successful liquidity swap. This function deletes the swap escrow.
/// 
/// **NOTE**: Only the chain interface may invoke this function.
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
pub fn on_send_liquidity_success(
    deps: &mut DepsMut,
    _env: &Env,
    info: &MessageInfo,
    channel_id: Bytes32,
    to_account: Binary,
    u: U256,
    escrow_amount: Uint128,
    block_number_mod: u32
) -> Result<VaultResponse, ContractError> {

    if Some(info.sender.clone()) != CHAIN_INTERFACE.load(deps.storage)? {
        return Err(ContractError::Unauthorized {})
    }

    // Hash the swap parameters to recover and release the swap escrow. If any of the values 
    // are tampered with this will fail.
    let send_liquidity_hash = compute_send_liquidity_hash(
        to_account.as_slice(),
        u,
        escrow_amount,
        block_number_mod
    );

    release_liquidity_escrow(deps, send_liquidity_hash.clone(), escrow_amount)?;

    Ok(
        Response::new()
            .add_event(
                send_liquidity_success_event(
                    channel_id,
                    to_account,
                    u,
                    escrow_amount,
                    block_number_mod
                )
            )
    )
}


/// Handle the confirmation of an unsuccessful liquidity swap. This function deletes the swap escrow
/// and mints the escrowed vault tokens for the fallback account.
/// 
/// **NOTE**: Only the chain interface may invoke this function.
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
pub fn on_send_liquidity_failure(
    deps: &mut DepsMut,
    env: &Env,
    info: &MessageInfo,
    channel_id: Bytes32,
    to_account: Binary,
    u: U256,
    escrow_amount: Uint128,
    block_number_mod: u32
) -> Result<VaultResponse, ContractError> {

    if Some(info.sender.clone()) != CHAIN_INTERFACE.load(deps.storage)? {
        return Err(ContractError::Unauthorized {})
    }

    // Hash the swap parameters to recover and release the swap escrow. If any of the values 
    // are tampered with this will fail.
    let send_liquidity_hash = compute_send_liquidity_hash(
        to_account.as_slice(),
        u,
        escrow_amount,
        block_number_mod
    );

    let fallback_address = release_liquidity_escrow(deps, send_liquidity_hash.clone(), escrow_amount)?;

    // Mint the escrowed vault tokens for the fallback account.
    let mut vault_token = VaultToken::load(&deps.as_ref())?;
    let mint_msg = vault_token.mint(
        deps,
        env,
        info,
        escrow_amount,
        fallback_address.to_string()
    )?;

    let mut response = Response::new();

    if let Some(msg) = mint_msg {
        response = response.add_message(msg.into_cosmos_vault_msg());
    }

    Ok(
        response
            .add_event(
                send_liquidity_failure_event(
                    channel_id,
                    to_account,
                    u,
                    escrow_amount,
                    block_number_mod
                )
            )
    )
}


/// Compute the keccak256 of the provided bytes.
/// 
/// # Arguments:
/// * `bytes` - Bytes to hash.
/// 
fn calc_keccak256(bytes: Vec<u8>) -> Vec<u8> {
    let mut hasher = Keccak256::new();
    hasher.update(bytes);
    hasher.finalize().to_vec()
}


/// Compute the hash of an asset swap.
/// 
/// # Arguments:
/// * `to_account` - The recipient of the swap output. Ensures no collisions between different users.
/// * `u` - The units value of the swap. Used to randomize the hash.
/// * `escrow_amount` - The escrowed asset amount. ! Required to validate the release escrow data.
/// * `asset_ref` - The swap source asset reference. ! Required to validate the release escrow data.
/// * `block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32). Used to randomize the hash.
/// 
pub fn compute_send_asset_hash(
    to_account: &[u8],
    u: U256,
    escrow_amount: Uint128,
    asset_ref: &str,
    block_number_mod: u32        
) -> Vec<u8> {

    let asset_bytes = asset_ref.as_bytes();

    let mut hash_data: Vec<u8> = Vec::with_capacity(    // Initialize vec with the specified capacity to avoid reallocations
        to_account.len()
            + 32
            + 16
            + asset_bytes.len()
            + 4
    );

    hash_data.extend_from_slice(to_account);
    hash_data.extend_from_slice(&u.to_be_bytes());
    hash_data.extend_from_slice(&escrow_amount.to_be_bytes());
    hash_data.extend_from_slice(asset_bytes);
    hash_data.extend_from_slice(&block_number_mod.to_be_bytes());
    
    calc_keccak256(hash_data)
}


/// Compute the hash of a liquidity swap.
/// 
/// # Arguments:
/// * `to_account` - The recipient of the swap output. Ensures no collisions between different users.
/// * `u` - The units value of the swap. Used to randomize the hash.
/// * `escrow_amount` - The escrowed liquidity amount. ! Required to validate the release escrow data.
/// * `block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32). Used to randomize the hash.
/// 
pub fn compute_send_liquidity_hash(
    to_account: &[u8],
    u: U256,
    escrow_amount: Uint128,
    block_number_mod: u32        
) -> Vec<u8> {

    let mut hash_data: Vec<u8> = Vec::with_capacity(    // Initialize vec with the specified capacity to avoid reallocations
        to_account.len()
            + 32
            + 16
            + 4
    );

    hash_data.extend_from_slice(to_account);
    hash_data.extend_from_slice(&u.to_be_bytes());
    hash_data.extend_from_slice(&escrow_amount.to_be_bytes());
    hash_data.extend_from_slice(&block_number_mod.to_be_bytes());
    
    calc_keccak256(hash_data)
}


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
            msg: to_json_binary(&ReceiverExecuteMsg::OnCatalystCall {
                purchased_tokens,
                data
            })?,
            funds: vec![]
        }
    ))

}



// Query helpers ****************************************************************************************************************

/// Query the chain interface.
pub fn query_chain_interface(deps: Deps) -> StdResult<ChainInterfaceResponse> {
    Ok(
        ChainInterfaceResponse {
            chain_interface: CHAIN_INTERFACE.load(deps.storage)?
        }
    )
}

/// Query the setup master.
pub fn query_setup_master(deps: Deps) -> StdResult<SetupMasterResponse> {
    Ok(
        SetupMasterResponse {
            setup_master: SETUP_MASTER.load(deps.storage)?
        }
    )
}

/// Query the factory.
pub fn query_factory(deps: Deps) -> StdResult<FactoryResponse> {
    Ok(
        FactoryResponse {
            factory: FACTORY.load(deps.storage)?
        }
    )
}

/// Query the factory owner.
pub fn query_factory_owner(deps: Deps) -> StdResult<FactoryOwnerResponse> {
    Ok(
        FactoryOwnerResponse {
            factory_owner: factory_owner(&deps)
                .map_err(|_| StdError::generic_err("Unable to get factory_owner."))?
        }
    )
}

/// Query if the vault is ready.
pub fn query_ready(deps: Deps) -> StdResult<ReadyResponse> {
    Ok(
        ReadyResponse {
            ready: ready(&deps)?
        }
    )
}

/// Query if the vault has no cross chain interface.
pub fn query_only_local(deps: Deps) -> StdResult<OnlyLocalResponse> {
    Ok(
        OnlyLocalResponse {
            only_local: only_local(&deps)?
        }
    )
}

/// Query the vault's assets.
pub fn query_assets(deps: Deps) -> StdResult<AssetsResponse<Asset>> {
    Ok(
        AssetsResponse {
            assets: VaultAssets::load(&deps)?
                .get_assets()
                .to_owned()
        }
    )
}

/// Query the vault's asset that corresponds to the given asset reference.
/// 
/// # Arguments:
/// * `asset_ref` - The asset reference.
/// 
pub fn query_asset(
    deps: Deps,
    asset_ref: String
) -> StdResult<AssetResponse<Asset>> {
    Ok(
        AssetResponse {
            asset: Asset::from_asset_ref(&deps, &asset_ref)?
        }
    )
}

/// Query the vault's asset that corresponds to the given asset index.
/// 
/// # Arguments:
/// * `asset_index` - The asset index.
/// 
pub fn query_asset_by_index(
    deps: Deps,
    asset_index: u8
) -> StdResult<AssetResponse<Asset>> {

    let asset_ref = VaultAssets::load_refs(&deps)?
        .get(asset_index as usize)
        .ok_or(ContractError::AssetNotFound {})?
        .clone();

    Ok(
        AssetResponse {
            asset: Asset::from_asset_ref(&deps, &asset_ref)?
        }
    )
}

/// Query the weight of an asset.
/// 
/// # Arguments:
/// * `asset_ref` - The reference of the asset of which to get the weight of.
/// 
pub fn query_weight(deps: Deps, asset_ref: String) -> StdResult<WeightResponse> {
    Ok(
        WeightResponse {
            weight: WEIGHTS.load(deps.storage, &asset_ref)?
        }
    )
}

/// Query the vault total supply.
pub fn query_total_supply(deps: Deps) -> StdResult<TotalSupplyResponse> {
    Ok(
        TotalSupplyResponse {
            total_supply: VaultToken::load(&deps)?
                .query_total_supply(&deps)?
        }
    )
}

/// Query an address' vault token balance.
pub fn query_balance(deps: Deps, address: String) -> StdResult<BalanceResponse> {
    Ok(
        BalanceResponse {
            balance: VaultToken::load(&deps)?
                .query_balance(&deps, address)?
        }
    )
}

/// Query the vault token denom.
#[cfg(feature="asset_native")]
pub fn query_vault_token_denom(deps: Deps) -> StdResult<VaultTokenDenomResponse> {
    Ok(
        VaultTokenDenomResponse {
            denom: VaultToken::load(&deps)?
                .denom().to_string()
        }
    )
}

/// Query the vault fee.
pub fn query_vault_fee(deps: Deps) -> StdResult<VaultFeeResponse> {
    Ok(
        VaultFeeResponse {
            fee: VAULT_FEE.load(deps.storage)?
        }
    )
}

/// Query the governance fee share.
pub fn query_governance_fee_share(deps: Deps) -> StdResult<GovernanceFeeShareResponse> {
    Ok(
        GovernanceFeeShareResponse {
            fee: GOVERNANCE_FEE_SHARE.load(deps.storage)?
        }
    )
}

/// Query the fee administrator.
pub fn query_fee_administrator(deps: Deps) -> StdResult<FeeAdministratorResponse> {
    Ok(
        FeeAdministratorResponse {
            administrator: FEE_ADMINISTRATOR.load(deps.storage)?
        }
    )
}

/// Query the total escrowed amount of an asset.
/// 
/// # Arguments:
/// * `asset_ref` - The reference of the asset of which to get the total escrowed amount.
/// 
pub fn query_total_escrowed_asset(deps: Deps, asset_ref: String) -> StdResult<TotalEscrowedAssetResponse> {
    Ok(
        TotalEscrowedAssetResponse {
            amount: TOTAL_ESCROWED_ASSETS.load(deps.storage, &asset_ref)?
        }
    )
}

/// Query the total escrowed liquidity amount.
pub fn query_total_escrowed_liquidity(deps: Deps) -> StdResult<TotalEscrowedLiquidityResponse> {
    Ok(
        TotalEscrowedLiquidityResponse {
            amount: TOTAL_ESCROWED_LIQUIDITY.load(deps.storage)?
        }
    )
}

/// Query an asset escrow.
/// 
/// # Arguments:
/// * `hash` - The id of the queried escrow.
/// 
pub fn query_asset_escrow(deps: Deps, hash: Binary) -> StdResult<AssetEscrowResponse> {
    Ok(
        AssetEscrowResponse {
            fallback_account: ASSET_ESCROWS.may_load(deps.storage, hash.0)?
        }
    )
}

/// Query an liquidity escrow.
/// 
/// # Arguments:
/// * `hash` - The id of the queried escrow.
/// 
pub fn query_liquidity_escrow(deps: Deps, hash: Binary) -> StdResult<LiquidityEscrowResponse> {
    Ok(
        LiquidityEscrowResponse {
            fallback_account: LIQUIDITY_ESCROWS.may_load(deps.storage, hash.0)?
        }
    )
}

/// Query the state of a vault connection.
/// 
/// # Arguments:
/// * `channel_id` - The channel id which connects the vault.
/// * `vault` - The remote vault address (Catalyst encoded).
/// 
pub fn query_vault_connection_state(deps: Deps, channel_id: &Bytes32, vault: Binary) -> StdResult<VaultConnectionStateResponse> {
    Ok(
        VaultConnectionStateResponse {
            state: VAULT_CONNECTIONS.may_load(deps.storage, (channel_id.as_slice(), vault.0))?.unwrap_or(false)
        }
    )
}