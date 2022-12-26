use std::convert::TryInto;

use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use ibc_interface;
use ibc_interface::cpi::accounts as interface_accounts;
use ibc_interface::program::IbcInterface;

use fixed_point_math_lib::u256::U256;
use fixed_point_math_lib::fixed_point_math_x64::{mul_x64, pow_x64, inv_pow2_x64};
use token_utils::create_pda_token_account;

use std::ops::Shl;

pub mod calculation_helpers;

const ONE_X64: U256 = U256([0, 1, 0, 0]);

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnA");

const NUMASSETS: usize = 3;

const DECAYRATE: u64 = 60*60*24;

const POOL_ASSET_WALLET_SEED : &[u8] = b"poolAsset";
const POOL_TOKEN_MINT_SEED   : &[u8] = b"poolMint";
const POOL_AUTHORITY_SEED    : &[u8] = b"poolAuth";
const POOL_ESCROW_SEED       : &[u8] = b"poolEscrow";

#[program]
pub mod swap_pool_amplified {

    use super::*;

    pub fn initialize<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, Initialize<'info>>,
        k: u64,
        initial_asset_balances: [u64; NUMASSETS],
        asset_weights: [u64; NUMASSETS]
    ) -> Result<()> {

        // ! Save the pubkey of setup_master to verify setup instructions (only allow setup_master to call setup instructions)
        ctx.accounts.swap_pool_state_account.setup_master = ctx.accounts.setup_master.key();
        ctx.accounts.swap_pool_state_account.dao_authority = ctx.accounts.dao_authority.key();
        ctx.accounts.swap_pool_state_account.amplification_x64 = (ONE_X64 / k).0;   // TODO Add serialisation/desarialisation to U256 struct, avoid having to explicitly extract U256 internal array
        ctx.accounts.swap_pool_state_account.authority_bump = ctx.bumps.get("swap_pool_authority").unwrap().to_owned();
        ctx.accounts.swap_pool_state_account.token_mint_bump = ctx.bumps.get("swap_pool_token_mint").unwrap().to_owned();

        let asset_count = match initial_asset_balances.iter().position(|balance| balance == &0u64) {
            Some(count) => count,
            None => NUMASSETS
        };

        if asset_count == 0 {
            return Err(error!(ErrorCode::NoAssetsProvided));
        }

        // TODO is the following check required?
        // Verify the count of passed remaining accounts
        // 3 Accounts given per added asset:
        //      - Asset mint
        //      - Swap pool asset wallet (to be created)
        //      - Depositor asset wallet
        if ctx.remaining_accounts.len() != 3*asset_count {
            return Err(error!(ErrorCode::InvalidRemainingAccountsCount));
        }

        let mut max_units_inflow_x64 = U256([0, 0, 0, 0]);
        let one_minus_amp = ONE_X64 - U256(ctx.accounts.swap_pool_state_account.amplification_x64);

        for asset_index in 0..asset_count {

            // Get accounts from 'remaining accounts'
            // ! TODO overhaul accounts verification
            let asset_mint             = &ctx.remaining_accounts[asset_index*3];    // Mint account verified by token account creation cpi // ! TODO verify
            let swap_pool_asset_wallet = &ctx.remaining_accounts[asset_index*3+1];  // Verified in 'create_swap_pool_asset_wallet' step
            let depositor_asset_wallet = &ctx.remaining_accounts[asset_index*3+2];  // No need to verify, assets taken from here
            
            // Create the wallet to hold the pool assets
            ctx.accounts.swap_pool_state_account.wallets_bumps[asset_index] = ctx.accounts.create_swap_pool_asset_wallet(
                asset_mint.to_owned(),
                swap_pool_asset_wallet.to_owned()
            ).unwrap();

            // Transfer the assets from the depositor to the pool
            ctx.accounts.transfer_assets_from_depositor(
                initial_asset_balances[asset_index],
                depositor_asset_wallet.to_account_info(),
                swap_pool_asset_wallet.to_account_info()
            ).unwrap();

            // Save asset details
            let asset_weight = asset_weights[asset_index];
            if asset_weight == 0 {
                return Err(error!(ErrorCode::InvalidAssetWeight));
            }

            max_units_inflow_x64 += U256::from(asset_weight).checked_mul(
                pow_x64(U256::from(initial_asset_balances[asset_index]).shl(64), one_minus_amp).unwrap()
            ).unwrap();

            ctx.accounts.swap_pool_state_account.pool_assets_mints[asset_index]       = asset_mint.key();
            ctx.accounts.swap_pool_state_account.pool_assets_weights[asset_index]     = asset_weight;
            ctx.accounts.swap_pool_state_account.pool_assets_eq_balances[asset_index] = initial_asset_balances[asset_index];

        }

        let units_inflow_amplification_x64 = ONE_X64.checked_sub(inv_pow2_x64(one_minus_amp).unwrap()).unwrap();

        ctx.accounts.swap_pool_state_account.units_inflow_amplification_x64 = units_inflow_amplification_x64.0;
        ctx.accounts.swap_pool_state_account.max_units_inflow_x64 = mul_x64(
            units_inflow_amplification_x64, 
            max_units_inflow_x64
        ).unwrap().0;

        // Mint pool tokens for the depositor
        ctx.accounts.mint_pool_tokens_for_depositor(
            1000000,    // TODO set value (cannot be set equal to EVM implementation, as that is too large)
        ).unwrap();
        
        Ok(())
    }
    
    pub fn create_connection(
        ctx: Context<CreateConnection>,
        chain: u64,                     // Used in the CreateConnection context
        pool: Pubkey,                   // Used in the CreateConnection context
        interface_program: Pubkey       // target interface program
    ) -> Result<()> {

        let ccsi_program = ctx.accounts.ibc_interface_program.to_account_info();

        let ccsi_swap_accounts = interface_accounts::CreateConnection {
            swap_pool_authority: ctx.accounts.swap_pool_authority.to_account_info(),
            rent_payer: ctx.accounts.rent_payer.to_account_info(),
            interface_state_account: ctx.accounts.interface_state_account.to_account_info(),
            connection_state_account: ctx.accounts.connection_state_account.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info()
        };

        let signer: &[&[&[u8]]] = &[&[
            ctx.accounts.swap_pool_state_account.to_account_info().key.as_ref(),
            &POOL_AUTHORITY_SEED[..],
            &[ctx.accounts.swap_pool_state_account.authority_bump]   // PDA bump
        ]];

        let ccsi_swap_ctx = CpiContext::new_with_signer(
            ccsi_program,
            ccsi_swap_accounts,
            signer
        );

        ibc_interface::cpi::create_connection(
            ccsi_swap_ctx,
            chain,
            pool,
            interface_program
        ).unwrap();

        Ok(())
    }

    pub fn link_ibc_interface(
        ctx: Context<LinkIBCInterface>,
        ibc_interface: Pubkey
    ) -> Result<()> {
        ctx.accounts.swap_pool_state_account.ibc_interface = ibc_interface;
        Ok(())
    }

    pub fn finish_setup(ctx: Context<FinishSetup>) -> Result<()> {

        // Make sure the swap interface is linked
        if ctx.accounts.swap_pool_state_account.ibc_interface.eq(&Pubkey::default()) {
            return Err(error!(ErrorCode::SwapInterfaceNotLinked));
        }

        // Set setup_master pubkey to default (empty pubkey) to block any future setup calls
        ctx.accounts.swap_pool_state_account.setup_master = Pubkey::default();
        Ok(())
    }

    //TODO add function to cancel SwapPool setup and recover rent?

    pub fn deposit<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, Deposit<'info>>,
        pool_tokens_amount: u64
    ) -> Result<()> {

        // Update the liqudity security limit. Since the limit is based on the current totalSupply, changing the totalSupply
        // upwards by depositing changes the limit.
        let current_timestamp: u64 = Clock::get().unwrap().unix_timestamp.try_into().unwrap();
        ctx.accounts.swap_pool_state_account.update_liquidity_units_inflow(
            0,
            ctx.accounts.swap_pool_token_mint.supply,
            current_timestamp
        )?;

        let mut deposited_amounts: [u64; NUMASSETS] = [0; NUMASSETS];

        let swap_pool_state_account = &mut ctx.accounts.swap_pool_state_account;

        let pool_assets_mints: [Pubkey; NUMASSETS] = swap_pool_state_account.pool_assets_mints.clone();

        let mut depositor_asset_wallets: [Pubkey; NUMASSETS] = [Pubkey::default(); NUMASSETS];

        // Verify the count of passed remaining accounts
        // 3 Accounts given per deposited asset:
        //      - Asset mint
        //      - Swap pool asset wallet
        //      - Depositor asset wallet
        for asset_index in 0..NUMASSETS {

            if swap_pool_state_account.pool_assets_mints[asset_index].eq(&Pubkey::default()) {
                break;
            }

            // Verify the depositor_asset_wallet and the swap_pool_asset_wallet accounts
            // ! TODO VERY IMPORTANT, THIS VERIFICATION HAS TO BE REVISED
            let asset_mint             = &ctx.remaining_accounts[asset_index*3];    // Mint account verified by token account creation cpi // ! TODO verify
            let swap_pool_asset_wallet = &ctx.remaining_accounts[asset_index*3+1];  // Verified in 'create_swap_pool_asset_wallet' step
            let depositor_asset_wallet = &ctx.remaining_accounts[asset_index*3+2];  // No need to verify, assets taken from here
            
            // Verify the asset mint
            if asset_mint.key().ne(&pool_assets_mints[asset_index]) {
                return Err(error!(ErrorCode::InvalidAssetMintAccount));
            }

            // Make sure the swap_pool_asset_wallet provided matches the expected pda
            let seeds: &[&[u8]] = &[
                &swap_pool_state_account.key().to_bytes(),
                &pool_assets_mints[asset_index].key().to_bytes(),
                POOL_ASSET_WALLET_SEED,
                &[swap_pool_state_account.wallets_bumps[asset_index]]
            ];

            let expected_swap_pool_asset_wallet  = Pubkey::create_program_address(seeds, &ID).unwrap();

            if swap_pool_asset_wallet.key() != expected_swap_pool_asset_wallet {
                return Err(error!(ErrorCode::InvalidSwapPoolAssetWalletAccount));
            }
        
            let swap_pool_asset_wallet_balance = TokenAccount::try_deserialize(&mut &swap_pool_asset_wallet.try_borrow_data()?[..])?.amount;

            // Compute the asset deposit amount
            let asset_eq_balance = swap_pool_state_account.pool_assets_eq_balances[asset_index];
            let pool_tokens_for_asset = pool_tokens_amount
                .checked_mul(asset_eq_balance).unwrap()
                .checked_div(ctx.accounts.swap_pool_token_mint.supply).unwrap();

            let asset_deposit_amount = calculation_helpers::calc_asset_amount_for_pool_tokens(
                pool_tokens_for_asset,
                swap_pool_asset_wallet_balance,     // Escrowed tokens are NOT subtracted from the total balance => deposits should return less
                asset_eq_balance
            )?;

            // Update asset eq balance
            swap_pool_state_account.pool_assets_eq_balances[asset_index] = asset_eq_balance.checked_add(pool_tokens_for_asset).unwrap();

            token_utils::transfer_tokens_using_pda_authority(  //TODO use transfer_checked?
                asset_deposit_amount,
                depositor_asset_wallet.to_owned(),
                swap_pool_asset_wallet.to_owned(),
                ctx.accounts.swap_pool_authority.to_account_info(),
                &[
                    &swap_pool_state_account.key().to_bytes(), // SwapPool state account
                    &POOL_AUTHORITY_SEED[..],                  // Pool authority seed
                    &[swap_pool_state_account.authority_bump]  // PDA bump
                ],
                ctx.accounts.token_program.to_account_info()
            );

            // Save data for logging
            depositor_asset_wallets[asset_index] = depositor_asset_wallet.key();
            deposited_amounts[asset_index] = asset_deposit_amount;
        }

        // Mint pool tokens for the depositor
        token_utils::mint_tokens_using_pda_authority(
            pool_tokens_amount,
            ctx.accounts.depositor_pool_token_wallet.to_account_info(),
            ctx.accounts.swap_pool_token_mint.to_account_info(),
            ctx.accounts.swap_pool_authority.to_account_info(),
            &[
                    &swap_pool_state_account.key().to_bytes(),  // SwapPool state account
                    &POOL_AUTHORITY_SEED[..],                   // Pool authority seed
                    &[swap_pool_state_account.authority_bump]   // PDA bump
                ],
            ctx.accounts.token_program.to_account_info()
        );

        emit!(DepositEvent {
            swap_pool: ctx.accounts.swap_pool_state_account.key(),

            deposited_asset_amounts: deposited_amounts,
            depositor_asset_wallets: depositor_asset_wallets,
            withdrawn_pool_token_amount: pool_tokens_amount,
            depositor_pool_token_wallet: ctx.accounts.depositor_pool_token_wallet.key()
        
        });

        Ok(())
    }

    pub fn withdraw<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, Withdraw<'info>>,
        pool_tokens_amount: u64
    ) -> Result<()> {

        // Update the liqudity security limit. Since the limit is based on the current totalSupply, changing the totalSupply
        // downwards by withdrawing changes the limit.
        let current_timestamp: u64 = Clock::get().unwrap().unix_timestamp.try_into().unwrap();
        ctx.accounts.swap_pool_state_account.update_liquidity_units_inflow(
            0,
            ctx.accounts.swap_pool_token_mint.supply,
            current_timestamp
        )?;

        // Burn pool tokens from the withdrawer wallet.
        ctx.accounts.burn_pool_tokens_of_withdrawer(pool_tokens_amount)?;

        let mut withdrawn_amounts: [u64; NUMASSETS] = [0; NUMASSETS];

        let swap_pool_state_account = &mut ctx.accounts.swap_pool_state_account;

        let pool_assets_mints: [Pubkey; NUMASSETS] = swap_pool_state_account.pool_assets_mints.clone();

        let mut withdrawer_asset_wallets: [Pubkey; NUMASSETS] = [Pubkey::default(); NUMASSETS];

        let initial_pool_tokens_supply = ctx.accounts.swap_pool_token_mint.supply;    // Theoretically, this is not needed, as the 'burn' operation does not update the data that has been loaded from the account, but this is here for clarity
        
        // 3 Accounts given per withdrawn asset:
        //      - Asset mint
        //      - Swap pool asset wallet
        //      - Depositor asset wallet
        for asset_index in 0..NUMASSETS {

            if swap_pool_state_account.pool_assets_mints[asset_index].eq(&Pubkey::default()) {
                break;
            }

            // Verify the withdrawer_asset_wallet and the swap_pool_asset_wallet accounts
            // ! TODO VERY IMPORTANT, THIS VERIFICATION HAS TO BE REVISED
            let asset_mint              = &ctx.remaining_accounts[asset_index*3];    // Mint account verified by token account creation cpi // ! TODO verify
            let swap_pool_asset_wallet  = &ctx.remaining_accounts[asset_index*3+1];  // Verified in 'create_swap_pool_asset_wallet' step
            let withdrawer_asset_wallet = &ctx.remaining_accounts[asset_index*3+2];  // No need to verify, assets taken from here
            
            // Verify the asset mint
            if asset_mint.key().ne(&pool_assets_mints[asset_index]) {
                return Err(error!(ErrorCode::InvalidAssetMintAccount));
            }

            // Make sure the swap_pool_asset_wallet provided matches the expected pda
            let seeds: &[&[u8]] = &[
                &swap_pool_state_account.key().to_bytes(),
                &pool_assets_mints[asset_index].key().to_bytes(),
                POOL_ASSET_WALLET_SEED,
                &[swap_pool_state_account.wallets_bumps[asset_index]]
            ];

            let expected_swap_pool_asset_wallet  = Pubkey::create_program_address(seeds, &ID).unwrap();

            if swap_pool_asset_wallet.key() != expected_swap_pool_asset_wallet {
                return Err(error!(ErrorCode::InvalidSwapPoolAssetWalletAccount));
            }
        
            let swap_pool_asset_wallet_balance = TokenAccount::try_deserialize(&mut &swap_pool_asset_wallet.try_borrow_data()?[..])?.amount;

            // Compute the asset withdraw amount
            let asset_eq_balance = swap_pool_state_account.pool_assets_eq_balances[asset_index];
            let pool_tokens_for_asset = pool_tokens_amount
                .checked_mul(asset_eq_balance).unwrap()
                .checked_div(initial_pool_tokens_supply).unwrap();
            
            let asset_withdraw_amount = calculation_helpers::calc_asset_amount_for_pool_tokens(
                pool_tokens_for_asset,
                swap_pool_asset_wallet_balance
                    .checked_sub(swap_pool_state_account.escrowed_assets[asset_index]).unwrap(),         // Escrowed tokens ARE subtracted from the total balance => withdrawals should return less
                asset_eq_balance
            )?;

            // Update asset eq balance
            swap_pool_state_account.pool_assets_eq_balances[asset_index] = asset_eq_balance.checked_sub(pool_tokens_for_asset).unwrap();
        
            token_utils::transfer_tokens_using_pda_authority(
                asset_withdraw_amount,
                swap_pool_asset_wallet.to_account_info(),
                withdrawer_asset_wallet.to_account_info(),
                ctx.accounts.swap_pool_authority.to_account_info(),
                &[
                    &swap_pool_state_account.key().to_bytes(), // SwapPool state account
                    &POOL_AUTHORITY_SEED[..],                  // Pool authority seed
                    &[swap_pool_state_account.authority_bump]  // PDA bump
                ],
                ctx.accounts.token_program.to_account_info()
            );
    
            // Save data for logging
            withdrawer_asset_wallets[asset_index] = withdrawer_asset_wallet.key();
            withdrawn_amounts[asset_index] = asset_withdraw_amount;
        }

        emit!(WithdrawEvent {
            swap_pool: ctx.accounts.swap_pool_state_account.key(),
        
            withdrawn_asset_amounts: withdrawn_amounts,
            withdrawer_asset_wallets: withdrawer_asset_wallets,
        
            burnt_pool_token_amount: pool_tokens_amount,
            withdrawer_pool_token_wallet: ctx.accounts.withdrawer_pool_token_wallet.key(),
        });

        Ok(())
    }

    pub fn local_swap(ctx: Context<LocalSwap>, amount: u64, min_yield: u64) -> Result<()> {

        let from_asset_index = ctx.accounts.swap_pool_state_account.get_asset_index(
            &ctx.accounts.input_asset_mint.key()
        ).unwrap();

        let to_asset_index = ctx.accounts.swap_pool_state_account.get_asset_index(
            &ctx.accounts.output_asset_mint.key()
        ).unwrap();
        
        let out: u64 = calculation_helpers::full_swap(
            U256::from(amount),
            U256::from(ctx.accounts.swap_pool_input_asset_wallet.amount),
            U256::from(ctx.accounts.swap_pool_state_account.pool_assets_weights[from_asset_index]),
            U256::from(
                ctx.accounts.swap_pool_output_asset_wallet.amount.checked_sub(
                    ctx.accounts.swap_pool_state_account.escrowed_assets[to_asset_index]
                ).unwrap()
            ),
            U256::from(ctx.accounts.swap_pool_state_account.pool_assets_weights[to_asset_index]),
            U256(ctx.accounts.swap_pool_state_account.amplification_x64)
        )?.as_u64();      // U256 to u64 will panic if overflow

        if out < min_yield { return Err(error!(ErrorCode::SwapMinYieldNotFulfilled))}

        // Transfer input assets to the pool
        token_utils::transfer_tokens_using_pda_authority(  //TODO use transfer_checked?
            amount,
            ctx.accounts.input_asset_wallet.to_account_info(),
            ctx.accounts.swap_pool_input_asset_wallet.to_account_info(),
            ctx.accounts.swap_pool_authority.to_account_info(),
            &[
                &ctx.accounts.swap_pool_state_account.key().to_bytes(),  // SwapPool state account
                &POOL_AUTHORITY_SEED[..],                                // Pool authority seed
                &[ctx.accounts.swap_pool_state_account.authority_bump]   // PDA bump
            ],
            ctx.accounts.token_program.to_account_info()
        );

        // Transfer output assets to the user
        token_utils::transfer_tokens_using_pda_authority(  //TODO use transfer_checked?
            out,
            ctx.accounts.swap_pool_output_asset_wallet.to_account_info(),
            ctx.accounts.output_asset_wallet.to_account_info(),
            ctx.accounts.swap_pool_authority.to_account_info(),
            &[
                &ctx.accounts.swap_pool_state_account.key().to_bytes(),  // SwapPool state account
                &POOL_AUTHORITY_SEED[..],                                // Pool authority seed
                &[ctx.accounts.swap_pool_state_account.authority_bump]   // PDA bump
            ],
            ctx.accounts.token_program.to_account_info()
        );


        ctx.accounts.swap_pool_state_account.max_units_inflow_x64 = 
            U256(ctx.accounts.swap_pool_state_account.max_units_inflow_x64)
                .checked_add(mul_x64(
                    U256(ctx.accounts.swap_pool_state_account.units_inflow_amplification_x64),
                    ctx.accounts.swap_pool_state_account.get_units_inflow_capacity(
                        ctx.accounts.swap_pool_input_asset_wallet.amount,           // NOTE: this amount is from BEFORE the asset transfer
                        ctx.accounts.swap_pool_input_asset_wallet.amount + amount,
                        from_asset_index
                    )?
                ).unwrap()).unwrap()
                .checked_sub(mul_x64(
                    U256(ctx.accounts.swap_pool_state_account.units_inflow_amplification_x64),
                    ctx.accounts.swap_pool_state_account.get_units_inflow_capacity(
                        ctx.accounts.swap_pool_output_asset_wallet.amount,           // NOTE: this amount is from BEFORE the asset transfer
                        ctx.accounts.swap_pool_output_asset_wallet.amount - out,
                        to_asset_index
                    )?
                ).unwrap()).unwrap().0;


        emit!(LocalSwapEvent {
            swap_pool: ctx.accounts.swap_pool_state_account.key(),
        
            deposited_asset_mint: ctx.accounts.input_asset_mint.key(),
            depositor_asset_wallet: ctx.accounts.input_asset_wallet.key(),
        
            withdrawn_asset_mint: ctx.accounts.output_asset_mint.key(),
            withdrawer_asset_wallet: ctx.accounts.output_asset_wallet.key(),
        
            deposited_asset_amount: amount,
            withdrawn_asset_amount: out,
            fees: 0
        });

        Ok(())
    }

    pub fn out_swap(
        ctx: Context<OutSwap>,
        chain: u64,
        target_pool: Pubkey,
        to_asset_index: u8,
        destination: Pubkey,
        amount: u64,
        min_output: [u64; 4],
        escrow_nonce: u32,
        fallback_wallet: Pubkey
    ) -> Result<()> {

        let from_asset_index = ctx.accounts.swap_pool_state_account.get_asset_index(
            &ctx.accounts.input_asset_mint.key()
        ).unwrap();

        let units_x64 = calculation_helpers::out_swap_x64(
            U256::from(amount),
            U256::from(ctx.accounts.swap_pool_input_asset_wallet.amount),
            U256::from(ctx.accounts.swap_pool_state_account.pool_assets_weights[from_asset_index]),
            U256(ctx.accounts.swap_pool_state_account.amplification_x64)
        )?;

        //TODO! unitsDiff       line 902

        let ccsi_program = ctx.accounts.ibc_interface_program.to_account_info(); // Check is the same as the stored program?
        let ccsi_swap_accounts = interface_accounts::CrossChainSwap {
            swap_pool_authority: ctx.accounts.swap_pool_authority.to_account_info(),
            interface_state_account: ctx.accounts.interface_state_account.to_account_info(),
            connection_state_account: ctx.accounts.connection_state_account.to_account_info(),
            polymerase_sender_program: ctx.accounts.polymerase_sender_program.to_account_info(),
            polymerase_ibc_data: ctx.accounts.polymerase_ibc_data.to_account_info(),
            polymerase_ibc_data_account_payer: ctx.accounts.polymerase_ibc_data_account_payer.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info()
        };

        let signer: &[&[&[u8]]] = &[&[
            ctx.accounts.swap_pool_state_account.to_account_info().key.as_ref(),
            &POOL_AUTHORITY_SEED[..],
            &[ctx.accounts.swap_pool_state_account.authority_bump]   // PDA bump
        ]];

        let ccsi_swap_ctx = CpiContext::new_with_signer(
            ccsi_program,
            ccsi_swap_accounts,
            signer
        );

        ibc_interface::cpi::cross_chain_swap(
            ccsi_swap_ctx,
            chain,
            false,
            ctx.accounts.swap_pool_state_account.key(),
            target_pool,
            to_asset_index,
            destination,
            units_x64.0,
            min_output,
            escrow_nonce
        ).unwrap();


        // Transfer input assets to the pool
        token_utils::transfer_tokens_using_pda_authority(  //TODO use transfer_checked?
            amount,
            ctx.accounts.input_asset_wallet.to_account_info(),
            ctx.accounts.swap_pool_input_asset_wallet.to_account_info(),
            ctx.accounts.swap_pool_authority.to_account_info(),
            &[
                &ctx.accounts.swap_pool_state_account.key().to_bytes(),  // SwapPool state account
                &POOL_AUTHORITY_SEED[..],                                // Pool authority seed
                &[ctx.accounts.swap_pool_state_account.authority_bump]   // PDA bump
            ],
            ctx.accounts.token_program.to_account_info()
        );

        // Escrow the tokens
        ctx.accounts.swap_pool_state_account.escrowed_assets[from_asset_index] =
            ctx.accounts.swap_pool_state_account.escrowed_assets[from_asset_index].checked_add(amount).unwrap();

        ctx.accounts.swap_escrow.amount                 = amount;
        ctx.accounts.swap_escrow.asset_index            = from_asset_index as u8;
        ctx.accounts.swap_escrow.fallback_wallet        = fallback_wallet;
        ctx.accounts.swap_escrow.swap_escrow_rent_payer = ctx.accounts.swap_escrow_rent_payer.key();
        ctx.accounts.swap_escrow.bump                   = ctx.bumps.get("swap_escrow").unwrap().to_owned();


        // TODO verify
        ctx.accounts.swap_pool_state_account.unit_tracker_x64 = ctx.accounts.swap_pool_state_account.unit_tracker_x64
            .checked_add(units_x64.as_u128().try_into().unwrap()).unwrap();

        
        ctx.accounts.swap_pool_state_account.max_units_inflow_x64 = 
            U256(ctx.accounts.swap_pool_state_account.max_units_inflow_x64)
                .checked_add(mul_x64(
                    U256(ctx.accounts.swap_pool_state_account.units_inflow_amplification_x64),
                    ctx.accounts.swap_pool_state_account.get_units_inflow_capacity(
                        ctx.accounts.swap_pool_input_asset_wallet.amount,           // NOTE: this amount is from BEFORE the asset transfer
                        ctx.accounts.swap_pool_input_asset_wallet.amount + amount,
                        from_asset_index
                    )?
                ).unwrap()).unwrap().0;



        // Incoming swaps are subtracted from the net pool unit flow. It is assumed that if the router is fraudulent, 
        // no one will execute a trade. Hence, if people swap into the pool, it is expected that there is exactly that 
        // 'inswapped' amount of trust in the pool. Otherwise there would be effectively a maximum allowed daily cross 
        // chain volume, which is bad for liquidity providers.
        let current_units_inflow_x64 = U256(ctx.accounts.swap_pool_state_account.current_units_inflow_x64);
        if current_units_inflow_x64 > units_x64 {
            ctx.accounts.swap_pool_state_account.current_units_inflow_x64 = (current_units_inflow_x64 - units_x64).0;
        }
        else if !current_units_inflow_x64.is_zero() {    // NOTE: Optimisation carried over from EVM. Does it also work in Solana? (i.e. not writing to a state variable unless strictly required)
            ctx.accounts.swap_pool_state_account.current_units_inflow_x64 = [0, 0, 0, 0];            
        }

        emit!(OutSwapEvent {
            swap_pool: ctx.accounts.swap_pool_state_account.key(),
            target_pool: target_pool,
            target_asset_index: 0,  //TODO! fix after changing to_asset type
            target_withdrawer: destination,
            target_chain: chain,

            escrow_nonce,
        
            deposited_asset_mint: ctx.accounts.input_asset_mint.key(),
            depositor_asset_wallet: ctx.accounts.input_asset_wallet.key(),
        
            deposited_asset_amount: amount,
            withdrawn_pool_units_x64: units_x64.0,
            fees: 0,
        });

        Ok(())
    }

    pub fn out_swap_ack(
        ctx: Context<OutSwapAck>,
        escrow_nonce: u32          // Used in context
    ) -> Result<()> {

        let escrowed_amount = ctx.accounts.swap_escrow.amount;
        let escrowed_asset_index = ctx.accounts.swap_escrow.asset_index as usize;

        ctx.accounts.swap_pool_state_account.escrowed_assets[escrowed_asset_index] = 
            ctx.accounts.swap_pool_state_account.escrowed_assets[escrowed_asset_index].checked_sub(escrowed_amount).unwrap();

        emit!(OutSwapAckEvent {
            swap_pool: ctx.accounts.swap_pool_state_account.key(),
            escrow_nonce
        });

        Ok(())
    }

    pub fn out_swap_timeout(
        ctx: Context<OutSwapTimeout>,
        escrow_nonce: u32,          // Used in context
    ) -> Result<()> {

        let escrowed_amount = ctx.accounts.swap_escrow.amount;
        let escrowed_asset_index = ctx.accounts.swap_escrow.asset_index as usize;

        ctx.accounts.swap_pool_state_account.escrowed_assets[escrowed_asset_index] = 
            ctx.accounts.swap_pool_state_account.escrowed_assets[escrowed_asset_index].checked_sub(escrowed_amount).unwrap();
        
        // Transfer tokens to fallback_wallet
        token_utils::transfer_tokens_using_pda_authority(  //TODO use transfer_checked?
            escrowed_amount,
            ctx.accounts.swap_pool_asset_wallet.to_account_info(),
            ctx.accounts.fallback_wallet.to_account_info(),
            ctx.accounts.swap_pool_authority.to_account_info(),
            &[
                &ctx.accounts.swap_pool_state_account.key().to_bytes(),  // SwapPool state account
                &POOL_AUTHORITY_SEED[..],                                // Pool authority seed
                &[ctx.accounts.swap_pool_state_account.authority_bump]   // PDA bump
            ],
            ctx.accounts.token_program.to_account_info()
        );

        emit!(OutSwapTimeoutEvent {
            swap_pool: ctx.accounts.swap_pool_state_account.key(),
            escrow_nonce
        });

        Ok(())
    }

    pub fn in_swap(
        ctx: Context<InSwap>,
        output_asset_index: u8,
        units_x64: [u64; 4],
        min_output: u64
    ) -> Result<()> {   //TODO! set units_x64 to U256 (add serialisation/deserialisation to type)
        // assert sender is chain_interface

        let to_asset_index = ctx.accounts.swap_pool_state_account.get_asset_index(
            &ctx.accounts.output_asset_mint.key()
        ).unwrap();

        if output_asset_index as usize != to_asset_index {
            return Err(error!(ErrorCode::InvalidAssetMintAccount));
        }

        let output_balance = calculation_helpers::in_swap(
            U256(units_x64),
            U256::from(
                ctx.accounts.swap_pool_output_asset_wallet.amount
                    .checked_sub(ctx.accounts.swap_pool_state_account.escrowed_assets[to_asset_index]).unwrap()
            ),
            U256::from(ctx.accounts.swap_pool_state_account.pool_assets_weights[output_asset_index as usize]),
            U256(ctx.accounts.swap_pool_state_account.amplification_x64)
        )?.as_u64();      // U256 to u64 will panic if overflow

        if output_balance < min_output {
            return Err(error!(ErrorCode::SwapMinYieldNotFulfilled))
        }

        let current_timestamp: u64 = Clock::get().unwrap().unix_timestamp.try_into().unwrap();
        ctx.accounts.swap_pool_state_account.update_units_inflow(U256(units_x64), current_timestamp)?;

        // TODO verify
        ctx.accounts.swap_pool_state_account.unit_tracker_x64 = ctx.accounts.swap_pool_state_account.unit_tracker_x64
            .checked_sub(U256(units_x64).as_u128().try_into().unwrap()).unwrap();

        ctx.accounts.swap_pool_state_account.max_units_inflow_x64 = 
            U256(ctx.accounts.swap_pool_state_account.max_units_inflow_x64)
                .checked_sub(mul_x64(
                    U256(ctx.accounts.swap_pool_state_account.units_inflow_amplification_x64),
                    ctx.accounts.swap_pool_state_account.get_units_inflow_capacity(
                        ctx.accounts.swap_pool_output_asset_wallet.amount,           // NOTE: this amount is from BEFORE the asset transfer
                        ctx.accounts.swap_pool_output_asset_wallet.amount - output_balance,
                        to_asset_index
                    )?
                ).unwrap()).unwrap().0;
        //TODO! unitsDiff       line 888

        // Transfer output assets to the user
        token_utils::transfer_tokens_using_pda_authority(  //TODO use transfer_checked?
            output_balance,
            ctx.accounts.swap_pool_output_asset_wallet.to_account_info(),
            ctx.accounts.output_asset_wallet.to_account_info(),
            ctx.accounts.swap_pool_authority.to_account_info(),
            &[
                &ctx.accounts.swap_pool_state_account.key().to_bytes(),  // SwapPool state account
                &POOL_AUTHORITY_SEED[..],                                // Pool authority seed
                &[ctx.accounts.swap_pool_state_account.authority_bump]   // PDA bump
            ],
            ctx.accounts.token_program.to_account_info()
        );

        emit!(InSwapEvent {
            swap_pool: ctx.accounts.swap_pool_state_account.key(),
        
            withdrawn_asset_mint: ctx.accounts.output_asset_mint.key(),
            withdrawer_asset_wallet: ctx.accounts.output_asset_wallet.key(),
        
            deposited_pool_units_x64: units_x64,
            withdrawn_asset_amount: output_balance,
            fees: 0
        });

        Ok(())
    }



    // Liquidity Swaps **********************************************************************************************************

    pub fn out_liquidity_swap(
        ctx: Context<OutLiquiditySwap>,
        chain: u64,
        target_pool: Pubkey,
        destination: Pubkey,
        pool_tokens_amount: u64
    ) -> Result<()> {

        let swap_pool_state_account = &mut ctx.accounts.swap_pool_state_account;

        // Compute the total amount of liquidity units being transferred
        let mut out_liquidity_units_x64 = U256::from(0);
    
        for asset_index in 0..NUMASSETS {

            if swap_pool_state_account.pool_assets_mints[asset_index].eq(&Pubkey::default()) {
                break;
            }

            let asset_eq_balance = swap_pool_state_account.pool_assets_eq_balances[asset_index];

            let pool_tokens_for_asset = pool_tokens_amount
                .checked_mul(asset_eq_balance).unwrap()
                .checked_div(ctx.accounts.swap_pool_token_mint.supply).unwrap();

            let liquidity_for_asset_x64 = calculation_helpers::calc_out_liquidity_swap_x64(
                U256::from(pool_tokens_for_asset),
                U256::from(asset_eq_balance),
                U256::from(swap_pool_state_account.pool_assets_weights[asset_index]),
                U256(swap_pool_state_account.amplification_x64)
            )?;
            
            out_liquidity_units_x64 = out_liquidity_units_x64.checked_add(liquidity_for_asset_x64).unwrap();

            swap_pool_state_account.pool_assets_eq_balances[asset_index] = asset_eq_balance.checked_sub(pool_tokens_for_asset).unwrap();

        }

        // Burn pool tokens
        ctx.accounts.burn_pool_tokens_of_sender(pool_tokens_amount)?;

        // Perform cross chain swap
        let ccsi_program = ctx.accounts.ibc_interface_program.to_account_info(); // Check is the same as the stored program?
        let ccsi_swap_accounts = interface_accounts::CrossChainSwap {
            swap_pool_authority: ctx.accounts.swap_pool_authority.to_account_info(),
            interface_state_account: ctx.accounts.interface_state_account.to_account_info(),
            connection_state_account: ctx.accounts.connection_state_account.to_account_info(),
            polymerase_sender_program: ctx.accounts.polymerase_sender_program.to_account_info(),
            polymerase_ibc_data: ctx.accounts.polymerase_ibc_data.to_account_info(),
            polymerase_ibc_data_account_payer: ctx.accounts.polymerase_ibc_data_account_payer.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info()
        };

        let signer: &[&[&[u8]]] = &[&[
            ctx.accounts.swap_pool_state_account.to_account_info().key.as_ref(),
            &POOL_AUTHORITY_SEED[..],
            &[ctx.accounts.swap_pool_state_account.authority_bump]   // PDA bump
        ]];

        let ccsi_swap_ctx = CpiContext::new_with_signer(
            ccsi_program,
            ccsi_swap_accounts,
            signer
        );

        ibc_interface::cpi::cross_chain_liquidity_swap(
            ccsi_swap_ctx,
            chain,
            false,
            ctx.accounts.swap_pool_state_account.key(),
            target_pool,
            destination,
            out_liquidity_units_x64.0
        ).unwrap();

        // Correct the routing security limit. (To increase the maximum allowed daily volume)
        if ctx.accounts.swap_pool_state_account.current_liquidity_inflow > pool_tokens_amount {
            ctx.accounts.swap_pool_state_account.current_liquidity_inflow -= pool_tokens_amount
        }
        else if ctx.accounts.swap_pool_state_account.current_liquidity_inflow != 0 {    // NOTE: Optimisation carried over from EVM. Does it also work in Solana? (i.e. not writing to a state variable unless strictly required)
            ctx.accounts.swap_pool_state_account.current_liquidity_inflow = 0
        }

        emit!(OutLiquiditySwapEvent {
            swap_pool: ctx.accounts.swap_pool_state_account.key(),
            target_pool: target_pool,
            target_beneficiary: destination,
            target_chain: chain,
            pool_token_mint: ctx.accounts.swap_pool_token_mint.key(),
            source_pool_token_wallet: ctx.accounts.liquidity_provider_pool_token_wallet.key(),
            pool_token_amount: pool_tokens_amount,
            liquidity_units_x64: out_liquidity_units_x64.0,
            fees: 0,
        });


        Ok(())
    }


    pub fn in_liquidity_swap<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InLiquiditySwap<'info>>,
        liquidity_units_x64: [u64; 4]
    ) -> Result<()> {

        let total_pool_tokens;
        {
            let swap_pool_state_account = &mut ctx.accounts.swap_pool_state_account;

            let amplification = U256(swap_pool_state_account.amplification_x64);
            let mut aggregate_weight_x64 = U256::from(0);
            let one_minus_amp_x64 = ONE_X64.checked_sub(amplification).unwrap();
        
            for asset_index in 0..NUMASSETS {

                if swap_pool_state_account.pool_assets_mints[asset_index].eq(&Pubkey::default()) {
                    break;
                }

                let asset_eq_balance = swap_pool_state_account.pool_assets_eq_balances[asset_index];

                aggregate_weight_x64 = aggregate_weight_x64.checked_add(
                    U256::from(swap_pool_state_account.pool_assets_weights[asset_index]).checked_mul(
                        pow_x64(
                            U256::from(asset_eq_balance).shl(64), // No overflow guaranteed, going from u64 to u256
                            one_minus_amp_x64
                        ).unwrap()
                    ).unwrap()
                ).unwrap();
                
            }

            // Compute the 'received' pool tokens corresponding to the first asset of the pool
            let asset_0_pool_tokens = calculation_helpers::calc_in_liquidity_swap(
                U256(liquidity_units_x64),
                U256::from(swap_pool_state_account.pool_assets_eq_balances[0]),
                aggregate_weight_x64,
                amplification
            )?;

            // Compute the total pool tokens 'received' from the ones corresponding to the first asset
            let pool_tokens_supply = U256::from(ctx.accounts.swap_pool_token_mint.supply);
            total_pool_tokens = asset_0_pool_tokens
                .checked_mul(pool_tokens_supply).unwrap()
                .checked_div(U256::from(swap_pool_state_account.pool_assets_eq_balances[0])).unwrap();

            // Update the eq balances
            swap_pool_state_account.pool_assets_eq_balances[0] = swap_pool_state_account.pool_assets_eq_balances[0].checked_add(
                asset_0_pool_tokens.as_u64()
            ).unwrap();

            for asset_index in 1..NUMASSETS {

                if swap_pool_state_account.pool_assets_mints[asset_index].eq(&Pubkey::default()) {
                    break;
                }

                let asset_eq_balance = swap_pool_state_account.pool_assets_eq_balances[asset_index];

                swap_pool_state_account.pool_assets_eq_balances[asset_index] = asset_eq_balance.checked_add(
                    total_pool_tokens
                        .checked_mul(U256::from(asset_eq_balance)).unwrap()
                        .checked_div(pool_tokens_supply).unwrap()
                        .as_u64()
                ).unwrap();

            }
        }

        // Verify and update the security limit
        let current_timestamp: u64 = Clock::get().unwrap().unix_timestamp.try_into().unwrap();
        ctx.accounts.swap_pool_state_account.update_liquidity_units_inflow(
            total_pool_tokens.as_u64(),
            ctx.accounts.swap_pool_token_mint.supply,
            current_timestamp
        )?;
        
        ctx.accounts.mint_pool_tokens_for_recipient(total_pool_tokens.as_u64())?;

        emit!(InLiquiditySwapEvent {
            swap_pool: ctx.accounts.swap_pool_state_account.key(),
        
            pool_token_mint: ctx.accounts.swap_pool_token_mint.key(),
            target_pool_token_wallet: ctx.accounts.destination_pool_token_wallet.key(),
        
            pool_token_amount: total_pool_tokens.as_u64(),
            liquidity_units_x64: liquidity_units_x64,
            fees: 0
        });

        Ok(())
    }



    // Fees *********************************************************************************************************************

    pub fn distribute_fees(
        ctx: Context<DistributeFees>
    ) -> Result<()> {

        let swap_pool_state_account = &mut ctx.accounts.swap_pool_state_account;

        let amplification_x64 = U256(swap_pool_state_account.amplification_x64);
        let one_minus_amp_x64 = ONE_X64.checked_sub(amplification_x64).unwrap();

        let mut aggregate_weight_x64 = U256::from(0);
        let mut calc_outstanding_units_x64: i128 = 0;
    
        for asset_index in 0..NUMASSETS {

            if swap_pool_state_account.pool_assets_mints[asset_index].eq(&Pubkey::default()) {
                break;
            }

            let swap_pool_asset_wallet = &ctx.remaining_accounts[asset_index];  // Verified in 'create_swap_pool_asset_wallet' step
            
            // Make sure the swap_pool_asset_wallet provided matches the expected pda
            let seeds: &[&[u8]] = &[
                &swap_pool_state_account.key().to_bytes(),
                &swap_pool_state_account.pool_assets_mints[asset_index].key().to_bytes(),
                POOL_ASSET_WALLET_SEED,
                &[swap_pool_state_account.wallets_bumps[asset_index]]
            ];

            let expected_swap_pool_asset_wallet  = Pubkey::create_program_address(seeds, &ID).unwrap();

            if swap_pool_asset_wallet.key() != expected_swap_pool_asset_wallet {
                return Err(error!(ErrorCode::InvalidSwapPoolAssetWalletAccount));
            }
    
            let swap_pool_asset_wallet_as_token_account = TokenAccount::try_deserialize(&mut &swap_pool_asset_wallet.try_borrow_data()?[..])?;    

            let asset_eq_balance = swap_pool_state_account.pool_assets_eq_balances[asset_index];
            let asset_balance    = swap_pool_asset_wallet_as_token_account.amount;
            
            if asset_balance > asset_eq_balance {
                calc_outstanding_units_x64 = calc_outstanding_units_x64.checked_add(
                    calculation_helpers::calc_out_liquidity_swap_x64(
                        U256::from(asset_balance - asset_eq_balance),
                        U256::from(asset_eq_balance),
                        U256::from(swap_pool_state_account.pool_assets_weights[asset_index]),
                        one_minus_amp_x64
                    )?.try_into().unwrap()
                ).unwrap();
            }
            else {
                calc_outstanding_units_x64 = calc_outstanding_units_x64.checked_sub(
                    calculation_helpers::calc_out_liquidity_swap_x64(
                        U256::from(asset_eq_balance - asset_balance),
                        U256::from(asset_balance),
                        U256::from(swap_pool_state_account.pool_assets_weights[asset_index]),
                        one_minus_amp_x64
                    )?.try_into().unwrap()
                ).unwrap();
            }

            aggregate_weight_x64 = aggregate_weight_x64.checked_add(
                U256::from(swap_pool_state_account.pool_assets_weights[asset_index]).checked_mul(
                    pow_x64(
                        U256::from(asset_eq_balance).shl(64), // No overflow guaranteed, going from u64 to u256
                        one_minus_amp_x64
                    ).unwrap()
                ).unwrap()
            ).unwrap();
            
        }

        calc_outstanding_units_x64 = swap_pool_state_account.unit_tracker_x64.checked_sub(calc_outstanding_units_x64).unwrap();

        // Compute the pool tokens corresponding to the first asset of the pool
        let asset_0_pool_tokens = calculation_helpers::calc_in_liquidity_swap(
            U256::from(calc_outstanding_units_x64),
            U256::from(swap_pool_state_account.pool_assets_eq_balances[0]),
            aggregate_weight_x64,
            amplification_x64
        )?;

        // Compute the total pool tokens from the ones corresponding to the first asset
        let pool_tokens_supply = U256::from(ctx.accounts.swap_pool_token_mint.supply);
        let total_pool_tokens = asset_0_pool_tokens
            .checked_mul(pool_tokens_supply).unwrap()
            .checked_div(U256::from(swap_pool_state_account.pool_assets_eq_balances[0])).unwrap();

        // Update the eq balances
        swap_pool_state_account.pool_assets_eq_balances[0] = swap_pool_state_account.pool_assets_eq_balances[0].checked_add(
            asset_0_pool_tokens.as_u64()
        ).unwrap();

        for asset_index in 1..NUMASSETS {

            if swap_pool_state_account.pool_assets_mints[asset_index].eq(&Pubkey::default()) {
                break;
            }

            let asset_eq_balance = swap_pool_state_account.pool_assets_eq_balances[asset_index];

            swap_pool_state_account.pool_assets_eq_balances[asset_index] = asset_eq_balance.checked_add(
                asset_eq_balance.checked_mul(total_pool_tokens.as_u64()).unwrap()
            ).unwrap();

        }

        // TODO event

        Ok(())
    }

}



