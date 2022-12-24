use anchor_lang::prelude::*;
use anchor_lang::prelude::borsh;
use anchor_lang::solana_program::entrypoint::ProgramResult;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke_signed};
use borsh::BorshSerialize;
use std::convert::TryInto;

use spl_token::ID as TOKEN_PROGRAM_ID;

use polymerase_register::cpi::accounts::RegisterPort;
use polymerase_register::{program::PolymeraseRegister};
use polymerase_sender::cpi::accounts::SendIbcPacket;
use polymerase_sender::program::PolymeraseSender;

//TODO! change id
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLCC");


// CPI sighashs
const SIGHASH_IN_SWAP           : [u8; 8] = [78, 13, 161, 247, 110, 249, 148, 65];
const SIGHASH_IN_LIQUIDITY_SWAP : [u8; 8] = [148, 187, 47, 168, 127, 65, 48, 95];
const SIGHASH_OUT_SWAP_ACK      : [u8; 8] = [25, 202, 103, 138, 82, 99, 175, 108];
const SIGHASH_OUT_SWAP_TIMEOUT  : [u8; 8] = [221, 78, 207, 141, 128, 224, 34, 43];

// Define swap contexts
const CTX_ASSET_SWAP            : u8 = 0x00;
const CTX_ASSET_APPROX_SWAP     : u8 = 0x01;
const CTX_LIQUIDITY_SWAP        : u8 = 0x02;
const CTX_LIQUIDITY_APPROX_SWAP : u8 = 0x03;

