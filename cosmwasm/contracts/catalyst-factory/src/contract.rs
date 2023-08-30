#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128, CosmosMsg, to_binary, StdError, SubMsg, Reply, SubMsgResult, StdResult, Uint64, Deps, Binary};
use cw0::parse_reply_instantiate_data;
use cw2::set_contract_version;
use catalyst_vault_common::asset::{Asset, VaultAssets, VaultAssetsTrait};

use crate::error::ContractError;
use crate::event::deploy_vault_event;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, OwnerResponse, DefaultGovernanceFeeShareResponse};
use crate::state::{set_default_governance_fee_share_unchecked, owner, default_governance_fee_share, save_deploy_vault_reply_args, DeployVaultReplyArgs, DEPLOY_VAULT_REPLY_ID, get_deploy_vault_reply_args, set_owner_unchecked, set_default_governance_fee_share, DEFAULT_GOVERNANCE_FEE_SHARE, update_owner};

// Version information
const CONTRACT_NAME: &str = "catalyst-vault-factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");



// Instantiation **********************************************************************************

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let set_owner_event = set_owner_unchecked(deps.branch(), info.sender)?;

    let set_default_fee_event = set_default_governance_fee_share_unchecked(
        &mut deps,
        msg.default_governance_fee_share
    )?;

    Ok(
        Response::new()
            .add_event(set_owner_event)
            .add_event(set_default_fee_event)
    )
}



// Execution **************************************************************************************

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {

    match msg {
        ExecuteMsg::DeployVault {
            vault_code_id,
            assets,
            assets_balances,
            weights,
            amplification,
            vault_fee,
            name,
            symbol,
            chain_interface
        } => execute_deploy_vault(
            deps,
            env,
            info,
            vault_code_id,
            assets,
            assets_balances,
            weights,
            amplification,
            vault_fee,
            name,
            symbol,
            chain_interface               
        ),

        ExecuteMsg::SetDefaultGovernanceFeeShare {
            fee
        } => execute_set_default_governance_fee_share(
            deps,
            info,
            fee
        ),

        // Ownership msgs
        ExecuteMsg::TransferOwnership {
            new_owner
        } => execute_transfer_ownership(
            deps,
            info,
            new_owner
        )
    }
}


/// Deploy a new vault (permissionless).
/// 
/// NOTE: The deployer must set approvals for this contract of the assets to be deposited on the newly created vault.
/// 
/// # Arguments
/// 
/// * `vault_code_id` - The code id of the *stored* contract with which to deploy the new vault.
/// * `assets` - A list of the assets that are to be supported by the vault.
/// * `assets_balances` - The asset balances that are going to be deposited on the vault.
/// * `weights` - The weights applied to the assets.
/// * `amplification` - The amplification value applied to the vault.
/// * `vault_fee` - The vault fee (18 decimals).
/// * `name` - The name of the vault token.
/// * `symbol` - The symbol of the vault token.
/// * `chain_interface` - The interface used for cross-chain swaps. It can be set to None to disable cross-chain swaps.
/// 
fn execute_deploy_vault(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    vault_code_id: u64,
    assets: Vec<Asset>,
    assets_balances: Vec<Uint128>,
    weights: Vec<Uint128>,
    amplification: Uint64,
    vault_fee: Uint64,
    name: String,
    symbol: String,
    chain_interface: Option<String>
) -> Result<Response, ContractError> {

    // Verify the correctness of the arguments
    if
        assets.len() == 0
        || weights.len() != assets.len()
        || assets_balances.len() != assets.len()
    {
        return Err(StdError::generic_err("Invalid asset/balances/weights count.").into());
    }

    // Handle asset transfer from the deployer to the factory
    let assets = VaultAssets::new(assets);
    let transfer_msgs = assets.receive_assets(
        &env,
        &info,
        assets_balances.clone()
    )?;

    // Save the required args to initalize the vault curves after the vault instantiation (used on 'reply').
    // ! This is set to only work if data is NOT present on the store (prevent a reentrancy attack).
    // This cannot be abused to 'lock' the contract from deploying vaults (i.e. somehow make it so that
    // data is left on the store), as:
    // - If the `instantiation` msg fails, the whole transaction is reverted (as the submessage `reply`
    //   field is set to `ReplyOn::Success`), and thus no data will be left on the store.
    // - The `reply` handler *always* clears out the store.
    // - If the `reply` handler fails, the whole transaction is reverted, and thus no data will be left
    //   on the store.
    save_deploy_vault_reply_args(
        deps.branch(),
        DeployVaultReplyArgs {
            vault_code_id: vault_code_id.clone(),
            assets: assets.get_assets().to_owned(),
            assets_balances,
            weights,
            amplification,
            chain_interface: chain_interface.clone(),
            depositor: info.sender.clone()
        }
    )?;

    // Create the msg to instantiate the new vault
    let instantiate_vault_submsg = SubMsg::reply_on_success(
        cosmwasm_std::WasmMsg::Instantiate {
            admin: None,            // ! The vault should NOT be upgradable.
            code_id: vault_code_id,
            msg: to_binary(&catalyst_vault_common::msg::InstantiateMsg {
                name,
                symbol: symbol.clone(),
                chain_interface: chain_interface.clone(),
                vault_fee,
                governance_fee_share: default_governance_fee_share(deps.as_ref())?,
                fee_administrator: owner(deps.as_ref())?
                    .ok_or(ContractError::NoOwner {})?.to_string(), // NOTE: with the current implementation there will *always* 
                                                                    // be an owner, as it is not possible to renounce the ownership
                                                                    // of the contract (or to set it to an invalid address).
                setup_master: info.sender.to_string()
            })?,
            funds: vec![],
            label: symbol.to_string()
        },
        DEPLOY_VAULT_REPLY_ID
    );
    
    Ok(
        Response::new()
            .add_messages(transfer_msgs)
            .add_submessage(instantiate_vault_submsg)
    )
}