#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub setup_master: Signer<'info>,
    pub dao_authority: Signer<'info>,
    #[account(init, payer = setup_master, space = SwapPoolState::LEN)]  // NOTE: LEN includes discriminator
    pub swap_pool_state_account: Account<'info, SwapPoolState>,

    // Pool token accounts
    #[account(
        init,
        seeds = [
            &swap_pool_state_account.key().to_bytes(),
            POOL_TOKEN_MINT_SEED
        ],
        bump,
        payer = setup_master,
        mint::decimals = 0, // TODO!
        mint::authority = swap_pool_authority
    )]
    pub swap_pool_token_mint: Account<'info, Mint>,

    /// CHECK: Safe, using as an argument
    pub depositor_pool_token_wallet_authority: UncheckedAccount<'info>,
    #[account(
        init,
        payer = setup_master,
        token::mint = swap_pool_token_mint,
        token::authority = depositor_pool_token_wallet_authority
    )]
    pub depositor_pool_token_wallet: Account<'info, TokenAccount>, // The token wallet of the pool depositor where to transfer the pool tokens

    #[account(
        seeds = [
            &swap_pool_state_account.key().to_bytes(),
            POOL_AUTHORITY_SEED
        ],
        bump
    )]
    /// CHECK: Safe, as we are not reading from the account.
    pub swap_pool_authority: UncheckedAccount<'info>,


    // Sys and generics
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>
}