#[program]
pub mod ibc_interface {

    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        swap_pool_state: Pubkey,                      // Used in context to derive interface_state account
        swap_pool_authority: Pubkey,
        polymerase_endpoint_program: Pubkey
    ) -> Result<()> {
        ctx.accounts.interface_state_account.configurator = ctx.accounts.configurator.key();
        ctx.accounts.interface_state_account.swap_pool = swap_pool_state;
        ctx.accounts.interface_state_account.swap_pool_authority = swap_pool_authority;
        ctx.accounts.interface_state_account.polymerase_endpoint_program = polymerase_endpoint_program;
        ctx.accounts.interface_state_account.interface_state_account_bump = ctx.bumps.get("interface_state_account").unwrap().to_owned();
        Ok(())
    }

    pub fn set_configurator(ctx: Context<SetConfigurator>, new_configurator: Pubkey) -> Result<()> {
        ctx.accounts.interface_state_account.configurator = new_configurator;
        Ok(())
    }

    // ! TODO must be required before completing setup
    pub fn register_receiver(
        ctx: Context<RegisterReceiver>
    ) -> Result<()> {

        let register_accounts = RegisterPort {
            port_registration: ctx.accounts.port_registration.to_account_info(),
            payer: ctx.accounts.payer.to_account_info(),
            registrant: ctx.accounts.interface_state_account.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        };

        // TODO: Figure out how to set port registration writer/signer flags
        //        to be applied during on_receive callback. Currently the flags
        //        are inherited from the account type. OR perhaps these accounts
        //        should be read-only??

        let signer: &[&[&[u8]]] = &[&[
            &ctx.accounts.interface_state_account.swap_pool.key().to_bytes(),
            &[ctx.accounts.interface_state_account.interface_state_account_bump]   // PDA bump
        ]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.register_program.to_account_info(),
            register_accounts,
            signer
        );

        polymerase_register::cpi::register_port(
            cpi_ctx, 
            ibc_interface::ID, 
            vec![], 
            String::from("on_recv_packet"),
            vec![
                ctx.accounts.interface_state_account.key(),             //interface_state_account
                ctx.accounts.interface_state_account.swap_pool.key(),   //swap_pool_state
                ctx.accounts.swap_pool.owner.key(),                     //swap_pool_program
                TOKEN_PROGRAM_ID                                        //token_program
            ],
            vec![
                false,                                                  //interface_state_account
                false,                                                  //swap_pool_state
                false,                                                  //swap_pool_program
                false                                                   //token_program
            ],
            vec![
                false,                                                  //interface_state_account
                true,                                                   //swap_pool_state
                false,                                                  //swap_pool_program
                false                                                   //token_program
            ]
        )
        // Ok(())
    }

    pub fn create_connection(
        ctx: Context<CreateConnection>,
        chain: u64,                     // Used in the CreateConnection context
        pool: Pubkey,                   // Used in the CreateConnection context
        interface_program: Pubkey       // target crosschain program
    ) -> Result<()> {
        ctx.accounts.connection_state_account.connected_interface_program   = interface_program;
        ctx.accounts.connection_state_account.connection_state_account_bump = ctx.bumps.get("connection_state_account").unwrap().to_owned();
        Ok(())
    }

    pub fn cross_chain_swap(
        ctx: Context<CrossChainSwap>,
        chain: u64,
        approx: bool,
        source_pool: Pubkey,    // Used to derive the ibc_interface_state account
        target_pool: Pubkey,
        target_asset_index: u8,
        destination: Pubkey,
        transferred_units_x64: [u64; 4],
        min_output: [u64; 4],
        source_swap_id: u32
    ) -> Result<()> {
        /*
            Message Format

            0       context            : u8
            1-32    source_pool        : Pubkey
            33-64   target_pool        : Pubkey
            65-96   destination        : Pubkey
            97-128  units              : u256 (as 4 u64)
            129     target_asset_index : u8
            130-161 min_output         : u256 (as 4 u64)
            162-165 source_swap_id     : u32
        */

        // ! Swap pool authority verification in CrossChainSwap context
        // !    ==> Only allow the authorized swap pool authority holder to execute this function

        // ! The target pool connection state (chain + pool) is checked in the context

        let mut message = vec![0; 166];

        match approx {
            false => message[0] = CTX_ASSET_SWAP,
            true =>  message[0] = CTX_ASSET_APPROX_SWAP
        }
        
        message[1..33]      // Context
            .copy_from_slice(&ctx.accounts.interface_state_account.key().to_bytes());
        
        message[33..65]     // Source pool
            .copy_from_slice(&target_pool.to_bytes());
        
        message[65..97]     // Target pool
            .copy_from_slice(&destination.to_bytes());
        
        message[97..129]    // Destination
            .copy_from_slice(&transferred_units_x64
                .iter()
                .flat_map(|el| {el.to_be_bytes()})
                .collect::<Vec<u8>>()[0..32]
            ); //TODO! verify + efficiency + use big_endian?

        message[129..130]   // Units
            .copy_from_slice(&[target_asset_index]);

        message[130..162]   // Minimum output
            .copy_from_slice(&min_output
                .iter()
                .flat_map(|el| {el.to_be_bytes()})
                .collect::<Vec<u8>>()[0..32]
            ); //TODO! verify + efficiency + use big_endian?
        
        message[162..166]   // Source swap id
            .copy_from_slice(&source_swap_id.to_be_bytes());

        // Invoke call_multichain
        //TODO create external function for the following code

        let signer: &[&[&[u8]]] = &[&[
            &source_pool.to_bytes(),
            &[ctx.accounts.interface_state_account.interface_state_account_bump]  // PDA bump
        ]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.polymerase_sender_program.to_account_info(),
            SendIbcPacket {
                ibc_data: ctx.accounts.polymerase_ibc_data.to_account_info(),
                payer: ctx.accounts.polymerase_ibc_data_account_payer.to_account_info(),
                sender: ctx.accounts.interface_state_account.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
            },
            signer
        );
        let channel_id: [u8; 32] = [1; 32]; // ! TODO channel_id from connection_state_account, linked to 'chain' and 'connection_state_account.connected_interface_program'

        let timeout_block_height: u64 = 0;
        polymerase_sender::cpi::send_ibc_packet(cpi_ctx, channel_id, message, timeout_block_height)    // ! TODO must add dynamic accounts

    }


    pub fn cross_chain_liquidity_swap(
        ctx: Context<CrossChainSwap>,
        chain: u64,
        approx: bool,
        source_pool: Pubkey,    // Used to derive the ibc_interface_state account
        target_pool: Pubkey,
        destination: Pubkey,
        transferred_liquidity_units_x64: [u64; 4]
    ) -> Result<()> {
        /*
            Message Format

            0       context            : u8
            1-32    source_pool        : Pubkey
            33-64   target_pool        : Pubkey
            65-96   destination        : Pubkey
            97-128  units              : u256 (as 4 u64)
            129-XX  TBD
        */

        // ! Swap pool authority verification in CrossChainSwap context
        // !    ==> Only allow the authorized swap pool authority holder to execute this function

        // ! The target pool connection state (chain + pool) is checked in the context

        assert!(!approx, "Cross chain liquidity swap does not support approx mode.");

        let mut message = vec![0; 130];

        message[0] = CTX_LIQUIDITY_SWAP;     // Context

        message[1..33]                  // Source pool
            .copy_from_slice(&ctx.accounts.interface_state_account.key().to_bytes());

        message[33..65]                 // Target pool
            .copy_from_slice(&target_pool.to_bytes());

        message[65..97]                 // Destination
            .copy_from_slice(&destination.to_bytes());

        message[97..129]                // Units
            .copy_from_slice(&transferred_liquidity_units_x64
            .iter()
            .flat_map(|el| {el.to_be_bytes()})
            .collect::<Vec<u8>>()[0..32]
        ); //TODO! verify + efficiency + use big_endian?


        // Invoke call_multichain
        //TODO create external function for the following code

        let signer: &[&[&[u8]]] = &[&[
            &source_pool.to_bytes(),
            &[ctx.accounts.interface_state_account.interface_state_account_bump]  // PDA bump
        ]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.polymerase_sender_program.to_account_info(),
            SendIbcPacket {
                ibc_data: ctx.accounts.polymerase_ibc_data.to_account_info(),
                payer: ctx.accounts.polymerase_ibc_data_account_payer.to_account_info(),
                sender: ctx.accounts.interface_state_account.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
            },
            signer
        );
        let channel_id: [u8; 32] = [1; 32]; // ! TODO channel_id from connection_state_account, linked to 'chain' and 'connection_state_account.connected_interface_program'

        let timeout_block_height: u64 = 0;
        polymerase_sender::cpi::send_ibc_packet(cpi_ctx, channel_id, message, timeout_block_height)    // ! TODO must add dynamic accounts

    }


    pub fn on_recv_packet<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, OnIBCInvocation<'info>>,
        message: Vec<u8>
    ) -> Result<()> {

        // ! TODO:
        // ! PolymeraseInterface authority verification in OnRecvPacket context
        // !    ==> Only allow the authorized PolymeraseInterface authority holder to execute this function

        // let chain //TODO

        let context: u8 = u8::from_be_bytes(message[..1].try_into().unwrap());
        let source_pool: Pubkey = Pubkey::new(&message[1..33]);
        let target_pool: Pubkey = Pubkey::new(&message[33..65]);

        //TODO move to context?
        // Check the provided SwapPool program is the expected one
        if !ctx.accounts.swap_pool_program.key.eq(&ctx.accounts.swap_pool.owner) {
            return Err(error!(ErrorCode::InvalidSwapPoolProgram));
        }

        //TODO!
        // //TODO move to context?
        // // Check the source chain + pool is valid, i.e. it is a connected and enabled pool connection
        // if !ctx.accounts.connection_state_account.state {
        //     return Err(error!(ErrorCode::SourcePoolNotConnected));
        // }

        match context {

            CTX_ASSET_SWAP | CTX_ASSET_APPROX_SWAP => {
                // Invoke in_swap of the target pool
                /*
                    Message Format
                    0       context            : u8
                    1-32    source_pool        : Pubkey
                    33-64   target_pool        : Pubkey
                    65-96   destination        : Pubkey
                    97-128  units              : u256 (as 4 u64)
                    129     target_asset_index : u8
                    130-161 min_output         : u256 (as 4 u64)
                    162-165 source_swap_id     : u32
                */
    
                let units_x64: [u64; 4] = [                                         // TODO create helper function
                    u64::from_be_bytes(message[97..105].try_into().unwrap()),
                    u64::from_be_bytes(message[105..113].try_into().unwrap()),
                    u64::from_be_bytes(message[113..121].try_into().unwrap()),
                    u64::from_be_bytes(message[121..129].try_into().unwrap())
                ];


                // Min output: for Solana, the maximum value allowed is u64
                let min_output: u64 = u64::from_be_bytes(message[130..138].try_into().unwrap());

                // Make sure that the rest of the 'min_output' value is zero
                if !message[138..162].iter().all(|slice| slice == &0) {
                    return Err(error!(ErrorCode::MinimumOutputExceedsMaxAllowed));
                }


                // ! VERY IMPORTANT verify provided output_asset_wallet matches the one specified in the message
                if Pubkey::new(&message[65..97]).ne(&ctx.remaining_accounts[1].key()) {
                    return Err(error!(ErrorCode::InvalidAssetWalletAccount));
                }


                ctx.accounts.invoke_in_swap(
                    message[129],
                    units_x64,
                    min_output,
                    context == CTX_ASSET_APPROX_SWAP,
                    ctx.remaining_accounts[0].to_account_info(),
                    ctx.remaining_accounts[1].to_account_info(),
                    ctx.remaining_accounts[2].to_account_info(),
                    ctx.remaining_accounts[3].to_account_info(),
                ).unwrap();
    
                Ok(())

            },

            CTX_LIQUIDITY_SWAP => {
                // Invoke in_liquidity_swap of the target pool
                /*
                    Message Format
                    0       context            : u8
                    1-32    source_pool        : Pubkey
                    33-64   target_pool        : Pubkey
                    65-96   destination        : Pubkey
                    97-128  units              : u256 (as 4 u64)
                */
    
                // CPI - Arguments 
                let liquidity_units_x64: [u64; 4] = [
                    u64::from_be_bytes(message[97..105].try_into().unwrap()),
                    u64::from_be_bytes(message[105..113].try_into().unwrap()),
                    u64::from_be_bytes(message[113..121].try_into().unwrap()),
                    u64::from_be_bytes(message[121..129].try_into().unwrap())
                ];

                // ! VERY IMPORTANT verify provided destination_pool_token_wallet matches the one specified in the message
                if Pubkey::new(&message[65..97]).ne(&ctx.remaining_accounts[1].key()) {
                    return Err(error!(ErrorCode::InvalidPoolTokenWalletAccount));
                }

                ctx.accounts.invoke_liquidity_in_swap(
                    liquidity_units_x64,
                    ctx.remaining_accounts[0].to_account_info(),
                    ctx.remaining_accounts[1].to_account_info(),
                    ctx.remaining_accounts[2].to_account_info()
                ).unwrap();

                Ok(())
            },
            _ => Err(error!(ErrorCode::InvalidContext))
        }

    }


    pub fn on_acknowledgement_packet<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, OnIBCResult<'info>>,
        message: Vec<u8>
    ) -> Result<()> {

        // ! TODO:
        // ! PolymeraseInterface authority verification in OnRecvPacket context
        // !    ==> Only allow the authorized PolymeraseInterface authority holder to execute this function

        let context: u8 = u8::from_be_bytes(message[..1].try_into().unwrap());
        let source_swap_id: u32 = u32::from_be_bytes(message[162..166].try_into().unwrap());

        //TODO move to context?
        // Check the provided SwapPool program is the expected one
        if !ctx.accounts.swap_pool_program.key.eq(&ctx.accounts.swap_pool.owner) {
            return Err(error!(ErrorCode::InvalidSwapPoolProgram));
        }

        match context {

            CTX_ASSET_SWAP | CTX_ASSET_APPROX_SWAP => {
                // Invoke in_swap of the target pool
                /*
                    Message Format
                    0       context            : u8
                    1-32    source_pool        : Pubkey
                    33-64   target_pool        : Pubkey
                    65-96   destination        : Pubkey
                    97-128  units              : u256 (as 4 u64)
                    129     target_asset_index : u8
                    130-161 min_output         : u256 (as 4 u64)
                    162-165 source_swap_id     : u32
                */

                ctx.accounts.invoke_out_swap_ack(
                    source_swap_id,
                    ctx.remaining_accounts[3].to_owned(),
                    ctx.remaining_accounts[4].to_owned(),
                ).unwrap();

                Ok(())
            },

            CTX_LIQUIDITY_SWAP => {
                // Invoke in_liquidity_swap of the target pool
                /*
                    Message Format
                    0       context            : u8
                    1-32    source_pool        : Pubkey
                    33-64   target_pool        : Pubkey
                    65-96   destination        : Pubkey
                    97-128  units              : u256 (as 4 u64)
                */

                Ok(())  //TODO do something?
            },
            _ => Err(error!(ErrorCode::InvalidContext))
        }

    }


    pub fn on_timeout_packet<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, OnIBCResult<'info>>,
        message: Vec<u8>
    ) -> Result<()> {

        // ! TODO:
        // ! PolymeraseInterface authority verification in OnRecvPacket context
        // !    ==> Only allow the authorized PolymeraseInterface authority holder to execute this function

        let context: u8 = u8::from_be_bytes(message[..1].try_into().unwrap());
        let source_swap_id: u32 = u32::from_be_bytes(message[162..166].try_into().unwrap());

        //TODO move to context?
        // Check the provided SwapPool program is the expected one
        if !ctx.accounts.swap_pool_program.key.eq(&ctx.accounts.swap_pool.owner) {
            return Err(error!(ErrorCode::InvalidSwapPoolProgram));
        }

        match context {

            CTX_ASSET_SWAP | CTX_ASSET_APPROX_SWAP => {
                // Invoke in_swap of the target pool
                /*
                    Message Format
                    0       context            : u8
                    1-32    source_pool        : Pubkey
                    33-64   target_pool        : Pubkey
                    65-96   destination        : Pubkey
                    97-128  units              : u256 (as 4 u64)
                    129     target_asset_index : u8
                    130-161 min_output         : u256 (as 4 u64)
                    162-165 source_swap_id     : u32
                */

                ctx.accounts.invoke_out_swap_timeout(
                    source_swap_id,
                    ctx.remaining_accounts[0].to_owned(),
                    ctx.remaining_accounts[1].to_owned(),
                    ctx.remaining_accounts[2].to_owned(),
                    ctx.remaining_accounts[3].to_owned(),
                    ctx.remaining_accounts[4].to_owned(),
                ).unwrap();

                Ok(())
            },

            CTX_LIQUIDITY_SWAP => {
                // Invoke in_liquidity_swap of the target pool
                /*
                    Message Format
                    0       context            : u8
                    1-32    source_pool        : Pubkey
                    33-64   target_pool        : Pubkey
                    65-96   destination        : Pubkey
                    97-128  units              : u256 (as 4 u64)
                */

                Ok(())  //TODO do something?
            },
            _ => Err(error!(ErrorCode::InvalidContext))
        }

    }

}