/// Modify the default governance fee share
/// 
/// # Arguments
/// 
/// * `fee` - The new governance fee share (18 decimals).
/// 
fn execute_set_default_governance_fee_share(
    mut deps: DepsMut,
    info: MessageInfo,
    fee: Uint64
) -> Result<Response, ContractError> {
    set_default_governance_fee_share(&mut deps, info, fee)
}


/// Transfer the ownership of the factory.
/// 
/// # Arguments
/// 
/// * `new_owner` - The new owner of the contract. Must be a valid address.
/// 
fn execute_transfer_ownership(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: String
) -> Result<Response, ContractError> {
    update_owner(deps, info, new_owner)
}



// Reply ******************************************************************************************

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    deps: DepsMut,
    env: Env,
    reply: Reply
) -> Result<Response, ContractError> {
    match reply.id {
        DEPLOY_VAULT_REPLY_ID => match reply.result {
            SubMsgResult::Ok(_) => handle_deploy_vault_reply(deps, env, reply),
            SubMsgResult::Err(_) => {       // Reply is set to 'on_success', so the code should never reach this point
                Err(StdError::GenericErr { msg: "Vault deployment failed.".to_string() }.into())
            }
        },
        _ => Err(ContractError::UnknownReplyId { id: reply.id }),
    }
}


/// Setup the newly deployed vault after instantiation.
fn handle_deploy_vault_reply(
    deps: DepsMut,
    env: Env,
    reply: Reply
) -> Result<Response, ContractError> {

    // Get the deployed vault contract address
    let vault_address = parse_reply_instantiate_data(reply)
        .map_err(|_| StdError::generic_err("Error deploying vault contract"))?
        .contract_address;

    // Load the deploy vault args
    // ! It is very important to clear out the 'args' store at this point, as otherwise the factory
    // ! would become 'locked', and no further vault deployments would be possible.
    let deploy_args = get_deploy_vault_reply_args(deps)?;

    // Handle asset transfer from the factory to the vault
    let assets = VaultAssets::new(deploy_args.assets.clone());
    let transfer_msgs = assets.send_assets(
        &env,
        deploy_args.assets_balances.clone(),
        vault_address.clone()
    )?;

    // Build a message to invoke the initialization of the vault swap curves
    let initialize_swap_curves_msg = CosmosMsg::Wasm(
        cosmwasm_std::WasmMsg::Execute {
            contract_addr: vault_address.clone(),
            msg: to_binary(&catalyst_vault_common::msg::ExecuteMsg::<()>::InitializeSwapCurves {
                assets: deploy_args.assets.clone(),
                weights: deploy_args.weights,
                amp: deploy_args.amplification,
                depositor: deploy_args.depositor.to_string()
            })?,
            funds: vec![]
        }
    );


    Ok(
        Response::new()
            .set_data(to_binary(&vault_address)?)   // Return the deployed vault address.
            .add_messages(transfer_msgs)
            .add_message(initialize_swap_curves_msg)
            .add_event(
                deploy_vault_event(
                    deploy_args.vault_code_id,
                    deploy_args.chain_interface,
                    deploy_args.depositor.to_string(),
                    vault_address,
                    deploy_args.assets,
                    deploy_args.amplification
                )
            )
    )

}



