use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke};
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
use polymerase_register::{PortRegistration, ID as POLYMERASE_REGISTER_ID};

declare_id!("6NUbfXpGgo9dUKiEv8NQsGJoHxteMBVoAXLtnRHbJH7J");

#[program]
pub mod polymerase_receiver {

    use super::*;

    // receive an ibc packet and forward to dapp
    pub fn on_receive<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, OnReceive<'info>>,
        port_account_id: Pubkey,
        _src_port: String,
        _src_channel: String,
        _dest_port: String,
        _dest_channel: String,
        _sequence: u64,
        data: Vec<u8>,
        _timeout_block: u64,
        _timeout_timestamp: u64
    ) -> Result<()> {

        // Full CPI data (sighash + arguments)
        let mut data_vec = ctx.accounts.port_registration.on_receive_sighash.try_to_vec().unwrap();
        data_vec.append(&mut data.try_to_vec().unwrap());

        let mut meta_accounts = vec![
            AccountMeta::new_readonly(ctx.accounts.port_registration.key(), false),
            AccountMeta::new_readonly(ctx.accounts.authority.key(), true)
        ];

        // add remaining accounts
        meta_accounts.extend(ctx.remaining_accounts.iter().map(|a| AccountMeta {
            pubkey: a.key(),
            is_signer: a.is_signer,
            is_writable: a.is_writable,
        }));

        let instruction = Instruction {
            data: data_vec,
            program_id: ctx.accounts.dapp_program.key(),
            accounts: meta_accounts,
        };

        let mut accounts: Vec<AccountInfo> = vec![
            ctx.accounts.port_registration.to_account_info(),
            ctx.accounts.authority.to_account_info(),           // TODO Make the receiver sign the invocation instead?
            ctx.accounts.dapp_program.to_account_info(),
        ];
        accounts.extend_from_slice(ctx.remaining_accounts);

        invoke(&instruction, &accounts).unwrap();

        Ok(())
    }
}



// Contexts ****************************************************************************************

#[derive(Accounts)]
#[instruction(port_account_id: Pubkey)]
pub struct OnReceive<'info> {
    // registration account (to read program id and sighash)
    #[account(
        seeds = [
            port_account_id.as_ref()
        ],
        bump,
        seeds::program = POLYMERASE_REGISTER_ID,
        has_one = dapp_program
    )]
    pub port_registration: Account<'info, PortRegistration>,
    /// CHECK: Crosschecked with the one saved in port_registration
    pub dapp_program: UncheckedAccount<'info>,
    #[account()]
    pub authority: Signer<'info>    // ! TODO verification required
}



// Accounts ****************************************************************************************

#[derive(PartialEq, Debug, Clone, AnchorSerialize)]
pub struct CallOnReceiveArgs {
    data: Vec<u8>,
}