impl<'info> Initialize<'info> {

    pub fn create_swap_pool_asset_wallet(
        &self,
        asset_mint: AccountInfo<'info>,
        swap_pool_asset_wallet: AccountInfo<'info>
    ) -> Result<u8> {

        // Make sure the swap_pool_asset_wallet provided matches the expected pda
        let seeds: &[&[u8]] = &[
            &self.swap_pool_state_account.key().to_bytes(),
            &asset_mint.key().to_bytes(),
            POOL_ASSET_WALLET_SEED
        ];

        let (
            expected_swap_pool_asset_wallet, 
            swap_pool_asset_wallet_bump
        ) = Pubkey::find_program_address(seeds, &ID);

        if swap_pool_asset_wallet.key() != expected_swap_pool_asset_wallet {
            return Err(error!(ErrorCode::InvalidSwapPoolAssetWalletAccount));
        }

        // Initialize the pool asset wallet

        create_pda_token_account(
            swap_pool_asset_wallet.to_owned(),
            &[
                &self.swap_pool_state_account.key().to_bytes(),
                &asset_mint.key().to_bytes(),
                POOL_ASSET_WALLET_SEED,
                &[swap_pool_asset_wallet_bump]
            ],
            asset_mint.to_owned(),
            self.swap_pool_authority.to_account_info(),
            self.setup_master.to_account_info(),
            self.rent.to_owned(),
            self.token_program.to_account_info(),
            self.system_program.to_account_info()
        );

        Ok(swap_pool_asset_wallet_bump)
    }