// Query ******************************************************************************************

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Owner {} => to_binary(&query_owner(deps)?),
        QueryMsg::DefaultGovernanceFeeShare {} => to_binary(&query_default_governance_fee_share(deps)?)
    }
}


/// Query the factory owner.
fn query_owner(deps: Deps) -> StdResult<OwnerResponse> {
    Ok(
        OwnerResponse {
            owner: owner(deps)?
        }
    )
}


/// Query the default governance fee share.
fn query_default_governance_fee_share(deps: Deps) -> StdResult<DefaultGovernanceFeeShareResponse> {
    Ok(
        DefaultGovernanceFeeShareResponse {
            fee: DEFAULT_GOVERNANCE_FEE_SHARE.load(deps.storage)?
        }
    )
}




#[cfg(test)]
mod catalyst_vault_factory_tests {
    use std::str::FromStr;

    use cosmwasm_std::{Addr, Uint64, Uint128, Event, StdError, Attribute, WasmMsg, to_binary};
    use cw20::TokenInfoResponse;
    use cw_multi_test::{App, Executor, ContractWrapper};
    use test_helpers::{env::CustomTestEnv, asset::CustomTestAsset};

    use crate::{msg::{InstantiateMsg, QueryMsg, OwnerResponse, ExecuteMsg, DefaultGovernanceFeeShareResponse}, state::MAX_DEFAULT_GOVERNANCE_FEE_SHARE, error::ContractError};

    use catalyst_vault_common::{msg::{ChainInterfaceResponse, FactoryResponse, SetupMasterResponse, AssetsResponse, WeightResponse, VaultFeeResponse, GovernanceFeeShareResponse, FeeAdministratorResponse}, event::format_vec_for_event, asset::Asset};
    use mock_vault::msg::QueryMsg as MockVaultQueryMsg;

    #[cfg(feature="asset_native")]
    use test_helpers::asset::TestNativeAsset as TestAsset;
    #[cfg(feature="asset_native")]
    use test_helpers::env::env_native_asset::TestNativeAssetEnv as TestEnv;
    
    #[cfg(feature="asset_cw20")]
    use test_helpers::asset::TestCw20Asset as TestAsset;
    #[cfg(feature="asset_cw20")]
    use test_helpers::env::env_cw20_asset::TestCw20AssetEnv as TestEnv;

    const GOVERNANCE: &str = "governance_addr";
    const SETUP_MASTER: &str = "setup_master_addr";

    const TEST_VAULT_FEE: Uint64 = Uint64::new(999999999u64);
    const TEST_GOVERNANCE_FEE: Uint64 = Uint64::new(111111111u64);


    fn mock_factory_contract(app: &mut App) -> u64 {
        app.store_code(
            Box::new(
                ContractWrapper::new(
                    crate::contract::execute,
                    crate::contract::instantiate,
                    crate::contract::query,
                ).with_reply(crate::contract::reply)
            )
        )
    }


    fn mock_vault_contract(app: &mut App) -> u64 {
        app.store_code(
            Box::new(
                ContractWrapper::new(
                    mock_vault::contract::execute,
                    mock_vault::contract::instantiate,
                    mock_vault::contract::query,
                )
            )
        )
    }


    fn mock_factory(app: &mut App) -> Addr {
        
        let factory_code_id = mock_factory_contract(app);
        
        app.instantiate_contract(
            factory_code_id,
            Addr::unchecked(GOVERNANCE),
            &InstantiateMsg { default_governance_fee_share: TEST_GOVERNANCE_FEE },
            &[],
            "catalyst-factory",
            None
        ).unwrap()

    }




    #[test]
    fn test_instantiate() {

        let mut app = App::default();
    
        // 'Deploy' the contract
        let code_id = mock_factory_contract(&mut app);

        let default_governance_fee_share = Uint64::new(10101u64);



        // Tested action
        let factory = app.instantiate_contract(
            code_id,
            Addr::unchecked(GOVERNANCE),
            &InstantiateMsg { default_governance_fee_share },
            &[],
            "catalyst-factory",
            None
        ).unwrap();



        // Verify the governance fee is set correctly
        let queried_default_governance_fee_share = app.wrap()
            .query_wasm_smart::<DefaultGovernanceFeeShareResponse>(factory, &QueryMsg::DefaultGovernanceFeeShare {})
            .unwrap()
            .fee;

        assert_eq!(
            queried_default_governance_fee_share,
            default_governance_fee_share
        )

    }