#[derive(Accounts)]
#[instruction(swap_pool_state: Pubkey)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub configurator: Signer<'info>,
    #[account(
        init,
        payer=configurator,
        space=CrossChainSwapInterfaceState::LEN,  // NOTE: LEN includes discriminator
        seeds=[&swap_pool_state.to_bytes()],
        bump
    )]
    pub interface_state_account: Account<'info, CrossChainSwapInterfaceState>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct SetConfigurator<'info> {
    #[account(mut, has_one = configurator @ ErrorCode::InvalidConfigurator)]
    pub interface_state_account: Account<'info, CrossChainSwapInterfaceState>,
    #[account(mut)]
    pub configurator: Signer<'info>
}


#[derive(Accounts)]
pub struct RegisterReceiver<'info> {
    #[account(mut)]
    pub configurator: Signer<'info>,
    #[account(
        mut,
        has_one    = configurator @ ErrorCode::InvalidConfigurator,
        constraint = interface_state_account.swap_pool.eq(&swap_pool.key()) @ ErrorCode::InvalidSwapPoolAccount
    )]
    pub interface_state_account: Account<'info, CrossChainSwapInterfaceState>,
    #[account()]
    /// CHECK: safe, as it must match that saved on interface_state_account
    pub swap_pool: UncheckedAccount<'info>,
    pub register_program: Program<'info, PolymeraseRegister>,
    #[account(mut)]
    /// CHECK: not reading from the account
    pub port_registration: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}