    pub fn transfer_assets_from_depositor(
        &self,
        amount: u64,
        depositor_asset_wallet: AccountInfo<'info>,
        swap_pool_asset_wallet: AccountInfo<'info>
    ) -> Result<()> {
        
        // Transfer assets to the pool
        token_utils::transfer_tokens_using_pda_authority(  //TODO use transfer_checked?
            amount,
            depositor_asset_wallet,
            swap_pool_asset_wallet,
            self.swap_pool_authority.to_account_info(),
            &[
                &self.swap_pool_state_account.key().to_bytes(),             // SwapPool state account
                &POOL_AUTHORITY_SEED[..],                                   // Pool authority seed
                &[self.swap_pool_state_account.authority_bump.to_owned()]   // PDA bump
            ],
            self.token_program.to_account_info()
        );

        Ok(())
    }

    pub fn mint_pool_tokens_for_depositor(
        &self,
        amount: u64,
    ) -> Result<()> {
        token_utils::mint_tokens_using_pda_authority(
            amount,    
            self.depositor_pool_token_wallet.to_account_info(),
            self.swap_pool_token_mint.to_account_info(),
            self.swap_pool_authority.to_account_info(),
            &[
                    &self.swap_pool_state_account.key().to_bytes(),             // SwapPool state account
                    &POOL_AUTHORITY_SEED[..],                                   // Pool authority seed
                    &[self.swap_pool_state_account.authority_bump.to_owned()]   // PDA bump
                ],
            self.token_program.to_account_info()
        );

        Ok(())
    }
}


