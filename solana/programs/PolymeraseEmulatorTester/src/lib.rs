use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke, hash::hash};
use borsh::BorshSerialize;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsPET");

#[program]
pub mod polymerase_emulator_tester {
    use std::convert::TryInto;

    use super::*;

    pub fn create_polymerase_connection(
        ctx: Context<CreatePolymeraseConnection>,
        polymerase_endpoint_program: Pubkey,
        polymerase_endpoint_state: Pubkey
    ) -> Result<()> {
        ctx.accounts.polymerase_connection_account.polymerase_endpoint_program = polymerase_endpoint_program;
        ctx.accounts.polymerase_connection_account.polymerase_endpoint_state = polymerase_endpoint_state;

        Ok(())
    }

    pub fn send_data(
        ctx: Context<SendData>,
        data: u64,
        target_chain: u64,
        target_program: Pubkey
    ) -> Result<()> {

        // Build the CPI instruction

        // CPI - sighash: target the 'recieve' instruction
        let mut sighash = [0u8; 8];
        sighash.copy_from_slice(&hash("global:call_multichain".as_bytes()).to_bytes()[..8]); //TODO! precompute? Optimized at compile time?

        // CPI - Arguments        
        let instruction_arguments: CallMultichainArgs = CallMultichainArgs {
            target_chain,
            target_program,
            payload: data.try_to_vec().unwrap()
        };

        // Full CPI data (sighash + arguments)
        let mut data_vec = sighash.try_to_vec().unwrap();
        data_vec.append(&mut instruction_arguments.try_to_vec().unwrap()); //TODO beter way to do this?

        let instruction = Instruction {
            data: data_vec,
            program_id: ctx.accounts.polymerase_endpoint_program.key(),
            accounts: vec![
                AccountMeta {
                    pubkey: ctx.accounts.polymerase_endpoint_state.key(),
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: ctx.accounts.polymerase_instruction_account_payer.key(),
                    is_signer: true,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: ctx.accounts.polymerase_instruction_signer.key(),
                    is_signer: true,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: ctx.accounts.polymerase_instruction_account.key(),
                    is_signer: false,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: ctx.accounts.system_program.key(),
                    is_signer: false,
                    is_writable: false,
                }
            ]
        };

        invoke(
            &instruction,
            &[
                ctx.accounts.polymerase_endpoint_state.to_account_info(),
                ctx.accounts.polymerase_instruction_account_payer.to_account_info(),
                ctx.accounts.polymerase_instruction_signer.to_account_info(),
                ctx.accounts.polymerase_instruction_account.to_account_info(),
                ctx.accounts.polymerase_endpoint_program.to_account_info(),
                ctx.accounts.system_program.to_account_info()
            ]
        ).unwrap();

        Ok(())
    }

    pub fn receive(
        ctx: Context<Receive>,
        header: Vec<u8>,
        message: Vec<u8>
    ) -> Result<()> {
        let data = u64::from_le_bytes(message[0..8].try_into().unwrap());

        emit!(MessageReceived {
            data
        });

        Ok(())
    }
}



// Contexts *********************************************************************************************************************

#[derive(Accounts)]
pub struct CreatePolymeraseConnection<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(init, payer=payer, space=8 + PolymeraseConnection::LEN)]
    pub polymerase_connection_account: Account<'info, PolymeraseConnection>,
    pub system_program: Program<'info, System>
}


#[derive(Accounts)]
pub struct SendData<'info> {
    #[account(
        has_one = polymerase_endpoint_program,
        has_one = polymerase_endpoint_state
    )] // Make sure the provided polymerase_endpoint matches with the one saved on polymerase_connection_account
    pub polymerase_connection_account: Account<'info, PolymeraseConnection>,

    // Polymerase endpoint accounts
    /// CHECK: Safe, as we are not reading from the account
    #[account(mut)]
    pub polymerase_endpoint_state: UncheckedAccount<'info>,
    #[account(mut)]
    pub polymerase_instruction_account_payer: Signer<'info>,
    /// CHECK: Safe, as we are not reading from the account
    #[account(mut)]
    pub polymerase_instruction_account: UncheckedAccount<'info>,
    /// CHECK: Safe, as we are not reading from the account
    #[account()]
    pub polymerase_endpoint_program: UncheckedAccount<'info>,
    #[account(mut)]
    pub polymerase_instruction_signer: Signer<'info>,
    /// CHECK: Safe, as we are not reading from the account
    #[account()]
    pub system_program: UncheckedAccount<'info> // Intentionally not Program<'info, System>, as we do not want to run any checks on the account (and hence minimize gas fees)
}


#[derive(Accounts)]
pub struct Receive<'info> {
    // Make sure the authority that signs the transaction is PDA derived from the endpoint state account
    #[account(
        seeds=[&polymerase_connection_account.polymerase_endpoint_state.to_bytes()],
        seeds::program=polymerase_connection_account.polymerase_endpoint_program,
        bump
    )]
    pub polymerase_authority: Signer<'info>,

    #[account()]
    pub polymerase_connection_account: Account<'info, PolymeraseConnection>,
}



// Accounts *********************************************************************************************************************

#[account]
pub struct PolymeraseConnection {
    pub polymerase_endpoint_program: Pubkey,
    pub polymerase_endpoint_state: Pubkey
}

impl PolymeraseConnection {
    const LEN: usize = 32 + 32;
}



// Events ***********************************************************************************************************************

#[event]
pub struct MessageSent {

}

#[event]
pub struct MessageReceived {
    data: u64
}



// CPI Argument Structs *********************************************************************************************************

#[derive(PartialEq, Debug, Clone, BorshSerialize)]
pub struct CallMultichainArgs {
    target_chain: u64,
    target_program: Pubkey,
    payload: Vec<u8>
}
