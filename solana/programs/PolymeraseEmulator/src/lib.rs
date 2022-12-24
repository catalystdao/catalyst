use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke_signed, hash::hash};
use borsh::BorshSerialize;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcPLYMERASEEMU");

#[program]
pub mod polymerase_emulator {
    use super::*;


    pub fn initialize(
        _ctx: Context<Initialize>
    ) -> Result<()> {
        Ok(())
    }


    pub fn call_multichain(
        ctx: Context<CallMultichain>,
        target_chain: u64,
        target_program: Pubkey,
        payload: Vec<u8>
    ) -> Result<()> {

        // Save the instruction parameters
        ctx.accounts.instruction_account.source_chain = 99u64;  //TODO! Solana chain hardcoded to 99

        if target_chain != 99u64 {
            return Err(error!(ErrorCode::InvalidTargetChain));
        }

        ctx.accounts.instruction_account.target_chain = target_chain.clone();
        ctx.accounts.instruction_account.target_program = target_program.clone();
        ctx.accounts.instruction_account.rent_payer = ctx.accounts.payer.key();
        ctx.accounts.instruction_account.signer = ctx.accounts.signer.key();
        ctx.accounts.instruction_account.payload = payload.clone();

        // Increment the next instruction index
        ctx.accounts.emulator_state_account.next_index.checked_add(1).unwrap();

        emit!(CrossChainTxEvent{
            target_chain,
            target_program,
            sender: ctx.accounts.signer.key(),
            payload
        });

        Ok(())
    }


    pub fn execute<'a>(
        ctx: Context<'_, '_, '_, 'a, Execute<'a>>,
        _index: u64                                // Used in the instruction's context
    ) -> Result<()> {

        // Build the instruction

        // Instruction data - sighash, i.e. target the 'receive' instruction
        let mut sighash = [0u8; 8];
        sighash.copy_from_slice(&hash("global:receive".as_bytes()).to_bytes()[..8]); //TODO! precompute? Optimized at compile time?

        // Instruction data - arguments
        let mut header_vec = ctx.accounts.instruction_account.target_chain.try_to_vec().unwrap();
        header_vec.append(&mut ctx.accounts.instruction_account.signer.try_to_vec().unwrap()); //TODO beter way to do this?
        
        let instruction_arguments: ReceiveArgs = ReceiveArgs {
            header: header_vec,
            data: ctx.accounts.instruction_account.payload.clone()
        };

        // Full instruction data (sighash + arguments)
        let mut data_vec = sighash.try_to_vec().unwrap();
        data_vec.append(&mut instruction_arguments.try_to_vec().unwrap()); //TODO beter way to do this?

        // CPI Account Infos: polymerase_authority + remaining_accounts
        let mut account_infos: Vec<AccountInfo<'a>> = vec![ctx.accounts.polymerase_authority.to_account_info()];
        account_infos.extend(ctx.remaining_accounts.iter().map(|a| -> AccountInfo<'a> {a.to_owned()}));

        // Instruction AccountMetas
        let mut account_metas: Vec<AccountMeta> = account_infos.iter().map(|acc| {AccountMeta {
            pubkey: acc.key.clone(),
            is_signer: acc.is_signer,
            is_writable: acc.is_writable
        }}).collect();
        account_metas[0].is_signer = true;  // Set polymerase_authority (first account) as signer


        // Log event
        emit!(CrossChainRxEvent{
            source_chain: ctx.accounts.instruction_account.source_chain,
            target_program: ctx.accounts.instruction_account.target_program.clone(),
            sender: ctx.accounts.instruction_account.signer.clone(),
            payload: ctx.accounts.instruction_account.payload.clone()
        });

        // CPI
        invoke_signed(
            &Instruction {
                data: data_vec,
                program_id: ctx.accounts.instruction_account.target_program,
                accounts: account_metas
            },
            account_infos.as_slice(),
            &[&[
                &ctx.accounts.emulator_state_account.key().to_bytes(),
                &[ctx.bumps.get("polymerase_authority").unwrap().to_owned()]
            ]]
        ).unwrap();

        Ok(())
    }

}



// Contexts *********************************************************************************************************************

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(init, payer=payer, space=8+PolymeraseEmulatorState::LEN)]
    pub emulator_state_account: Account<'info, PolymeraseEmulatorState>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(target_chain: u64, target_program: Pubkey, payload: Vec<u8>)]