#[derive(Accounts)]
#[instruction(chain: u64, pool: Pubkey)]
pub struct CreateConnection<'info> {
    pub swap_pool_authority: Signer<'info>,
    #[account(mut)]
    pub rent_payer: Signer<'info>,
    #[account(has_one = swap_pool_authority @ ErrorCode::InvalidSwapPoolAuthority)]
    pub interface_state_account: Account<'info, CrossChainSwapInterfaceState>,
    #[account(
        init,
        payer = rent_payer,
        space = ConnectionState::LEN,  // NOTE: includes discriminator
        seeds = [
            &interface_state_account.key().to_bytes(),
            chain.to_le_bytes().as_ref(),
            &pool.to_bytes()
        ],
        bump
    )]
    pub connection_state_account: Account<'info, ConnectionState>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(chain: u64, approx: bool, source_pool: Pubkey, target_pool: Pubkey)]
pub struct CrossChainSwap<'info> {
    pub swap_pool_authority: Signer<'info>,
    #[account(
        has_one    = swap_pool_authority @ ErrorCode::InvalidSwapPoolAuthority,  // The invocation must come from the swap_pool_authority
        constraint = interface_state_account.swap_pool == source_pool @ ErrorCode::InvalidSwapPoolAccount      // The source_pool must match the one saved
        // has_one = polymerase_endpoint_program,  // The provided polymerase_endpoint_program must match the saved one // ! TODO verify polymerase sender
    )]
    pub interface_state_account: Account<'info, CrossChainSwapInterfaceState>,    //TODO! must verify that it has been created using swap_pool
    #[account(
        constraint = !(connection_state_account.connected_interface_program.eq(&Pubkey::default())) @ ErrorCode::PoolNotConnected,  // The target pool must be connected
        seeds = [
            &interface_state_account.key().to_bytes(),
            chain.to_le_bytes().as_ref(),
            &target_pool.to_bytes()
        ],
        bump = connection_state_account.connection_state_account_bump
    )]
    pub connection_state_account: Account<'info, ConnectionState>,

    // Polymerase endpoint accounts
    pub polymerase_sender_program: Program<'info, PolymeraseSender>,
    #[account(mut)]
    pub polymerase_ibc_data: Signer<'info>,
    #[account(mut)]
    pub polymerase_ibc_data_account_payer: Signer<'info>,    //TODO rename
    /// CHECK: Safe, as we are not reading from the account
    pub system_program: UncheckedAccount<'info> // Intentionally not Program<'info, System>, as we do not want to run any checks on the account (and hence minimize gas fees)
}

