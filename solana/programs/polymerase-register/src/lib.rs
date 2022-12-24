use anchor_lang::prelude::*;
use anchor_lang::solana_program::{hash::hash};

declare_id!("BYbaE3pipg9w4kAo7NVrN9TrNm4nnnHpKEVNvh4mvgvi");

#[program]
pub mod polymerase_register {

    use super::*;

    // Create account to hold port registration data
    pub fn register_port(
        ctx: Context<RegisterPort>,
        program: Pubkey,
        data: Vec<u8>,
        receive_callback: String,
        accounts_pubkey: Vec<Pubkey>, // TODO use accountMetas? (research struct as argument)
        accounts_signer: Vec<bool>,
        accounts_writable: Vec<bool>
    ) -> Result<()> {

        if data.len() > MAX_PAYLOAD_LENGTH {
            return Err(Errors::DataTooLong.into());
        }

        let port_registration: &mut Account<PortRegistration> = &mut ctx.accounts.port_registration;

        port_registration.payer             = ctx.accounts.payer.key();
        port_registration.data              = data;
        port_registration.dapp_program      = program;
        port_registration.bump              = *ctx.bumps.get("port_registration").unwrap();

        port_registration.accounts_pubkey   = accounts_pubkey.clone();      //TODO clone required?
        port_registration.accounts_signer   = accounts_signer.clone();
        port_registration.accounts_writable = accounts_writable.clone();
        
        port_registration.on_receive_sighash.copy_from_slice(
            &hash(
                format!("global:{}", receive_callback).as_bytes(),
            )
            .to_bytes()[..8],
        );

        Ok(())

    }

    // TODO: make sure only only the relayer can call this!!!
    // Close Ibc data account. 
    pub fn close_account(
        ctx: Context<CloseAccount>,
        registrant: Pubkey
    ) -> Result<()> {
        Ok(())
    }

}



// Contexts ****************************************************************************************

#[derive(Accounts)]
pub struct RegisterPort<'info> {
    #[account(init,
        payer = payer,
        space = PortRegistration::LEN,
        seeds = [
            registrant.key().as_ref()
        ],
        bump)]
    pub port_registration: Account<'info, PortRegistration>,
    #[account(mut)]
    pub registrant: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}


// TODO: test that only the relayer can call this!!!
#[derive(Accounts)]
#[instruction(registrant: Pubkey)]
pub struct CloseAccount<'info> {
    #[account(
        mut,
        close = payer,
        has_one = payer,
        seeds = [
            registrant.key().as_ref()
        ],
        bump
    )]
    pub port_registration: Account<'info, PortRegistration>,
    #[account(mut)]
    /// CHECK: account to credit, must match the one saved in port_registration
    pub payer: AccountInfo<'info>,
    #[account(mut)]
    pub authority: Signer<'info>, // relayer
    pub system_program: Program<'info, System>,
}



// Accounts ****************************************************************************************

const DISCRIMINATOR_LENGTH   : usize = 8;
const PUBLIC_KEY_LENGTH      : usize = 32;
const BOOL_LENGTH            : usize = 1;
const VECTOR_LENGTH_PREFIX   : usize = 4;   // stores the size of the vector.
const MAX_PAYLOAD_LENGTH     : usize = 512; // max length
const RECEIVE_SIGHASH_LENGTH : usize = 8;   // length of the receive sighash length

#[account]
pub struct PortRegistration {
    pub payer: Pubkey,               // the user who create the account
    pub data: Vec<u8>,               // extra init data
    pub on_receive_sighash: [u8; 8], // on_receive callback name
    pub dapp_program: Pubkey,
    pub bump: u8,
    pub accounts_pubkey: Vec<Pubkey>, // accounts (in-order) required for on_receive_args
    pub accounts_signer: Vec<bool>,
    pub accounts_writable: Vec<bool>,
}

impl PortRegistration {
    pub const LEN: usize = DISCRIMINATOR_LENGTH
        + PUBLIC_KEY_LENGTH        // payer
        + VECTOR_LENGTH_PREFIX     // data
        + MAX_PAYLOAD_LENGTH
        + RECEIVE_SIGHASH_LENGTH   // on_receive_sighash
        + PUBLIC_KEY_LENGTH        // dapp_program
        + BOOL_LENGTH              // bump
        + VECTOR_LENGTH_PREFIX     // accounts_pubkey
        + PUBLIC_KEY_LENGTH * 12
        + VECTOR_LENGTH_PREFIX     // accounts_signer
        + BOOL_LENGTH * 12
        + VECTOR_LENGTH_PREFIX     // accounts_writable
        + BOOL_LENGTH * 12;
}



// Events *****************************************************************************************

#[event]
pub struct PortRegistrationEvent {
    pub on_receive_callback: String,
}



// Errors *****************************************************************************************

#[error_code]
pub enum Errors {
    #[msg("The provided data should be 512 chars long maximum")]
    DataTooLong,
}