pub struct CallMultichain<'info> {
    #[account(mut)]
    pub emulator_state_account: Account<'info, PolymeraseEmulatorState>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account()]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer=payer,
        space=8 + PolymeraseInstruction::FIXED_LEN + payload.len(),
        seeds = [
            &emulator_state_account.key().to_bytes(),
            emulator_state_account.next_index.to_be_bytes().as_ref()
        ],
        bump
    )]
    pub instruction_account: Account<'info, PolymeraseInstruction>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(_index: u64)]
pub struct Execute<'info> {
    #[account()]
    pub emulator_state_account: Account<'info, PolymeraseEmulatorState>,
    #[account(
        mut,
        close = rent_receiver,                                                                                  // Close the instruction_account upon completion of the instruction. Send account lamports to rent_receiver
        constraint = instruction_account.rent_payer == rent_receiver.key() @ ErrorCode::InvalidRentReceiver,    // The provided rent_reciver must be equal to the original rent payer (that it is stored in the instruction_account)
        has_one = target_program @ ErrorCode::InvalidTargetProgram,                                             // The provided target_program must be equal to the saved one in instruction_account
        seeds = [
            &emulator_state_account.key().to_bytes(),
            _index.to_be_bytes().as_ref()
        ],
        bump
    )]
    pub instruction_account: Account<'info, PolymeraseInstruction>,
    /// CHECK: Safe, as we are not reading from the account
    #[account(mut)]
    pub rent_receiver: AccountInfo<'info>,
    /// CHECK: Safe, as we are not reading from the account.
    #[account(
        seeds = [
            &emulator_state_account.key().to_bytes()
        ],
        bump
    )]
    pub polymerase_authority: UncheckedAccount<'info>,
    /// CHECK: Safe, as we are not reading from the account
    target_program: AccountInfo<'info>,
}



// Accounts *********************************************************************************************************************

#[account]
pub struct PolymeraseEmulatorState {
    pub next_index: u64
}

impl PolymeraseEmulatorState {
    pub const LEN: usize = 8;
}


#[account]
pub struct PolymeraseInstruction {
    source_chain: u64,
    target_chain: u64,
    target_program: Pubkey,
    signer: Pubkey,
    rent_payer: Pubkey,
    payload: Vec<u8>
}

impl PolymeraseInstruction {
    pub const FIXED_LEN: usize = 
        8*8       + // source_chain
        8*8       + // target_chain
        8*32      + // target_program
        8*32      + // signer
        8*32        // rent_payer
    ;
}



// CPI Argument Structs *********************************************************************************************************

#[derive(PartialEq, Debug, Clone, BorshSerialize)]
pub struct ReceiveArgs {
    pub header: Vec<u8>,
    pub data: Vec<u8>
}



// Events ***********************************************************************************************************************

#[event]
pub struct CrossChainTxEvent {
    target_chain: u64,
    target_program: Pubkey,
    sender: Pubkey,
    payload: Vec<u8>
}

#[event]
pub struct CrossChainRxEvent {
    source_chain: u64,
    target_program: Pubkey,
    sender: Pubkey,
    payload: Vec<u8>
}



// Errors ***********************************************************************************************************************

#[error_code]
pub enum ErrorCode {
    #[msg("The provided target chain id is not supported.")]
    InvalidTargetChain,
    #[msg("The provided target program account does not match with the target program id.")]
    InvalidTargetProgram,
    #[msg("The provided rent receiver does not match with the original rent payer.")]
    InvalidRentReceiver,
}