#[derive(Accounts)]
#[instruction(message: Vec<u8>)]
pub struct OnIBCInvocation<'info> {
    /// CHECK: unused
    pub port_registration: UncheckedAccount<'info>,
    #[account()]    // ! TODO add seeds => polymerase receiver set to pda with no seeds?
    pub polymerase_authority: Signer<'info>,
    #[account(
        constraint = interface_state_account.key().eq(&Pubkey::new(&message[33..65])) @ ErrorCode::InvalidInterfaceAccount, // ! The interface_state_account, derived from the provided swap_pool account, must match the target cross chain pool id present in the payload (i.e. verify the provided swap pool) 
        constraint = interface_state_account.swap_pool == swap_pool.key() @ ErrorCode::InvalidSwapPoolAccount               // ! The swap_pool must match the one saved
    )]
    pub interface_state_account: Box<Account<'info, CrossChainSwapInterfaceState>>,    //TODO! must verify that it has been created using swap_pool
    #[account(mut)]
    /// CHECK: //TODO
    pub swap_pool: AccountInfo<'info>,
    /// CHECK: //TODO
    #[account(
        constraint = swap_pool_program.key().eq(swap_pool.owner) @ ErrorCode::InvalidSwapPoolProgram    // ! Make sure the provided swap pool program matches the expected one
    )]
    pub swap_pool_program: AccountInfo<'info>,
    /// CHECK: Safe, as we are not reading from the account.
    #[account()]    //TODO account attribute required?
    pub token_program: UncheckedAccount<'info>,

    // Changing accounts
    #[account(
        constraint=!connection_state_account.connected_interface_program.eq(&Pubkey::default()), //TODO delete account to remove connection ? => avoid check
        // seeds = [        // ! TODO - Disabled until source chain and sender is known
        //     &interface_state_account.key().to_bytes(),
        //     &header[0..8],  // ! The source chain
        //     &header[8..40]  // ! The source pool id (transaction signer)
        // ],
        // bump
    )]
    pub connection_state_account: Account<'info, ConnectionState>
}

