
use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;
use fixed_point_math_lib::u256::U256;
use swap_pool_common::state::DECAYRATE;

use crate::ContractError;

// TODO move to swap-pool-common?
// TODO change name + storage_key to avoid possible collisions when importing? (e.g. swap-pool-state) 
pub const STATE: Item<SwapPoolState> = Item::new("state");

#[cw_serde]
pub struct SwapPoolState {
    pub setup_master: Option<Addr>,
    // pub dao_authority: Addr,     // TODO to be replaced/checked once the DAO structure gets finalized
    pub ibc_interface: Option<Addr>,
    pub assets: Vec<Addr>,
    pub assets_weights: Vec<u64>,
    pub assets_balance0s: Vec<Uint128>,

    pub escrowed_assets: Vec<Uint128>,

    pub max_units_inflow_x64: [u64; 4],         // TODO use U256 directly
    pub current_units_inflow_x64: [u64; 4],     // TODO use U256 directly
    pub current_units_inflow_timestamp: u64,

    pub current_liquidity_inflow: Uint128,
    pub current_liquidity_inflow_timestamp: u64
}

// TODO this has been copied (almost as is) from the Solana implementation
// TODO do we want to define the functions here? (as impl of the SwapPoolState)
impl SwapPoolState {

    pub fn get_asset_index(&self, asset: &String) -> Result<usize, ContractError> {
        self.assets
            .iter()
            .enumerate()
            .find_map(|(index, a): (usize, &Addr)| if *a == *asset { Some(index) } else { None })
            .ok_or(ContractError::InvalidAssets {})
    }

    pub fn update_units_inflow(
        &mut self,
        units_inflow_x64: U256,
        current_timestamp: u64
    ) -> Result<(), ContractError> {

        let max_units_inflow_x64 = U256(self.max_units_inflow_x64);

        // If more time has passed since the last update than DECAYRATE, the current inflow state does not matter (it has fully decayed)
        if current_timestamp > self.current_units_inflow_timestamp + DECAYRATE {
            if units_inflow_x64 > max_units_inflow_x64 {
                return Err(ContractError::SwapLimitExceeded {});
            }

            self.current_units_inflow_x64       = units_inflow_x64.0;
            self.current_units_inflow_timestamp = current_timestamp;

            return Ok(());
        }

        // Compute how much inflow has decayed since last update
        let current_units_inflow_x64 = U256(self.current_units_inflow_x64);

        let decayed_inflow = max_units_inflow_x64.checked_mul(
            U256::from(current_timestamp.checked_sub(self.current_units_inflow_timestamp).unwrap())  // TODO checked_sub required?
        ).unwrap() / DECAYRATE;

        // If the current inflow is less then the (max allowed) decayed one
        if current_units_inflow_x64 <= decayed_inflow {
            if units_inflow_x64 > max_units_inflow_x64 {
                return Err(ContractError::SwapLimitExceeded {});
            }

            self.current_units_inflow_x64 = units_inflow_x64.0;
        }
        // If some of the current inflow still matters
        else {
            let new_net_units_inflow_x64 = (current_units_inflow_x64 - decayed_inflow).checked_add(units_inflow_x64).unwrap();  // Substraction is safe, as current_units_inflow_x64 > decayed_inflow is guaranteed by if statement

            if new_net_units_inflow_x64 > max_units_inflow_x64 {
                return Err(ContractError::SwapLimitExceeded {});
            }

            self.current_units_inflow_x64 = new_net_units_inflow_x64.0;
        }

        self.current_units_inflow_timestamp = current_timestamp;

        Ok(())
    }


    pub fn update_liquidity_units_inflow(
        &mut self,
        pool_tokens_flow: Uint128,
        current_pool_token_supply: Uint128,
        current_timestamp: u64
    ) -> Result<(), ContractError> {

        // Allows 1/3 of the pool to be drained through liquidity swaps
        let max_pool_tokens_flow = current_pool_token_supply / Uint128::from(2_u64);

        // If more time has passed since the last update than DECAYRATE, the current inflow state does not matter (it has fully decayed)
        if current_timestamp > self.current_liquidity_inflow_timestamp + DECAYRATE {
            if pool_tokens_flow > max_pool_tokens_flow {
                return Err(ContractError::LiquiditySwapLimitExceeded {});
            }

            self.current_liquidity_inflow           = pool_tokens_flow;
            self.current_liquidity_inflow_timestamp = current_timestamp;

            return Ok(());
        }

        // Compute how much inflow has decayed since last update
        let decayed_inflow = max_pool_tokens_flow.checked_mul(
            current_timestamp.checked_sub(self.current_liquidity_inflow_timestamp).unwrap().try_into().unwrap()  // TODO checked_sub required?
        ).unwrap() / Uint128::new(DECAYRATE as u128);

        // If the current inflow is less then the (max allowed) decayed one
        if self.current_liquidity_inflow <= decayed_inflow {
            if pool_tokens_flow > max_pool_tokens_flow {
                return Err(ContractError::LiquiditySwapLimitExceeded {});
            }

            self.current_liquidity_inflow = pool_tokens_flow;
        }
        // If some of the current inflow still matters
        else {
            let new_net_liquidity_inflow = (self.current_liquidity_inflow - decayed_inflow).checked_add(pool_tokens_flow).unwrap();  // Substraction is safe, as current_liquidity_inflow > decayed_inflow is guaranteed by if statement

            if new_net_liquidity_inflow > max_pool_tokens_flow {
                return Err(ContractError::LiquiditySwapLimitExceeded {});
            }

            self.current_liquidity_inflow = new_net_liquidity_inflow;
        }

        self.current_liquidity_inflow_timestamp = current_timestamp;

        Ok(())
    }

}