#[derive(Accounts)]
#[instruction(chain: u64, pool: Pubkey)]
pub struct CreateConnection<'info> {
    pub authority: Signer<'info>,
    #[account(
        constraint =
            swap_pool_state_account.setup_master == authority.key() || 
            swap_pool_state_account.dao_authority == authority.key()
            @ ErrorCode::InvalidAuthority
    )]
    pub swap_pool_state_account: Account<'info, SwapPoolState>,

    // Accounts for IBCInterface.createConnection()
    pub ibc_interface_program: Program<'info, IbcInterface>,
    /// CHECK: Safe, as we are passing it directly to the IBCInterface
    pub swap_pool_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub rent_payer: Signer<'info>,
    /// CHECK: Safe, as we are passing it directly to the IBCInterface
    pub interface_state_account: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Safe, as we are passing it directly to the IBCInterface
    pub connection_state_account: UncheckedAccount<'info>,
    /// CHECK: Safe, as we are passing it directly to the IBCInterface
    pub system_program: UncheckedAccount<'info>,
}


#[derive(Accounts)]
pub struct LinkIBCInterface<'info> {
    // ! Make sure the provided setup_master matches the one saved in swap_pool_state_account
    #[account(mut, has_one = setup_master @ ErrorCode::InvalidSetupAuthority)]
    pub swap_pool_state_account: Account<'info, SwapPoolState>,
    #[account(mut)]
    pub setup_master: Signer<'info>,
}