impl<'info> OnIBCInvocation<'info> {

    pub fn invoke_in_swap(
        &self,
        output_asset_index: u8,
        units_x64: [u64; 4],
        min_output: u64,
        approx: bool,
        output_asset_mint: AccountInfo<'info>,
        output_asset_wallet: AccountInfo<'info>,
        swap_pool_asset_wallet: AccountInfo<'info>,
        swap_authority: AccountInfo<'info>
    ) -> ProgramResult {

        // Build CPI data (sighash + arguments)
        let mut data_vec = SIGHASH_IN_SWAP.try_to_vec().unwrap();
        data_vec.append(&mut InSwapArgs {
            output_asset_index,
            units_x64,
            min_output,
            approx
        }.try_to_vec().unwrap()); //TODO beter way to do this?

        // Build instruction
        let instruction = Instruction {
            data: data_vec,
            program_id: self.swap_pool_program.key(),
            accounts: vec![
                AccountMeta {
                    pubkey: self.swap_pool.key(),
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: output_asset_mint.key(),        // Output asset mint
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: output_asset_wallet.key(),      // Output asset wallet // ! VERY IMPORTANT use the key as provided in the message, not the account provided in the context
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: swap_pool_asset_wallet.key(),   // Swap pool output asset wallet
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: swap_authority.key(),           // Swap pool authority
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: self.interface_state_account.key(),
                    is_signer: true,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: self.token_program.key(),
                    is_signer: false,
                    is_writable: false,
                }
            ]
        };

        invoke_signed(
            &instruction,
            &[
                self.swap_pool.to_account_info(),
                output_asset_mint,
                output_asset_wallet,
                swap_pool_asset_wallet,
                swap_authority,
                self.interface_state_account.to_account_info(),
                self.token_program.to_account_info()
            ],
            &[&[
                &self.swap_pool.key().to_bytes(),
                &[self.interface_state_account.interface_state_account_bump]
            ]]
        )
    }