    #[test]
    fn test_instantiate_event() {

        let mut app = App::default();
    
        // 'Deploy' the contract
        let code_id = mock_factory_contract(&mut app);

        let default_governance_fee_share = Uint64::new(10101u64);



        // Tested action
        let wasm_instantiate_msg = WasmMsg::Instantiate {
            admin: None,
            code_id: code_id,
            msg: to_binary(&InstantiateMsg { default_governance_fee_share }).unwrap(),
            funds: vec![],
            label: "catalyst-factory".into(),
        };

        let response = app.execute(
            Addr::unchecked(GOVERNANCE),
            wasm_instantiate_msg.into()
        ).unwrap();



        // Check the events
        let owner_event = response.events[1].clone();
        assert_eq!(owner_event.ty, "wasm-set-owner");
        assert_eq!(
            owner_event.attributes[1],
            Attribute::new("account", GOVERNANCE)
        );

        let default_governance_fee_event = response.events[2].clone();
        assert_eq!(default_governance_fee_event.ty, "wasm-set-default-governance-fee-share");
        assert_eq!(
            default_governance_fee_event.attributes[1],
            Attribute::new("fee", default_governance_fee_share.to_string())
        );

    }


    #[test]
    fn test_max_default_governance_fee_share_constant() {

        let max_fee = (MAX_DEFAULT_GOVERNANCE_FEE_SHARE.u64() as f64) / 1e18;

        assert!( max_fee <= 1.);
    }


