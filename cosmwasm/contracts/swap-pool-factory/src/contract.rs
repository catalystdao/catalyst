#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128, CosmosMsg, to_binary, StdError, SubMsg, Reply, SubMsgResult, StdResult, Uint64, Deps, Binary};
use cw0::parse_reply_instantiate_data;
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, OwnerResponse, DefaultGovernanceFeeShareResponse};
use crate::state::{set_default_governance_fee_share_unchecked, owner, default_governance_fee_share, save_deploy_vault_reply_args, DeployVaultReplyArgs, DEPLOY_VAULT_REPLY_ID, load_deploy_vault_reply_args, set_owner_unchecked, set_default_governance_fee_share, DEFAULT_GOVERNANCE_FEE_SHARE, set_owner};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:catalyst-vault-factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    set_owner_unchecked(&mut deps, info.sender)?;

    let set_default_fee_event = set_default_governance_fee_share_unchecked(
        &mut deps,
        msg.default_governance_fee_share
    )?;

    Ok(
        Response::new()
            .add_event(set_default_fee_event)
    )
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
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
            pool_fee,
            name,
            symbol,
            chain_interface
        } => execute_deploy_vault(
            deps,
            info,
            vault_code_id,
            assets,
            assets_balances,
            weights,
            amplification,
            pool_fee,
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

fn execute_deploy_vault(
    mut deps: DepsMut,
    info: MessageInfo,
    vault_contract_id: u64,
    assets: Vec<String>,
    assets_balances: Vec<Uint128>,
    weights: Vec<u64>,
    amplification: u64,
    pool_fee: u64,
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

    // Store the required args to initalize the vault curves for after the vault instantiation (used on 'reply')
    // TODO! verify the safety of passing information to the reply handler in the following way (reentrancy attacks?)
    save_deploy_vault_reply_args(
        deps.branch(),
        DeployVaultReplyArgs {
            assets,
            assets_balances,
            weights,
            amplification,
            depositor: info.sender.clone()
        }
    )?;

    // Create msg to instantiate a new vault
    let instantiate_vault_submsg = SubMsg::reply_on_success(
        cosmwasm_std::WasmMsg::Instantiate {
            admin: None,        //TODO set factory as admin?
            code_id: vault_contract_id,
            msg: to_binary(&swap_pool_common::msg::InstantiateMsg {
                name,
                symbol: symbol.clone(),
                chain_interface: chain_interface.clone(),
                pool_fee,
                governance_fee: default_governance_fee_share(deps.as_ref())?,
                fee_administrator: owner(deps.as_ref())?.ok_or(ContractError::Unauthorized {})?.to_string(),      //TODO error
                setup_master: info.sender.to_string()
            })?,
            funds: vec![],
            label: symbol.to_string()   //TODO review: what to set as 'label'?
        },
        DEPLOY_VAULT_REPLY_ID
    );

    //TODO implement IsCreatedByFactory?
    
    Ok(
        Response::new()
            .add_submessage(instantiate_vault_submsg)
            .add_attribute("vault_contract_id", Uint64::from(vault_contract_id))                //TODO EVM mismatch event name
            .add_attribute("chain_interface", chain_interface.unwrap_or("None".to_string()))    //TODO review 'chain_interface' format
            .add_attribute("deployer", info.sender)
    )
}

fn execute_set_default_governance_fee_share(
    mut deps: DepsMut,
    info: MessageInfo,
    fee: u64
) -> Result<Response, ContractError> {
    set_default_governance_fee_share(&mut deps, info, fee)
}

fn execute_transfer_ownership(
    mut deps: DepsMut,
    info: MessageInfo,
    new_owner: String
) -> Result<Response, ContractError> {
    set_owner(&mut deps, info, new_owner)
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    deps: DepsMut,
    _env: Env,
    reply: Reply
) -> Result<Response, ContractError> {
    match reply.id {
        DEPLOY_VAULT_REPLY_ID => match reply.result {
            SubMsgResult::Ok(_) => handle_deploy_vault_reply(deps, reply),
            SubMsgResult::Err(_) => Err(StdError::GenericErr { msg: "Vault deployment failed.".to_string() }.into())   // Reply is set to on_success, so the code should never reach this point
        },
        _ => Err(ContractError::UnknownReplyId { id: reply.id }),
    }
}

fn handle_deploy_vault_reply(
    deps: DepsMut,
    reply: Reply
) -> Result<Response, ContractError> {

    // Get deployed vault contract address
    let vault_address = parse_reply_instantiate_data(reply).map_err(|_| StdError::generic_err("Error deploying vault contract"))?.contract_address;

    // Load the deploy vault args
    let deploy_args = load_deploy_vault_reply_args(deps.as_ref())?;

    // Build messages to order the transfer of tokens from setup_master to the vault
    let transfer_msgs: Vec<CosmosMsg> = deploy_args.assets.iter()
        .zip(&deploy_args.assets_balances)
        .map(|(asset, balance)| {    // zip: assets_balances.len() == assets.len()
            Ok(CosmosMsg::Wasm(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: asset.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                        owner: deploy_args.depositor.to_string(),
                        recipient: vault_address.clone(),
                        amount: *balance
                    })?,
                    funds: vec![]
                }
            ))
        }).collect::<StdResult<Vec<CosmosMsg>>>()?;

    // Build message to invoke vault initialize swap curves
    let initialize_swap_curves_msg = CosmosMsg::Wasm(
        cosmwasm_std::WasmMsg::Execute {
            contract_addr: vault_address.clone(),
            msg: to_binary(&swap_pool_common::msg::ExecuteMsg::<()>::InitializeSwapCurves {
                assets: deploy_args.assets.clone(),
                weights: deploy_args.weights,
                amp: deploy_args.amplification,
                depositor: deploy_args.depositor.to_string()
            })?,
            funds: vec![]
        }
    );


    //TODO event
    Ok(
        Response::new()
            .add_messages(transfer_msgs)
            .add_message(initialize_swap_curves_msg)
            .add_attribute("vault", vault_address)                                      //TODO EVM mismatch: key name (pool_address)
            .add_attribute("assets", deploy_args.assets.join(", "))
            .add_attribute("amplification", Uint64::from(deploy_args.amplification))    //TODO EVM mismatch: key name (k)
    )

}



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Owner {} => to_binary(&query_owner(deps)?),
        QueryMsg::DefaultGovernanceFeeShare {} => to_binary(&query_default_governance_fee_share(deps)?)
    }
}