    pub fn invoke_liquidity_in_swap(
        &self,
        liquidity_units_x64: [u64; 4],
        swap_pool_token_mint: AccountInfo<'info>,
        destination_pool_token_wallet: AccountInfo<'info>,
        swap_authority: AccountInfo<'info>
    ) -> ProgramResult {

        // Build CPI data (sighash + arguments)
        let mut data_vec = SIGHASH_IN_LIQUIDITY_SWAP.try_to_vec().unwrap();
        data_vec.append(&mut InLiquiditySwapArgs {
            liquidity_units_x64
        }.try_to_vec().unwrap()); //TODO beter way to do this?

        // Build instruction
        let instruction = Instruction {
            data: data_vec,
            program_id: self.swap_pool_program.key(),
            accounts: vec![
                AccountMeta {
                    pubkey: self.swap_pool.key(),
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: swap_pool_token_mint.key(),             // Swap pool token mint
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: destination_pool_token_wallet.key(),    // Destination pool token wallet
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: swap_authority.key(),                   // Swap pool authority
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: self.interface_state_account.key(),
                    is_signer: true,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: self.token_program.key(),
                    is_signer: false,
                    is_writable: false,
                }
            ]
        };

        invoke_signed(
            &instruction,
            &[
                self.swap_pool.to_account_info(),
                swap_pool_token_mint,
                destination_pool_token_wallet,
                swap_authority,
                self.interface_state_account.to_account_info(),
                self.token_program.to_account_info()
            ],
            &[&[
                &self.swap_pool.key().to_bytes(),
                &[self.interface_state_account.interface_state_account_bump]
            ]]
        )
    }

}


#[derive(Accounts)]
#[instruction(message: Vec<u8>)]
pub struct OnIBCResult<'info> {
    /// CHECK: unused
    pub port_registration: UncheckedAccount<'info>,
    // #[account()]    // ! TODO add seeds => polymerase receiver set to pda with no seeds?
    // pub polymerase_authority: Signer<'info>, // TODO disabled for now (until Polymer contract is implemented)

    #[account(
        constraint = interface_state_account.key().eq(&Pubkey::new(&message[1..33])) @ ErrorCode::InvalidInterfaceAccount, // ! The interface_state_account, derived from the provided swap_pool account, must match the source cross chain pool id present in the payload (i.e. verify the provided swap pool) 
        constraint = interface_state_account.swap_pool == swap_pool.key() @ ErrorCode::InvalidSwapPoolAccount              // ! The swap_pool must match the one saved
    )]
    pub interface_state_account: Box<Account<'info, CrossChainSwapInterfaceState>>,    //TODO! must verify that it has been created using swap_pool
    
    #[account(mut)]
    /// CHECK: //TODO
    pub swap_pool: AccountInfo<'info>,
    /// CHECK: //TODO
    #[account(
        constraint = swap_pool_program.key().eq(swap_pool.owner) @ ErrorCode::InvalidSwapPoolProgram                       // ! Make sure the provided swap pool program matches the expected one
    )]
    pub swap_pool_program: AccountInfo<'info>,

    /// CHECK: Safe, as we are passing it directly to the SwapPool program
    pub swap_pool_authority: UncheckedAccount<'info>,

    /// CHECK: Safe, as we are passing it directly to the SwapPool program
    pub token_program: UncheckedAccount<'info>,

    /// CHECK: safe, as we are passing it directly to the SwapPool program
    pub system_program: UncheckedAccount<'info>,

    // Remaining accounts used for dynamic accounts
    // 0 -     - asset_mint
    // 1 - mut - fallback_wallet
    // 2 - mut - swap_pool_asset_wallet
    // 3 - mut - swap_escrow
    // 4 - mut - swap_escrow_rent_payer
}


impl<'info> OnIBCResult<'info> {

    pub fn invoke_out_swap_ack(
        &self,
        source_swap_id: u32,
        swap_escrow: AccountInfo<'info>,
        swap_escrow_rent_payer: AccountInfo<'info>
    ) -> ProgramResult {

        // Build CPI data (sighash + arguments)
        let mut data_vec = SIGHASH_OUT_SWAP_ACK.try_to_vec().unwrap();
        data_vec.append(&mut OutSwapAckArgs {escrow_nonce: source_swap_id}.try_to_vec().unwrap()); //TODO beter way to do this?

        // Build instruction
        let instruction = Instruction {
            data: data_vec,
            program_id: self.swap_pool_program.key(),
            accounts: vec![
                AccountMeta {
                    pubkey: self.swap_pool.key(),
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: swap_escrow.key(),
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: swap_escrow_rent_payer.key(),
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: self.interface_state_account.key(),
                    is_signer: true,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: self.system_program.key(),
                    is_signer: false,
                    is_writable: false,
                }
            ]
        };

        invoke_signed(
            &instruction,
            &[
                self.swap_pool.to_account_info(),
                swap_escrow,
                swap_escrow_rent_payer,
                self.interface_state_account.to_account_info(),
                self.system_program.to_account_info()
            ],
            &[&[
                &self.swap_pool.key().to_bytes(),
                &[self.interface_state_account.interface_state_account_bump]
            ]]
        )

    }