    #[test]
    fn test_instantiate_governance_fee_share_max() {

        let mut app = App::default();
    
        // 'Deploy' the contract
        let code_id = mock_factory_contract(&mut app);



        // Tested action 1: Set max fee
        let default_governance_fee_share = MAX_DEFAULT_GOVERNANCE_FEE_SHARE;
        app.instantiate_contract(
            code_id,
            Addr::unchecked(GOVERNANCE),
            &InstantiateMsg { default_governance_fee_share },
            &[],
            "catalyst-factory",
            None
        ).unwrap();     // Make sure the transaction passes




        // Tested action 2: Set fee too large
        let default_governance_fee_share = MAX_DEFAULT_GOVERNANCE_FEE_SHARE + Uint64::one();  // ! Governance fee too large
        let response_result = app.instantiate_contract(
            code_id,
            Addr::unchecked(GOVERNANCE),
            &InstantiateMsg { default_governance_fee_share },
            &[],
            "catalyst-factory",
            None
        );

        // Make sure the transaction does not pass
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidDefaultGovernanceFeeShare { requested_fee, max_fee }
                if requested_fee == default_governance_fee_share && max_fee == MAX_DEFAULT_GOVERNANCE_FEE_SHARE
        ));

    }


    #[test]
    fn test_deploy_vault() {

        // Use the 'TestEnv' helper for 'deploy_vault' tests to handle asset transfers
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
    
        // Instantiate factory
        let factory = mock_factory(test_env.get_app());

        // Deploy vault contract
        let vault_code_id = mock_vault_contract(test_env.get_app());

        // Define vault config
        let vault_assets = test_env.get_assets()[..3].to_vec();
        let vault_initial_balances = vec![Uint128::from(1u64), Uint128::from(2u64), Uint128::from(3u64)];
        let vault_weights = vec![Uint128::one(), Uint128::one(), Uint128::one()];



        // Tested action
        let response = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            factory.clone(),
            &crate::msg::ExecuteMsg::DeployVault {
                vault_code_id,
                assets: vault_assets.iter().map(|asset| asset.into_vault_asset()).collect(),
                assets_balances: vault_initial_balances.clone(),
                weights: vault_weights.clone(),
                amplification: Uint64::new(1000000000000000000u64),
                vault_fee: TEST_VAULT_FEE,
                name: "TestVault".to_string(),
                symbol: "TP".to_string(),
                chain_interface: Some("chain_interface".to_string())
            },
            vault_assets.clone(),
            vault_initial_balances.clone()
        ).unwrap();     // Make sure the transaction succeeds



        let deploy_vault_event = response.events
            .iter()
            .find(|event| event.ty == "wasm-deploy-vault".to_string())
            .unwrap()
            .clone();

        let vault = get_response_attribute::<String>(deploy_vault_event, "vault_address").unwrap();

        // Verify the assets have been transferred to the vault
        vault_assets
            .iter()
            .zip(&vault_initial_balances)
            .for_each(|(asset, balance)| {
                let queried_balance: Uint128 = asset.query_balance(test_env.get_app(), vault.clone());

                assert_eq!(
                    queried_balance,
                    balance
                );
            });


        // Verify the deployed vault has the 'factory' set
        let queried_factory = test_env.get_app().wrap().query_wasm_smart::<FactoryResponse>(
            vault.clone(),
            &MockVaultQueryMsg::Factory {}
        ).unwrap().factory;

        assert_eq!(
            queried_factory,
            factory
        );


        // Verify the deployed vault has the 'setup master' set
        let queried_setup_master = test_env.get_app().wrap().query_wasm_smart::<SetupMasterResponse>(
            vault.clone(),
            &MockVaultQueryMsg::SetupMaster {}
        ).unwrap().setup_master;

        assert_eq!(
            queried_setup_master,
            Some(Addr::unchecked(SETUP_MASTER))
        );


        // Verify the deployed vault has the 'chain interface' set
        let queried_interface = test_env.get_app().wrap().query_wasm_smart::<ChainInterfaceResponse>(
            vault.clone(),
            &MockVaultQueryMsg::ChainInterface {}
        ).unwrap().chain_interface;

        assert_eq!(
            queried_interface,
            Some(Addr::unchecked("chain_interface"))
        );


        // Verify the deployed vault has the 'assets' set
        let queried_assets = test_env.get_app().wrap().query_wasm_smart::<AssetsResponse>(
            vault.clone(),
            &MockVaultQueryMsg::Assets {}
        ).unwrap().assets;

        assert_eq!(
            queried_assets
                .iter()
                .map(|asset| TestAsset::from_vault_asset(asset))
                .collect::<Vec<TestAsset>>(),
            vault_assets
        );


        // Verify the deployed vault has the 'weights' set
        vault_assets
            .iter()
            .zip(&vault_weights)
            .for_each(|(asset, weight)| {
                let queried_weight = test_env.get_app().wrap().query_wasm_smart::<WeightResponse>(
                    vault.clone(),
                    &MockVaultQueryMsg::Weight { asset: asset.get_asset_ref() }
                ).unwrap().weight;
        
                assert_eq!(
                    queried_weight,
                    weight
                );
            });


        // Verify the deployed vault has the 'vault_fee' set
        let queried_vault_fee = test_env.get_app().wrap().query_wasm_smart::<VaultFeeResponse>(
            vault.clone(),
            &MockVaultQueryMsg::VaultFee {}
        ).unwrap().fee;

        assert_eq!(
            queried_vault_fee,
            TEST_VAULT_FEE
        );


        // Verify the deployed vault has the 'governance_fee_share' set
        let queried_governance_fee_share = test_env.get_app().wrap().query_wasm_smart::<GovernanceFeeShareResponse>(
            vault.clone(),
            &MockVaultQueryMsg::GovernanceFeeShare {}
        ).unwrap().fee;

        assert_eq!(
            queried_governance_fee_share,
            TEST_GOVERNANCE_FEE
        );


        // Verify the deployed vault has the 'fee_administrator' set
        let queried_fee_administrator = test_env.get_app().wrap().query_wasm_smart::<FeeAdministratorResponse>(
            vault.clone(),
            &MockVaultQueryMsg::FeeAdministrator {}
        ).unwrap().administrator;

        assert_eq!(
            queried_fee_administrator,
            GOVERNANCE
        );


        // Verify the deployed vault has the 'name' and 'symbol' set
        let queried_token_info = test_env.get_app().wrap().query_wasm_smart::<TokenInfoResponse>(
            vault.clone(),
            &MockVaultQueryMsg::TokenInfo {}
        ).unwrap();

        assert_eq!(
            queried_token_info.name,
            "TestVault"
        );

        assert_eq!(
            queried_token_info.symbol,
            "TP"
        );

    }


    #[test]
    fn test_deploy_vault_event() {

        // Use the 'TestEnv' helper for 'deploy_vault' tests to handle asset transfers
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
    
        // Instantiate factory
        let factory = mock_factory(test_env.get_app());

        // Deploy vault contract
        let vault_code_id = mock_vault_contract(test_env.get_app());

        // Define vault config
        let vault_assets = test_env.get_assets()[..3].to_vec();
        let vault_initial_balances = vec![Uint128::from(1u64), Uint128::from(2u64), Uint128::from(3u64)];
        let vault_weights = vec![Uint128::one(), Uint128::one(), Uint128::one()];



        // Tested action
        let response = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            factory.clone(),
            &crate::msg::ExecuteMsg::DeployVault {
                vault_code_id,
                assets: vault_assets.iter().map(|asset| asset.into_vault_asset()).collect(),
                assets_balances: vault_initial_balances.clone(),
                weights: vault_weights.clone(),
                amplification: Uint64::new(1000000000000000000u64),
                vault_fee: TEST_VAULT_FEE,
                name: "TestVault".to_string(),
                symbol: "TP".to_string(),
                chain_interface: Some("chain_interface".to_string())
            },
            vault_assets.clone(),
            vault_initial_balances.clone()
        ).unwrap();     // Make sure the transaction succeeds



        // Check the event
        let deploy_vault_event = response.events
            .iter()
            .find(|event| event.ty == "wasm-deploy-vault".to_string())
            .unwrap()
            .clone();

        assert_eq!(
            deploy_vault_event.attributes[1],
            Attribute::new("vault_code_id", vault_code_id.to_string())
        );
        assert_eq!(
            deploy_vault_event.attributes[2],
            Attribute::new("chain_interface", "chain_interface".to_string())
        );
        assert_eq!(
            deploy_vault_event.attributes[3],
            Attribute::new("deployer", SETUP_MASTER.to_string())
        );
        
        // NOTE: 'vault_address' is indirectly checked on `test_deploy_vault`.

        assert_eq!(
            deploy_vault_event.attributes[5],
            Attribute::new("assets", format_vec_for_event(
                vault_assets.iter().map(|asset| asset.into_vault_asset().to_string()).collect()
            ))
        );
        assert_eq!(
            deploy_vault_event.attributes[6],
            Attribute::new("k", Uint64::new(1000000000000000000u64).to_string())
        );

    }
    

    #[test]
    fn test_deploy_vault_no_interface_event() {

        // Use the 'TestEnv' helper for 'deploy_vault' tests to handle asset transfers
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
    
        // Instantiate factory
        let factory = mock_factory(test_env.get_app());

        // Deploy vault contract
        let vault_code_id = mock_vault_contract(test_env.get_app());

        // Define vault config
        let vault_assets = test_env.get_assets()[..3].to_vec();
        let vault_initial_balances = vec![Uint128::from(1u64), Uint128::from(2u64), Uint128::from(3u64)];
        let vault_weights = vec![Uint128::one(), Uint128::one(), Uint128::one()];



        // Tested action
        let response = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            factory.clone(),
            &crate::msg::ExecuteMsg::DeployVault {
                vault_code_id,
                assets: vault_assets.iter().map(|asset| asset.into_vault_asset()).collect(),
                assets_balances: vault_initial_balances.clone(),
                weights: vault_weights.clone(),
                amplification: Uint64::new(1000000000000000000u64),
                vault_fee: TEST_VAULT_FEE,
                name: "TestVault".to_string(),
                symbol: "TP".to_string(),
                chain_interface: None   // ! Interface set to 'None'
            },
            vault_assets.clone(),
            vault_initial_balances.clone()
        ).unwrap();     // Make sure the transaction succeeds



        // Check the event
        let deploy_vault_event = response.events
            .iter()
            .find(|event| event.ty == "wasm-deploy-vault".to_string())
            .unwrap()
            .clone();

        assert_eq!(
            deploy_vault_event.attributes[2],
            Attribute::new("chain_interface", "null".to_string())
        );

    }


    #[test]
    fn test_deploy_vault_no_assets() {

        // Use the 'TestEnv' helper for 'deploy_vault' tests to handle asset transfers
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
    
        // Instantiate factory
        let factory = mock_factory(test_env.get_app());

        // Deploy vault contract
        let vault_code_id = mock_vault_contract(test_env.get_app());

        // Define vault config
        let vault_assets: Vec<Asset> = vec![];              // ! no assets
        let vault_initial_balances = vec![];
        let vault_weights = vec![];



        // Tested action
        let response_result = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            factory,
            &crate::msg::ExecuteMsg::DeployVault {
                vault_code_id,
                assets: vault_assets,
                assets_balances: vault_initial_balances,
                weights: vault_weights,
                amplification: Uint64::new(1000000000000000000u64),
                vault_fee: Uint64::zero(),
                name: "TestVault".to_string(),
                symbol: "TP".to_string(),
                chain_interface: None
            },
            vec![],
            vec![]
        );

        

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Std(StdError::GenericErr { msg })
                if msg == "Invalid asset/balances/weights count."
        ))

    }


    #[test]
    fn test_deploy_vault_invalid_weights_balances() {

        // Use the 'TestEnv' helper for 'deploy_vault' tests to handle asset transfers
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
    
        // Instantiate factory
        let factory = mock_factory(test_env.get_app());

        // Deploy vault contract
        let vault_code_id = mock_vault_contract(test_env.get_app());

        // Define vault config
        let vault_assets = test_env.get_assets()[..3].to_vec();
        let vault_initial_balances = vec![Uint128::from(1u64), Uint128::from(2u64), Uint128::from(3u64)];
        let vault_weights = vec![Uint128::one(), Uint128::one(), Uint128::one()];



        // Tested action 1: len(assets_balances) != len(assets)
        let response_result = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            factory.clone(),
            &crate::msg::ExecuteMsg::DeployVault {
                vault_code_id,
                assets: vault_assets.iter().map(|asset| asset.into_vault_asset()).collect(),
                assets_balances: vault_initial_balances[..2].to_vec(),   // ! Only 2 balances are provided 
                weights: vault_weights.clone(),
                amplification: Uint64::new(1000000000000000000u64),
                vault_fee: Uint64::zero(),
                name: "TestVault".to_string(),
                symbol: "TP".to_string(),
                chain_interface: None
            },
            vault_assets.clone(),
            vault_initial_balances.clone()
        );

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Std(StdError::GenericErr { msg })
                if msg == "Invalid asset/balances/weights count."
        ));



        // Tested action 2: len(weights) != len(assets)
        let response_result = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            factory.clone(),
            &crate::msg::ExecuteMsg::DeployVault {
                vault_code_id,
                assets: vault_assets.iter().map(|asset| asset.into_vault_asset()).collect(),
                assets_balances: vault_initial_balances.clone(),
                weights: vault_weights[..2].to_vec(),   // ! Only 2 weights are provided 
                amplification: Uint64::new(1000000000000000000u64),
                vault_fee: Uint64::zero(),
                name: "TestVault".to_string(),
                symbol: "TP".to_string(),
                chain_interface: None
            },
            vault_assets.clone(),
            vault_initial_balances.clone()
        );

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Std(StdError::GenericErr { msg })
                if msg == "Invalid asset/balances/weights count."
        ));



        // Make sure the transaction does succeed with valid params
        test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            factory,
            &crate::msg::ExecuteMsg::DeployVault {
                vault_code_id,
                assets: vault_assets.iter().map(|asset| asset.into_vault_asset()).collect(),
                assets_balances: vault_initial_balances.clone(),
                weights: vault_weights,
                amplification: Uint64::new(1000000000000000000u64),
                vault_fee: Uint64::zero(),
                name: "TestVault".to_string(),
                symbol: "TP".to_string(),
                chain_interface: None
            },
            vault_assets.clone(),
            vault_initial_balances.clone()
        ).unwrap();     // ! Make sure the transaction succeeds

    }




    #[test]
    fn test_change_default_governance_fee_share() {

        let mut app = App::default();
    
        // 'Deploy' the contract
        let code_id = mock_factory_contract(&mut app);

        let initial_default_governance_fee_share = Uint64::new(10101u64);
        let new_default_governance_fee_share = Uint64::new(20202u64);

        // Instantiate factory
        let factory = app.instantiate_contract(
            code_id,
            Addr::unchecked(GOVERNANCE),
            &InstantiateMsg { default_governance_fee_share: initial_default_governance_fee_share },
            &[],
            "catalyst-factory",
            None
        ).unwrap();



        // Tested action
        let response = app.execute_contract(
            Addr::unchecked(GOVERNANCE),
            factory.clone(),
            &ExecuteMsg::<()>::SetDefaultGovernanceFeeShare { fee: new_default_governance_fee_share },
            &[]
        ).unwrap();     // Make sure the transaction passes



        // Verify the event
        let event = response.events[1].clone();

        assert_eq!(event.ty, "wasm-set-default-governance-fee-share");

        assert_eq!(
            event.attributes[1],
            Attribute::new("fee", new_default_governance_fee_share.to_string())
        );

        // Verify the governance fee is set correctly
        let queried_default_governance_fee_share = app.wrap()
            .query_wasm_smart::<DefaultGovernanceFeeShareResponse>(factory, &QueryMsg::DefaultGovernanceFeeShare {})
            .unwrap()
            .fee;

        assert_eq!(
            queried_default_governance_fee_share,
            new_default_governance_fee_share
        )

    }

    #[test]
    fn test_change_default_governance_fee_share_no_auth() {

        let mut app = App::default();
    
        // 'Deploy' the contract
        let code_id = mock_factory_contract(&mut app);

        let initial_default_governance_fee_share = Uint64::new(10101u64);
        let new_default_governance_fee_share = Uint64::new(20202u64);

        // Instantiate factory
        let factory = app.instantiate_contract(
            code_id,
            Addr::unchecked(GOVERNANCE),
            &InstantiateMsg { default_governance_fee_share: initial_default_governance_fee_share },
            &[],
            "catalyst-factory",
            None
        ).unwrap();



        // Tested action
        let response_result = app.execute_contract(
            Addr::unchecked("not_governance"),      // ! Not the contract owner (i.e. GOVERNANCE)
            factory.clone(),
            &ExecuteMsg::<()>::SetDefaultGovernanceFeeShare { fee: new_default_governance_fee_share },
            &[]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ))

    }


    // Ownership tests
    #[test]
    fn test_owner_is_set_on_instantiation_and_query() {

        let mut app = App::default();
    
        // 'Deploy' the contract
        let code_id = mock_factory_contract(&mut app);



        // Tested action
        let factory = app.instantiate_contract(
            code_id,
            Addr::unchecked(GOVERNANCE),
            &InstantiateMsg { default_governance_fee_share: Uint64::zero() },
            &[],
            "catalyst-factory",
            None
        ).unwrap();     // Make sure the transaction passes



        // Query owner
        let owner_response = app.wrap().query_wasm_smart::<OwnerResponse>(factory, &QueryMsg::Owner {}).unwrap();

        assert_eq!(
            owner_response.owner,
            Some(Addr::unchecked(GOVERNANCE))
        )

    }


    #[test]
    fn test_transfer_ownership_and_query() {

        let mut app = App::default();
    
        // 'Deploy' the contract
        let code_id = mock_factory_contract(&mut app);

        // Instantiate a factory
        let factory = app.instantiate_contract(
            code_id,
            Addr::unchecked(GOVERNANCE),
            &InstantiateMsg { default_governance_fee_share: Uint64::zero() },
            &[],
            "catalyst-factory",
            None
        ).unwrap();     // Make sure the transaction passes

        let new_owner = "new_owner_addr".to_string();



        // Tested action: transfer ownership
        let response = app.execute_contract(
            Addr::unchecked(GOVERNANCE),
            factory.clone(),
            &ExecuteMsg::<()>::TransferOwnership { new_owner: new_owner.clone() },
            &[]
        ).unwrap();     // Make sure the transaction passes



        // Verify the event
        let event = response.events[1].clone();

        assert_eq!(event.ty, "wasm-set-owner");

        assert_eq!(
            event.attributes[1],
            Attribute::new("account", new_owner.to_string())
        );

        // Query owner
        let owner_response = app.wrap().query_wasm_smart::<OwnerResponse>(factory, &QueryMsg::Owner {}).unwrap();

        assert_eq!(
            owner_response.owner,
            Some(Addr::unchecked(new_owner))
        )

    }


    #[test]
    fn test_transfer_ownership_no_auth() {

        let mut app = App::default();
    
        // 'Deploy' the contract
        let code_id = mock_factory_contract(&mut app);

        // Instantiate a factory
        let factory = app.instantiate_contract(
            code_id,
            Addr::unchecked(GOVERNANCE),
            &InstantiateMsg { default_governance_fee_share: Uint64::zero() },
            &[],
            "catalyst-factory",
            None
        ).unwrap();     // Make sure the transaction passes

        let new_owner = "new_owner_addr".to_string();



        // Tested action: transfer ownership
        let response_result = app.execute_contract(
            Addr::unchecked("not-factory-owner"),           // ! Not the factory owner (i.e. GOVERNANCE)
            factory.clone(),
            &ExecuteMsg::<()>::TransferOwnership { new_owner: new_owner.clone() },
            &[]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ))

    }



    // Misc helpers
    
    pub fn get_response_attribute<T: FromStr>(event: Event, attribute: &str) -> Result<T, String> {
        event.attributes
            .iter()
            .find(|attr| attr.key == attribute).ok_or("Attribute not found")?
            .value
            .parse::<T>().map_err(|_| "Parse error".to_string())
    }

}