fn query_owner(deps: Deps) -> StdResult<OwnerResponse> {
    Ok(
        OwnerResponse {
            owner: owner(deps).map_err(|_| StdError::generic_err("Query owner error."))?    //TODO error
        }
    )
}


fn query_default_governance_fee_share(deps: Deps) -> StdResult<DefaultGovernanceFeeShareResponse> {
    Ok(
        DefaultGovernanceFeeShareResponse {
            fee: DEFAULT_GOVERNANCE_FEE_SHARE.load(deps.storage)?
        }
    )
}


#[cfg(test)]
mod catalyst_swap_pool_factory_tests {
    use std::str::FromStr;

    use cosmwasm_std::{Addr, Uint128, Event, StdError, to_binary};
    use cw20::{TokenInfoResponse, BalanceResponse, Cw20QueryMsg};
    use cw_multi_test::{App, Executor, ContractWrapper};
    use token_helpers::helpers::{deploy_test_tokens, set_token_allowance};

    use crate::{msg::{InstantiateMsg, QueryMsg, OwnerResponse, ExecuteMsg, DefaultGovernanceFeeShareResponse}, state::MAX_GOVERNANCE_FEE_SHARE, error::ContractError};

    use swap_pool_common::msg::{ChainInterfaceResponse, FactoryResponse, SetupMasterResponse, AssetsResponse, WeightsResponse, PoolFeeResponse, GovernanceFeeShareResponse, FeeAdministratorResponse};
    use mock_swap_pool::msg::QueryMsg as MockPoolQueryMsg;

    const GOVERNANCE: &str = "governance_addr";
    const SETUP_MASTER: &str = "setup_master_addr";
    const DEPLOYER: &str = "deployer_addr";

    const TEST_POOL_FEE: u64 = 999999999u64;
    const TEST_GOVERNANCE_FEE: u64 = 111111111u64;


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
                    mock_swap_pool::contract::execute,
                    mock_swap_pool::contract::instantiate,
                    mock_swap_pool::contract::query,
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

        let default_governance_fee_share = 10101u64;