    pub fn invoke_out_swap_timeout(
        &self,
        source_swap_id: u32,
        asset_mint: AccountInfo<'info>,
        fallback_wallet: AccountInfo<'info>,
        swap_pool_asset_wallet: AccountInfo<'info>,
        swap_escrow: AccountInfo<'info>,
        swap_escrow_rent_payer: AccountInfo<'info>
    ) -> ProgramResult {

        // Build CPI data (sighash + arguments)
        let mut data_vec = SIGHASH_OUT_SWAP_TIMEOUT.try_to_vec().unwrap();
        data_vec.append(&mut OutSwapTimeoutArgs {
            escrow_nonce: source_swap_id
        }.try_to_vec().unwrap()); //TODO beter way to do this?

        // Build instruction
        let instruction = Instruction {
            data: data_vec,
            program_id: self.swap_pool_program.key(),
            accounts: vec![
                AccountMeta {
                    pubkey: self.swap_pool.key(),
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: asset_mint.key(),
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: fallback_wallet.key(),
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: swap_pool_asset_wallet.key(),
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: self.swap_pool_authority.key(),
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: self.token_program.key(),
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: swap_escrow.key(),
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: swap_escrow_rent_payer.key(),
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: self.interface_state_account.key(),
                    is_signer: true,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: self.system_program.key(),
                    is_signer: false,
                    is_writable: false,
                }
            ]
        };

        invoke_signed(
            &instruction,
            &[
                self.swap_pool.to_account_info(),
                asset_mint,
                fallback_wallet,
                swap_pool_asset_wallet,
                self.swap_pool_authority.to_account_info(),
                self.token_program.to_account_info(),
                swap_escrow,
                swap_escrow_rent_payer,
                self.interface_state_account.to_account_info(),
                self.system_program.to_account_info(),
            ],
            &[&[
                &self.swap_pool.key().to_bytes(),
                &[self.interface_state_account.interface_state_account_bump]
            ]]
        )

    }
}



// Accounts *********************************************************************************************************************

#[account]
pub struct CrossChainSwapInterfaceState {
    pub configurator: Pubkey,
    pub swap_pool: Pubkey,
    pub swap_pool_authority: Pubkey,
    pub polymerase_endpoint_program: Pubkey,    //TODO Hardcode?
    pub interface_state_account_bump: u8
}

impl CrossChainSwapInterfaceState {
    pub const LEN: usize = 8 + 32*4 + 1;  // NOTE: includes discriminator
}


#[account]
pub struct ConnectionState {
    pub connected_interface_program: Pubkey,
    pub connection_state_account_bump: u8
}

impl ConnectionState {
    pub const LEN: usize = 8 + 32*1 + 1;  // NOTE: includes discriminator
}



// CPI Argument Structs *********************************************************************************************************

#[derive(PartialEq, Debug, Clone, BorshSerialize)]
pub struct CallMultichainArgs {
    target_chain: u64,
    target_program: Pubkey,
    payload: Vec<u8>
}

#[derive(PartialEq, Debug, Clone, BorshSerialize)]
pub struct OutSwapAckArgs {
    escrow_nonce: u32
}

#[derive(PartialEq, Debug, Clone, BorshSerialize)]
pub struct OutSwapTimeoutArgs {
    escrow_nonce: u32
}

#[derive(PartialEq, Debug, Clone, BorshSerialize)]
pub struct InSwapArgs {
    output_asset_index: u8,
    units_x64: [u64; 4],
    min_output: u64,
    approx: bool
}

#[derive(PartialEq, Debug, Clone, BorshSerialize)]
pub struct InLiquiditySwapArgs {
    liquidity_units_x64: [u64; 4]
}



// Events ***********************************************************************************************************************

//TODO


// Errors ***********************************************************************************************************************

#[error_code]
pub enum ErrorCode {
    #[msg("The provided config authority does not match the expected one.")]
    InvalidConfigurator,
    #[msg("The provided swap pool authority does not match the expected one.")]
    InvalidSwapPoolAuthority,
    #[msg("Pool not connected.")]
    PoolNotConnected,
    #[msg("The source pool is not connected.")]
    SourcePoolNotConnected,
    #[msg("Invalid received swap context index.")]
    InvalidContext,
    #[msg("The provided swap pool account does not match the one linked to this interface.")]
    InvalidSwapPoolAccount,
    #[msg("The provided swap pool program does not match the one linked to this interface.")]
    InvalidSwapPoolProgram,
    #[msg("The provided interface account does not match the expected one.")]
    InvalidInterfaceAccount,
    #[msg("The provided output asset wallet does not match with the one specified in the request payload.")]
    InvalidAssetWalletAccount,
    #[msg("The provided output pool token wallet does not match with the one specified in the request payload.")]
    InvalidPoolTokenWalletAccount,
    #[msg("The specified minimum output exceeds the allowed one by the chain implementation.")]
    MinimumOutputExceedsMaxAllowed,
}