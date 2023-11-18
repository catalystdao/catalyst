use anyhow::Result as AnyResult;
use cosmwasm_schema::serde::Serialize;
use cosmwasm_std::{Uint128, Addr, testing::{MockApi, MockStorage}, Empty, Coin};
use cw_multi_test::{App, AppResponse, BankKeeper, WasmKeeper, FailingModule};

use crate::asset::CustomTestAsset;

pub mod env_cw20_asset;
pub mod env_native_asset;


pub type CustomApp<CustomHandler=FailingModule<Empty, Empty, Empty>, ExecT=Empty> = App<
    BankKeeper,
    MockApi,
    MockStorage,
    CustomHandler,
    WasmKeeper<ExecT, Empty>
>;


/// Helper around `cw_multi_test::App` to allow configurable contract testing.
pub trait CustomTestEnv<AppC, TestAssetC: CustomTestAsset<AppC>> {

    /// Setup the test 'environment'. This includes creating mock assets according to the
    /// desired asset type (native assets/cw20).
    /// 
    /// # Arguments:
    /// * `gov` - The account to hold the created assets.
    /// 
    fn initialize(gov: String) -> Self;


    /// Get a reference to `cw_multi_test::App`.
    fn get_app(&mut self) -> &mut AppC;


    /// Get the mock assets.
    fn get_assets(&self) -> Vec<TestAssetC>;


    /// Execute a contract with the specified funds.
    /// - Native assets: the funds are specified on the contract execution.
    /// - CW20 assets: token allowances are set for the invoked contract.
    fn execute_contract<U: Serialize + std::fmt::Debug>(
        &mut self,
        sender: Addr,
        contract_addr: Addr,
        msg: &U,
        send_assets: Vec<TestAssetC>,
        send_amounts: Vec<Uint128>
    ) -> AnyResult<AppResponse> {

        self.execute_contract_with_additional_coins(
            sender,
            contract_addr,
            msg,
            send_assets,
            send_amounts,
            vec![]
        )
    }


    /// Execute a contract with the specified funds and additional coins.
    /// - Native assets: the funds are specified on the contract execution.
    /// - CW20 assets: token allowances are set for the invoked contract.
    fn execute_contract_with_additional_coins<U: Serialize + std::fmt::Debug>(
        &mut self,
        sender: Addr,
        contract_addr: Addr,
        msg: &U,
        send_assets: Vec<TestAssetC>,
        send_amounts: Vec<Uint128>,
        additional_coins: Vec<Coin>
    ) -> AnyResult<AppResponse>;


    /// Initialize a new coin and set a balance for the given account.
    fn initialize_coin(
        &mut self,
        denom: String,
        amount: Uint128,
        account: String
    ) -> ();

}