#[derive(Accounts)]
pub struct FinishSetup<'info> {
    // ! Make sure the provided setup_master matches the one saved in swap_pool_state_account
    #[account(mut, has_one = setup_master @ ErrorCode::InvalidSetupAuthority)]
    pub swap_pool_state_account: Account<'info, SwapPoolState>,
    #[account(mut)]
    pub setup_master: Signer<'info>,
}



#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub swap_pool_state_account: Account<'info, SwapPoolState>,

    #[account(mut)]
    pub depositor_pool_token_wallet: Account<'info, TokenAccount>, // The token wallet of the depositor where to transfer the pool tokens

    #[account(
        mut,
        seeds = [
            &swap_pool_state_account.key().to_bytes(),
            POOL_TOKEN_MINT_SEED
        ],
        bump = swap_pool_state_account.token_mint_bump
    )]
    pub swap_pool_token_mint: Box<Account<'info, Mint>>,

    #[account(
        seeds = [
            &swap_pool_state_account.key().to_bytes(),
            POOL_AUTHORITY_SEED
        ],
        bump = swap_pool_state_account.authority_bump
    )]
    /// CHECK: Safe, as we are not reading from the account.
    pub swap_pool_authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>
}



#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub swap_pool_state_account: Account<'info, SwapPoolState>,

    #[account(mut)]
    pub withdrawer_pool_token_wallet: Account<'info, TokenAccount>, // The token wallet of the withdrawer from where to burn the pool tokens

    #[account(
        mut,
        seeds = [
            &swap_pool_state_account.key().to_bytes(),
            POOL_TOKEN_MINT_SEED
        ],
        bump = swap_pool_state_account.token_mint_bump
    )]
    pub swap_pool_token_mint: Box<Account<'info, Mint>>,

    #[account(
        seeds = [
            &swap_pool_state_account.key().to_bytes(),
            POOL_AUTHORITY_SEED
        ],
        bump = swap_pool_state_account.authority_bump
    )]
    /// CHECK: Safe, as we are not reading from the account.
    pub swap_pool_authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>
}

impl<'info> Withdraw<'info> {
    
    pub fn burn_pool_tokens_of_withdrawer(
        &self,
        amount: u64,
    ) -> Result<()> {
        token_utils::burn_tokens_using_pda_authority(
            amount,    
            self.withdrawer_pool_token_wallet.to_account_info(),
            self.swap_pool_token_mint.to_account_info(),
            self.swap_pool_authority.to_account_info(),
            &[
                    &self.swap_pool_state_account.key().to_bytes(),             // SwapPool state account
                    &POOL_AUTHORITY_SEED[..],                                   // Pool authority seed
                    &[self.swap_pool_state_account.authority_bump.to_owned()]   // PDA bump
                ],
            self.token_program.to_account_info()
        );

        Ok(())
    }
}



#[derive(Accounts)]
pub struct LocalSwap<'info> {
    #[account(mut)]
    pub swap_pool_state_account: Box<Account<'info, SwapPoolState>>,

    // Input asset
    #[account()]
    pub input_asset_mint: Box<Account<'info, Mint>>,
    #[account(mut)]
    pub input_asset_wallet: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [
            &swap_pool_state_account.key().to_bytes(),
            &input_asset_mint.key().to_bytes(),
            POOL_ASSET_WALLET_SEED
        ],
        bump = swap_pool_state_account.wallets_bumps[
            swap_pool_state_account.get_asset_index(&input_asset_mint.key()).unwrap()
        ]
    )]
    pub swap_pool_input_asset_wallet: Account<'info, TokenAccount>,

    // Output asset
    #[account()]
    pub output_asset_mint: Box<Account<'info, Mint>>,
    #[account(mut)]
    pub output_asset_wallet: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [
            &swap_pool_state_account.key().to_bytes(),
            &output_asset_mint.key().to_bytes(),
            POOL_ASSET_WALLET_SEED
        ],
        bump = swap_pool_state_account.wallets_bumps[
            swap_pool_state_account.get_asset_index(&output_asset_mint.key()).unwrap()
        ]
    )]
    pub swap_pool_output_asset_wallet: Account<'info, TokenAccount>,

    // Asset wallet authority
    #[account(
        seeds = [
            &swap_pool_state_account.key().to_bytes(),
            POOL_AUTHORITY_SEED
        ],
        bump = swap_pool_state_account.authority_bump
    )]
    /// CHECK: Safe, as we are not reading from the account.
    pub swap_pool_authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>
}

#[derive(Accounts)]
#[instruction(
    chain: u64,
    target_pool: Pubkey,
    to_asset_index: u8,
    destination: Pubkey,
    amount: u64,
    min_output: [u64; 4],
    escrow_nonce: u32
)]
pub struct OutSwap<'info> {

    #[account(mut)]
    pub swap_pool_state_account: Box<Account<'info, SwapPoolState>>,

    // Input asset
    #[account()]
    pub input_asset_mint: Box<Account<'info, Mint>>,
    #[account(mut)]
    pub input_asset_wallet: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [
            &swap_pool_state_account.key().to_bytes(),
            &input_asset_mint.key().to_bytes(),
            POOL_ASSET_WALLET_SEED
        ],
        bump = swap_pool_state_account.wallets_bumps[
            swap_pool_state_account.get_asset_index(&input_asset_mint.key()).unwrap()
        ]
    )]
    pub swap_pool_input_asset_wallet: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,

    // Escrow account
    #[account(
        init,
        payer = swap_escrow_rent_payer,
        space = SwapEscrow::LEN,
        seeds = [
            &swap_pool_state_account.key().to_bytes().as_ref(),
            &escrow_nonce.to_be_bytes().as_ref(),
            &POOL_ESCROW_SEED,
        ],
        bump
    )]
    pub swap_escrow: Account<'info, SwapEscrow>,
    #[account(mut)]
    pub swap_escrow_rent_payer: Signer<'info>,

    pub ibc_interface_program: Program<'info, IbcInterface>,

    /// CHECK: Safe, as we are not reading from the account
    #[account(
        seeds = [
            &swap_pool_state_account.key().to_bytes(),
            POOL_AUTHORITY_SEED
        ],
        bump = swap_pool_state_account.authority_bump
    )]
    pub swap_pool_authority: UncheckedAccount<'info>,

    /// CHECK: Safe, as we are not reading from the account
    #[account()] // Make deterministic via seeds? Make it a PDA - owned by SwapPool?
    pub interface_state_account: UncheckedAccount<'info>,    //TODO where is this account verified?

    /// CHECK: Safe, as we are not reading from the account. Seeds checked by CCSI program
    #[account()]
    pub connection_state_account: UncheckedAccount<'info>,

    // Polymerase endpoint accounts
    /// CHECK: Safe, as we are not reading from the account
    pub polymerase_sender_program: UncheckedAccount<'info>,
    #[account(mut)]
    pub polymerase_ibc_data: Signer<'info>,
    #[account(mut)]
    pub polymerase_ibc_data_account_payer: Signer<'info>,    //TODO rename

    pub system_program: Program<'info, System> 

}



#[derive(Accounts)]
#[instruction(
    escrow_nonce: u32
)]
pub struct OutSwapAck<'info> {

    #[account(mut, has_one = ibc_interface @ ErrorCode::InvalidIBCInterfaceAccount)]
    pub swap_pool_state_account: Account<'info, SwapPoolState>,

    // Escrow account
    #[account(
        mut,
        close = swap_escrow_rent_payer,
        has_one = swap_escrow_rent_payer @ ErrorCode::InvalidSwapEscrowRentPayerAccount,
        seeds = [
            &swap_pool_state_account.key().to_bytes().as_ref(),
            &escrow_nonce.to_be_bytes().as_ref(),
            &POOL_ESCROW_SEED
        ],
        bump = swap_escrow.bump
    )]
    pub swap_escrow: Account<'info, SwapEscrow>,

    #[account(mut)]
    /// CHECK: Must match the one saved in swap_escrow
    pub swap_escrow_rent_payer: UncheckedAccount<'info>,

    // Chain Interface
    pub ibc_interface: Signer<'info>,

    pub system_program: Program<'info, System> 

}