        // Tested action
        let factory = app.instantiate_contract(
            code_id,
            Addr::unchecked(GOVERNANCE),
            &InstantiateMsg { default_governance_fee_share },
            &[],
            "catalyst-factory",
            None
        ).unwrap();



        // TODO verify event

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
    fn test_max_governance_fee_share_constant() {

        let max_fee = (MAX_GOVERNANCE_FEE_SHARE as f64) / 1e18;

        assert!( max_fee <= 1.);
    }


    #[test]
    fn test_instantiate_governance_fee_share_max() {

        let mut app = App::default();
    
        // 'Deploy' the contract
        let code_id = mock_factory_contract(&mut app);



        // Tested action 1: Set max fee
        let default_governance_fee_share = MAX_GOVERNANCE_FEE_SHARE;
        app.instantiate_contract(
            code_id,
            Addr::unchecked(GOVERNANCE),
            &InstantiateMsg { default_governance_fee_share },
            &[],
            "catalyst-factory",
            None
        ).unwrap();     // Make sure the transaction passes




        // Tested action 2: Set fee too large
        let default_governance_fee_share = MAX_GOVERNANCE_FEE_SHARE + 1u64;  // ! Governance fee too large
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
                if requested_fee == default_governance_fee_share && max_fee == MAX_GOVERNANCE_FEE_SHARE
        ));

    }


    #[test]
    fn test_deploy_vault() {

        let mut app = App::default();
    
        // Instantiate factory
        let factory = mock_factory(&mut app);

        // Deploy vault contract
        let vault_code_id = mock_vault_contract(&mut app);

        // Define vault config
        let vault_assets = deploy_test_tokens(
            &mut app,
            Addr::unchecked(SETUP_MASTER),
            None,
            None
        );
        let vault_initial_balances = vec![Uint128::from(1u64), Uint128::from(2u64), Uint128::from(3u64)];
        let vault_weights = vec![1u64, 1u64, 1u64];

        // Set asset allowances for the factory
        vault_assets
            .iter()
            .zip(&vault_initial_balances)
            .for_each(|(asset, amount)| {
                set_token_allowance(
                    &mut app,
                    *amount,
                    asset.clone(),
                    Addr::unchecked(SETUP_MASTER),
                    factory.to_string()
                );
            });



        // Tested action
        let response = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            factory.clone(),
            &crate::msg::ExecuteMsg::DeployVault {
                vault_code_id,
                assets: vault_assets.iter().map(|asset| asset.to_string()).collect(),
                assets_balances: vault_initial_balances.clone(),
                weights: vault_weights.clone(),
                amplification: 1000000000000000000u64,
                pool_fee: TEST_POOL_FEE,
                name: "TestPool".to_string(),
                symbol: "TP".to_string(),
                chain_interface: Some("chain_interface".to_string())
            },
            &[]
        ).unwrap();     // Make sure the transaction succeeds



        // TODO check events (instantiate + reply) to make sure pool deployment is successful

        let vault = get_response_attribute::<String>(response.events[7].clone(), "vault").unwrap();

        // Verify the assets have been transferred to the vault
        vault_assets
            .iter()
            .zip(&vault_initial_balances)
            .for_each(|(asset, balance)| {
                let queried_balance: Uint128 = app.wrap().query_wasm_smart::<BalanceResponse>(
                    asset,
                    &Cw20QueryMsg::Balance { address: vault.clone() }
                ).unwrap().balance;

                assert_eq!(
                    queried_balance,
                    balance
                );
            });


        // Verify the deployed vault has the 'factory' set
        let queried_factory = app.wrap().query_wasm_smart::<FactoryResponse>(
            vault.clone(),
            &MockPoolQueryMsg::Factory {}
        ).unwrap().factory;

        assert_eq!(
            queried_factory,
            factory
        );


        // Verify the deployed vault has the 'setup master' set
        let queried_setup_master = app.wrap().query_wasm_smart::<SetupMasterResponse>(
            vault.clone(),
            &MockPoolQueryMsg::SetupMaster {}
        ).unwrap().setup_master;

        assert_eq!(
            queried_setup_master,
            Some(Addr::unchecked(SETUP_MASTER))
        );


        // Verify the deployed vault has the 'chain interface' set
        let queried_interface = app.wrap().query_wasm_smart::<ChainInterfaceResponse>(
            vault.clone(),
            &MockPoolQueryMsg::ChainInterface {}
        ).unwrap().chain_interface;

        assert_eq!(
            queried_interface,
            Some(Addr::unchecked("chain_interface"))
        );


        // Verify the deployed vault has the 'assets' set
        let queried_assets = app.wrap().query_wasm_smart::<AssetsResponse>(
            vault.clone(),
            &MockPoolQueryMsg::Assets {}
        ).unwrap().assets;

        assert_eq!(
            queried_assets,
            vault_assets.iter().map(|asset| asset.to_string()).collect::<Vec<String>>()
        );


        // Verify the deployed vault has the 'weights' set
        let queried_weights = app.wrap().query_wasm_smart::<WeightsResponse>(
            vault.clone(),
            &MockPoolQueryMsg::Weights {}
        ).unwrap().weights;

        assert_eq!(
            queried_weights,
            vault_weights
        );


        // Verify the deployed vault has the 'pool_fee' set
        let queried_pool_fee = app.wrap().query_wasm_smart::<PoolFeeResponse>(
            vault.clone(),
            &MockPoolQueryMsg::PoolFee {}
        ).unwrap().fee;

        assert_eq!(
            queried_pool_fee,
            TEST_POOL_FEE
        );


        // Verify the deployed vault has the 'governance_fee_share' set
        let queried_governance_fee_share = app.wrap().query_wasm_smart::<GovernanceFeeShareResponse>(
            vault.clone(),
            &MockPoolQueryMsg::GovernanceFeeShare {}
        ).unwrap().fee;

        assert_eq!(
            queried_governance_fee_share,
            TEST_GOVERNANCE_FEE
        );


        // Verify the deployed vault has the 'fee_administrator' set
        let queried_fee_administrator = app.wrap().query_wasm_smart::<FeeAdministratorResponse>(
            vault.clone(),
            &MockPoolQueryMsg::FeeAdministrator {}
        ).unwrap().administrator;

        assert_eq!(
            queried_fee_administrator,
            GOVERNANCE
        );


        //TODO review where the 'name' and 'symbol' are saved
        // Verify the deployed vault has the 'name' and 'symbol' set
        let queried_token_info = app.wrap().query_wasm_smart::<TokenInfoResponse>(
            vault.clone(),
            &MockPoolQueryMsg::TokenInfo {}
        ).unwrap();

        assert_eq!(
            queried_token_info.name,
            "TestPool"
        );

        assert_eq!(
            queried_token_info.symbol,
            "TP"
        );

    }


    #[test]
    fn test_deploy_vault_no_assets() {

        let mut app = App::default();
    
        // Instantiate factory
        let factory = mock_factory(&mut app);

        // Deploy vault contract
        let vault_code_id = mock_vault_contract(&mut app);

        // Define vault config
        let vault_assets = vec![];              // ! no assets
        let vault_initial_balances = vec![];
        let vault_weights = vec![];



        // Tested action
        let response_result = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            factory,
            &crate::msg::ExecuteMsg::DeployVault {
                vault_code_id,
                assets: vault_assets,
                assets_balances: vault_initial_balances,
                weights: vault_weights,
                amplification: 1000000000000000000u64,
                pool_fee: 0u64,
                name: "TestPool".to_string(),
                symbol: "TP".to_string(),
                chain_interface: None
            },
            &[]
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

        let mut app = App::default();
    
        // Instantiate factory
        let factory = mock_factory(&mut app);

        // Deploy vault contract
        let vault_code_id = mock_vault_contract(&mut app);

        // Define vault config
        let vault_assets = deploy_test_tokens(
            &mut app,
            Addr::unchecked(SETUP_MASTER),
            None,
            None
        );
        let vault_initial_balances = vec![Uint128::from(1u64), Uint128::from(2u64), Uint128::from(3u64)];
        let vault_weights = vec![1u64, 1u64, 1u64];

        // Set asset allowances for the factory
        vault_assets
            .iter()
            .zip(&vault_initial_balances)
            .for_each(|(asset, amount)| {
                set_token_allowance(
                    &mut app,
                    *amount,
                    asset.clone(),
                    Addr::unchecked(SETUP_MASTER),
                    factory.to_string()
                );
            });



        // Tested action 1: len(assets_balances) != len(assets)
        let response_result = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            factory.clone(),
            &crate::msg::ExecuteMsg::DeployVault {
                vault_code_id,
                assets: vault_assets.iter().map(|asset| asset.to_string()).collect(),
                assets_balances: vault_initial_balances[..2].to_vec(),   // ! Only 2 balances are provided 
                weights: vault_weights.clone(),
                amplification: 1000000000000000000u64,
                pool_fee: 0u64,
                name: "TestPool".to_string(),
                symbol: "TP".to_string(),
                chain_interface: None
            },
            &[]
        );

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Std(StdError::GenericErr { msg })
                if msg == "Invalid asset/balances/weights count."
        ));



        // Tested action 2: len(weights) != len(assets)
        let response_result = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            factory.clone(),
            &crate::msg::ExecuteMsg::DeployVault {
                vault_code_id,
                assets: vault_assets.iter().map(|asset| asset.to_string()).collect(),
                assets_balances: vault_initial_balances.clone(),
                weights: vault_weights[..2].to_vec(),   // ! Only 2 weights are provided 
                amplification: 1000000000000000000u64,
                pool_fee: 0u64,
                name: "TestPool".to_string(),
                symbol: "TP".to_string(),
                chain_interface: None
            },
            &[]
        );

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Std(StdError::GenericErr { msg })
                if msg == "Invalid asset/balances/weights count."
        ));



        // Make sure the transaction does succeed with valid params
        app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            factory,
            &crate::msg::ExecuteMsg::DeployVault {
                vault_code_id,
                assets: vault_assets.iter().map(|asset| asset.to_string()).collect(),
                assets_balances: vault_initial_balances,
                weights: vault_weights,
                amplification: 1000000000000000000u64,
                pool_fee: 0u64,
                name: "TestPool".to_string(),
                symbol: "TP".to_string(),
                chain_interface: None
            },
            &[]
        ).unwrap();     // ! Make sure the transaction succeeds

    }




    #[test]
    fn test_change_default_governance_fee_share() {

        let mut app = App::default();
    
        // 'Deploy' the contract
        let code_id = mock_factory_contract(&mut app);

        let initial_default_governance_fee_share = 10101u64;
        let new_default_governance_fee_share = 20202u64;

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
        let _response = app.execute_contract(
            Addr::unchecked(GOVERNANCE),
            factory.clone(),
            &ExecuteMsg::SetDefaultGovernanceFeeShare { fee: new_default_governance_fee_share },
            &[]
        ).unwrap();     // Make sure the transaction passes



        // TODO verify event

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

        let initial_default_governance_fee_share = 10101u64;
        let new_default_governance_fee_share = 20202u64;

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
            &ExecuteMsg::SetDefaultGovernanceFeeShare { fee: new_default_governance_fee_share },
            &[]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ))

    }


    // Ownership tests
    // TODO events

    #[test]
    fn test_owner_is_set_on_instantiation_and_query() {

        let mut app = App::default();
    
        // 'Deploy' the contract
        let code_id = mock_factory_contract(&mut app);



        // Tested action
        let factory = app.instantiate_contract(
            code_id,
            Addr::unchecked(GOVERNANCE),
            &InstantiateMsg { default_governance_fee_share: 0u64 },
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
            &InstantiateMsg { default_governance_fee_share: 0u64 },
            &[],
            "catalyst-factory",
            None
        ).unwrap();     // Make sure the transaction passes

        let new_owner = "new_owner_addr".to_string();



        // Tested action: transfer ownership
        let _response = app.execute_contract(
            Addr::unchecked(GOVERNANCE),
            factory.clone(),
            &ExecuteMsg::TransferOwnership { new_owner: new_owner.clone() },
            &[]
        ).unwrap();     // Make sure the transaction passes



        //TODO check event

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
            &InstantiateMsg { default_governance_fee_share: 0u64 },
            &[],
            "catalyst-factory",
            None
        ).unwrap();     // Make sure the transaction passes

        let new_owner = "new_owner_addr".to_string();



        // Tested action: transfer ownership
        let response_result = app.execute_contract(
            Addr::unchecked("not-factory-owner"),           // ! Not the factory owner (i.e. GOVERNANCE)
            factory.clone(),
            &ExecuteMsg::TransferOwnership { new_owner: new_owner.clone() },
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