#[derive(Accounts)]
#[instruction(
    escrow_nonce: u32
)]
pub struct OutSwapTimeout<'info> {

    #[account(mut, has_one = ibc_interface @ ErrorCode::InvalidIBCInterfaceAccount)]
    pub swap_pool_state_account: Box<Account<'info, SwapPoolState>>,

    // Returned asset
    #[account()]
    pub asset_mint: Box<Account<'info, Mint>>,
    #[account(mut)]
    pub fallback_wallet: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [
            &swap_pool_state_account.key().to_bytes(),
            &asset_mint.key().to_bytes(),
            POOL_ASSET_WALLET_SEED
        ],
        bump = swap_pool_state_account.wallets_bumps[
            swap_pool_state_account.get_asset_index(&asset_mint.key()).unwrap()
        ]
    )]
    pub swap_pool_asset_wallet: Account<'info, TokenAccount>,

    /// CHECK: Safe, as we are not reading from the account
    #[account(
        seeds = [
            &swap_pool_state_account.key().to_bytes(),
            POOL_AUTHORITY_SEED
        ],
        bump = swap_pool_state_account.authority_bump
    )]
    pub swap_pool_authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,

    // Escrow account
    #[account(
        mut,
        close = swap_escrow_rent_payer,
        has_one = swap_escrow_rent_payer @ ErrorCode::InvalidSwapEscrowRentPayerAccount,
        has_one = fallback_wallet @ ErrorCode::InvalidFallbackWalletAccount,
        seeds = [
            &swap_pool_state_account.key().to_bytes().as_ref(),
            &escrow_nonce.to_be_bytes().as_ref(),
            &POOL_ESCROW_SEED
        ],
        bump = swap_escrow.bump
    )]
    pub swap_escrow: Account<'info, SwapEscrow>,

    #[account(mut)]
    /// CHECK: Must match the one saved in swap_escrow
    pub swap_escrow_rent_payer: UncheckedAccount<'info>,

    // Chain Interface
    pub ibc_interface: Signer<'info>,

    pub system_program: Program<'info, System> 

}


#[derive(Accounts)]
pub struct InSwap<'info> {
    #[account(mut, has_one = ibc_interface @ ErrorCode::InvalidIBCInterfaceAccount)]
    pub swap_pool_state_account: Box<Account<'info, SwapPoolState>>,

    // Output asset
    #[account()]
    pub output_asset_mint: Box<Account<'info, Mint>>,
    #[account(mut)]
    pub output_asset_wallet: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        seeds = [
            &swap_pool_state_account.key().to_bytes(),
            &output_asset_mint.key().to_bytes(),
            POOL_ASSET_WALLET_SEED
        ],
        bump = swap_pool_state_account.wallets_bumps[
            swap_pool_state_account.get_asset_index(&output_asset_mint.key()).unwrap()
        ]
    )]
    pub swap_pool_output_asset_wallet: Account<'info, TokenAccount>,

    // Asset wallet authority
    #[account(
        seeds = [
            &swap_pool_state_account.key().to_bytes(),
            POOL_AUTHORITY_SEED
        ],
        bump = swap_pool_state_account.authority_bump
    )]
    /// CHECK: Safe, as we are not reading from the account.
    pub swap_pool_authority: UncheckedAccount<'info>,

    // Chain Interface
    pub ibc_interface: Signer<'info>,

    pub token_program: Program<'info, Token>
}


#[derive(Accounts)]
pub struct OutLiquiditySwap<'info> {

    #[account(mut)]
    pub swap_pool_state_account: Box<Account<'info, SwapPoolState>>,

    // Pool tokens
    #[account(mut)]
    pub liquidity_provider_pool_token_wallet: Account<'info, TokenAccount>, // The token wallet of the withdrawer from where to burn the pool tokens

    #[account(
        mut,
        seeds = [
            &swap_pool_state_account.key().to_bytes(),
            POOL_TOKEN_MINT_SEED
        ],
        bump = swap_pool_state_account.token_mint_bump
    )]
    pub swap_pool_token_mint: Box<Account<'info, Mint>>,

    pub token_program: Program<'info, Token>,

    pub ibc_interface_program: Program<'info, IbcInterface>,

    /// CHECK: Safe, as we are not reading from the account
    #[account(
        seeds = [
            &swap_pool_state_account.key().to_bytes(),
            POOL_AUTHORITY_SEED
        ],
        bump = swap_pool_state_account.authority_bump
    )]
    pub swap_pool_authority: UncheckedAccount<'info>,

    /// CHECK: Safe, as we are not reading from the account
    #[account()] // Make deterministic via seeds? Make it a PDA - owned by SwapPool?
    pub interface_state_account: UncheckedAccount<'info>,    //TODO where is this account verified?

    /// CHECK: Safe, as we are not reading from the account. Seeds checked by CCSI program
    #[account()]
    pub connection_state_account: UncheckedAccount<'info>,

    // Polymerase endpoint accounts
    /// CHECK: Safe, as we are not reading from the account
    pub polymerase_sender_program: UncheckedAccount<'info>,
    #[account(mut)]
    pub polymerase_ibc_data: Signer<'info>,
    #[account(mut)]
    pub polymerase_ibc_data_account_payer: Signer<'info>,    //TODO rename
    /// CHECK: Safe, as we are not reading from the account
    pub system_program: UncheckedAccount<'info> // Intentionally not Program<'info, System>, as we do not want to run any checks on the account (and hence minimize gas fees)

}

impl<'info> OutLiquiditySwap<'info> {
    
    pub fn burn_pool_tokens_of_sender(
        &self,
        amount: u64,
    ) -> Result<()> {
        token_utils::burn_tokens_using_pda_authority(
            amount,    
            self.liquidity_provider_pool_token_wallet.to_account_info(),
            self.swap_pool_token_mint.to_account_info(),
            self.swap_pool_authority.to_account_info(),
            &[
                    &self.swap_pool_state_account.key().to_bytes(),             // SwapPool state account
                    &POOL_AUTHORITY_SEED[..],                                   // Pool authority seed
                    &[self.swap_pool_state_account.authority_bump.to_owned()]   // PDA bump
                ],
            self.token_program.to_account_info()
        );

        Ok(())
    }
}



#[derive(Accounts)]
pub struct InLiquiditySwap<'info> {
    #[account(mut, has_one = ibc_interface @ ErrorCode::InvalidIBCInterfaceAccount)]
    pub swap_pool_state_account: Box<Account<'info, SwapPoolState>>,

    #[account(
        mut,
        seeds = [
            &swap_pool_state_account.key().to_bytes(),
            POOL_TOKEN_MINT_SEED
        ],
        bump = swap_pool_state_account.token_mint_bump
    )]
    pub swap_pool_token_mint: Box<Account<'info, Mint>>,
    #[account(mut)]
    pub destination_pool_token_wallet: Account<'info, TokenAccount>, // The token wallet of the withdrawer from where to burn the pool tokens

    // Asset wallet authority
    #[account(
        seeds = [
            &swap_pool_state_account.key().to_bytes(),
            POOL_AUTHORITY_SEED
        ],
        bump = swap_pool_state_account.authority_bump
    )]
    /// CHECK: Safe, as we are not reading from the account.
    pub swap_pool_authority: UncheckedAccount<'info>,

    // Chain Interface
    pub ibc_interface: Signer<'info>,

    pub token_program: Program<'info, Token>
}

impl<'info> InLiquiditySwap<'info> {
    
    pub fn mint_pool_tokens_for_recipient(
        &self,
        amount: u64,
    ) -> Result<()> {
        token_utils::mint_tokens_using_pda_authority(
            amount,    
            self.destination_pool_token_wallet.to_account_info(),
            self.swap_pool_token_mint.to_account_info(),
            self.swap_pool_authority.to_account_info(),
            &[
                    &self.swap_pool_state_account.key().to_bytes(),             // SwapPool state account
                    &POOL_AUTHORITY_SEED[..],                                   // Pool authority seed
                    &[self.swap_pool_state_account.authority_bump.to_owned()]   // PDA bump
                ],
            self.token_program.to_account_info()
        );

        Ok(())
    }
}



#[derive(Accounts)]
pub struct DistributeFees<'info> {
    pub swap_pool_state_account: Box<Account<'info, SwapPoolState>>,

    #[account(
        mut,
        seeds = [
            &swap_pool_state_account.key().to_bytes(),
            POOL_TOKEN_MINT_SEED
        ],
        bump = swap_pool_state_account.token_mint_bump
    )]
    pub swap_pool_token_mint: Box<Account<'info, Mint>>
}



#[account]
pub struct SwapPoolState {
    pub setup_master: Pubkey,
    pub dao_authority: Pubkey,     // TODO to be replaced/checked once the DAO structure gets finalized
    pub ibc_interface: Pubkey,
    pub pool_assets_mints: [Pubkey; NUMASSETS],
    pub pool_assets_weights: [u64; NUMASSETS],
    pub pool_assets_eq_balances: [u64; NUMASSETS],
    pub amplification_x64: [u64; 4],

    pub escrowed_assets: [u64; NUMASSETS],

    pub max_units_inflow_x64: [u64; 4],
    pub units_inflow_amplification_x64: [u64; 4],
    pub current_units_inflow_x64: [u64; 4],
    pub current_units_inflow_timestamp: u64,

    pub current_liquidity_inflow: u64,
    pub current_liquidity_inflow_timestamp: u64,

    pub unit_tracker_x64: i128,

    pub authority_bump: u8,
    pub token_mint_bump: u8,
    pub wallets_bumps: [u8; NUMASSETS]
}

impl SwapPoolState {
    pub const LEN: usize = 
        8               // discriminator
        + 32            // setup_master
        + 32            // dao_authority
        + 32            // ibc_interface
        + 32*NUMASSETS  // pool_assets_mints
        + 8*NUMASSETS   // pool_assets_weights
        + 8*NUMASSETS   // pool_assets_eq_balances
        + 32            // amplification_x64
        + 8*NUMASSETS   // escrowed_assets
        + 8*4           // max_units_inflow_x64
        + 8*4           // units_inflow_amplification_x64
        + 8*4           // current_units_inflow_x64
        + 8             // current_units_inflow_timestamp
        + 8             // current_liquidity_inflow
        + 8             // current_liquidity_inflow_timestamp
        + 4*4           // unit_tracker_x64
        + 1             // authority_bump
        + 1             // token_mint_bump
        + NUMASSETS;    // wallets_bumps


    pub fn get_asset_index(&self, asset: &Pubkey) -> Option<usize> {
        self.pool_assets_mints.iter().position(|a| { a == asset })
    }


    pub fn update_units_inflow(
        &mut self,
        units_inflow_x64: U256,
        current_timestamp: u64
    ) -> Result<()> {

        let max_units_inflow_x64 = U256(self.max_units_inflow_x64);

        // If more time has passed since the last update than DECAYRATE, the current inflow state does not matter (it has fully decayed)
        if current_timestamp > self.current_units_inflow_timestamp + DECAYRATE {
            if units_inflow_x64 > max_units_inflow_x64 {
                return Err(error!(ErrorCode::SwapLimitExceeded));
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
                return Err(error!(ErrorCode::SwapLimitExceeded));
            }

            self.current_units_inflow_x64 = units_inflow_x64.0;
        }
        // If some of the current inflow still matters
        else {
            let new_net_units_inflow_x64 = (current_units_inflow_x64 - decayed_inflow).checked_add(units_inflow_x64).unwrap();  // Substraction is safe, as current_units_inflow_x64 > decayed_inflow is guaranteed by if statement

            if new_net_units_inflow_x64 > max_units_inflow_x64 {
                return Err(error!(ErrorCode::SwapLimitExceeded));
            }

            self.current_units_inflow_x64 = new_net_units_inflow_x64.0;
        }

        self.current_units_inflow_timestamp = current_timestamp;

        Ok(())
    }


    pub fn update_liquidity_units_inflow(
        &mut self,
        pool_tokens_flow: u64,
        current_pool_token_supply: u64,
        current_timestamp: u64
    ) -> Result<()> {

        // Allows 1/3 of the pool to be drained through liquidity swaps
        let max_pool_tokens_flow = current_pool_token_supply / 2;

        // If more time has passed since the last update than DECAYRATE, the current inflow state does not matter (it has fully decayed)
        if current_timestamp > self.current_liquidity_inflow_timestamp + DECAYRATE {
            if pool_tokens_flow > max_pool_tokens_flow {
                return Err(error!(ErrorCode::LiquiditySwapLimitExceeded));
            }

            self.current_liquidity_inflow           = pool_tokens_flow;
            self.current_liquidity_inflow_timestamp = current_timestamp;

            return Ok(());
        }

        // Compute how much inflow has decayed since last update
        let decayed_inflow = max_pool_tokens_flow.checked_mul(
            current_timestamp.checked_sub(self.current_liquidity_inflow_timestamp).unwrap().try_into().unwrap()  // TODO checked_sub required?
        ).unwrap() / DECAYRATE;

        // If the current inflow is less then the (max allowed) decayed one
        if self.current_liquidity_inflow <= decayed_inflow {
            if pool_tokens_flow > max_pool_tokens_flow {
                return Err(error!(ErrorCode::LiquiditySwapLimitExceeded));
            }

            self.current_liquidity_inflow = pool_tokens_flow;
        }
        // If some of the current inflow still matters
        else {
            let new_net_liquidity_inflow = (self.current_liquidity_inflow - decayed_inflow).checked_add(pool_tokens_flow).unwrap();  // Substraction is safe, as current_liquidity_inflow > decayed_inflow is guaranteed by if statement

            if new_net_liquidity_inflow > max_pool_tokens_flow {
                return Err(error!(ErrorCode::LiquiditySwapLimitExceeded));
            }

            self.current_liquidity_inflow = new_net_liquidity_inflow;
        }

        self.current_liquidity_inflow_timestamp = current_timestamp;

        Ok(())
    }

    pub fn get_units_inflow_capacity(
        &self,
        old_balance: u64,
        new_balance: u64,
        asset_index: usize
    ) -> Result<U256> {
        if old_balance == new_balance {
            return Ok(U256([0, 0, 0, 0]))
        }

        let one_minus_amp = ONE_X64 - U256(self.amplification_x64);

        if old_balance < new_balance {
            return Ok(
                U256::from(self.pool_assets_weights[asset_index]).checked_mul(
                    pow_x64(U256::from(new_balance).shl(64), one_minus_amp).unwrap().checked_sub(
                        pow_x64(U256::from(old_balance).shl(64), one_minus_amp).unwrap()
                    ).unwrap()
                ).unwrap()
            );
        }

        Ok(
            U256::from(self.pool_assets_weights[asset_index]).checked_mul(
                pow_x64(U256::from(old_balance).shl(64), one_minus_amp).unwrap().checked_sub(
                    pow_x64(U256::from(new_balance).shl(64), one_minus_amp).unwrap()
                ).unwrap()
            ).unwrap()
        )
    }

}


#[account]
pub struct SwapEscrow {
    pub amount: u64,
    pub asset_index: u8,
    pub fallback_wallet: Pubkey,
    pub swap_escrow_rent_payer: Pubkey,
    pub bump: u8,
}

impl SwapEscrow {
    pub const LEN: usize = 
        8       // discriminator
        + 8     // amount
        + 1     // asset_index
        + 32    // fallback_wallet
        + 32    // rent_receiver
        + 1;    // bump
}


// Events ***********************************************************************************************************************

// TODO add assets weights to events?
// TODO add asset mints to deposit event?
#[event]
pub struct DepositEvent {
    swap_pool: Pubkey,

    deposited_asset_amounts: [u64; NUMASSETS],
    depositor_asset_wallets: [Pubkey; NUMASSETS],

    withdrawn_pool_token_amount: u64,
    depositor_pool_token_wallet: Pubkey
}

// TODO add asset mints to withdraw event?
#[event]
pub struct WithdrawEvent {
    swap_pool: Pubkey,

    withdrawn_asset_amounts: [u64; NUMASSETS],
    withdrawer_asset_wallets: [Pubkey; NUMASSETS],

    burnt_pool_token_amount: u64,
    withdrawer_pool_token_wallet: Pubkey
}

#[event]
pub struct LocalSwapEvent {
    swap_pool: Pubkey,

    deposited_asset_mint: Pubkey,
    depositor_asset_wallet: Pubkey,

    withdrawn_asset_mint: Pubkey,
    withdrawer_asset_wallet: Pubkey,

    deposited_asset_amount: u64,
    withdrawn_asset_amount: u64,
    fees: u64
}

#[event]
pub struct InSwapEvent {
    swap_pool: Pubkey,
    //TODO add source data?

    withdrawn_asset_mint: Pubkey,
    withdrawer_asset_wallet: Pubkey,

    deposited_pool_units_x64: [u64; 4],
    withdrawn_asset_amount: u64,
    fees: u64
}

#[event]
pub struct OutSwapEvent {
    swap_pool: Pubkey,
    target_pool: Pubkey,
    target_asset_index: u8,
    target_withdrawer: Pubkey,
    target_chain: u64,

    escrow_nonce: u32,

    deposited_asset_mint: Pubkey,
    depositor_asset_wallet: Pubkey,

    deposited_asset_amount: u64,
    withdrawn_pool_units_x64: [u64; 4],
    fees: u64,
}

#[event]
pub struct OutSwapAckEvent {
    swap_pool: Pubkey,
    escrow_nonce: u32
}

#[event]
pub struct OutSwapTimeoutEvent {
    swap_pool: Pubkey,
    escrow_nonce: u32
}

#[event]
pub struct InLiquiditySwapEvent {
    swap_pool: Pubkey,
    //TODO add source data?

    pool_token_mint: Pubkey,
    target_pool_token_wallet: Pubkey,

    pool_token_amount: u64,
    liquidity_units_x64: [u64; 4],
    fees: u64
}

#[event]
pub struct OutLiquiditySwapEvent {
    swap_pool: Pubkey,
    target_pool: Pubkey,
    target_beneficiary: Pubkey,
    target_chain: u64,

    pool_token_mint: Pubkey,
    source_pool_token_wallet: Pubkey,

    pool_token_amount: u64,
    liquidity_units_x64: [u64; 4],
    fees: u64,
}



// Errors ***********************************************************************************************************************

#[error_code]
pub enum ErrorCode {

    // Setup
    #[msg("Invalid amplification constant for pool (must be set to 1).")]   // Unused in this program, here so that error codes match between amplified/non-amplified programs
    InvalidAmplification,
    
    #[msg("All asset weights must be non-zero.")]
    InvalidAssetWeight,
    
    #[msg("There must be at least one asset with a non-zero balance to create the pool.")]
    NoAssetsProvided,
    
    #[msg("The swap interface has not been linked to the pool.")]
    SwapInterfaceNotLinked, //TODO allow fully local pools?
    
    
    // Swaps
    #[msg("Swap yield is less than the specified minimum.")]
    SwapMinYieldNotFulfilled,
    
    #[msg("Swap amount exceeds pool limit.")]
    SwapLimitExceeded,
    
    #[msg("Liquidity swap amount exceeds pool limit.")]
    LiquiditySwapLimitExceeded,
    
    
    // Accounts
    #[msg("The authority does not match the expected one.")]
    InvalidAuthority,

    #[msg("The setup authority does not match the expected one.")]
    InvalidSetupAuthority,
    
    #[msg("The provided asset mint does not match the expected one/is not contained in the pool.")]
    InvalidAssetMintAccount,
    
    #[msg("The swap pool asset wallet account provided does not match the expected one.")]
    InvalidSwapPoolAssetWalletAccount,
    
    #[msg("The fallback wallet account provided does not match the expected one.")]
    InvalidFallbackWalletAccount,
    
    #[msg("The provided swap escrow rent payer does not match with the original payer of the escrow account rent.")]
    InvalidSwapEscrowRentPayerAccount,
    
    #[msg("Unexpected number of remaining accounts provided.")]
    InvalidRemainingAccountsCount,
    
    #[msg("The provided IBC state account does not match the expected one.")]
    InvalidIBCInterfaceAccount,
}