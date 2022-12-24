use anchor_lang::{prelude::*, solana_program::{self, program::invoke_signed}};
use anchor_spl::token::{TokenAccount, InitializeAccount, initialize_account};


pub fn transfer_tokens_using_pda_authority<'info>(
    amount              : u64,
    source_wallet       : AccountInfo<'info>,
    destination_wallet  : AccountInfo<'info>,
    pda_authority       : AccountInfo<'info>,
    pda_authority_seeds : &[&[u8]],
    token_program       : AccountInfo<'info>    // TODO Is this necessary?
) {
    anchor_lang::solana_program::program::invoke_signed(
        // Transfer instruction
        &spl_token::instruction::transfer(  //TODO use transfer_checked?
            &token_program.key(),
            &source_wallet.key(),
            &destination_wallet.key(),
            &pda_authority.key(),
            &[],
            amount
        ).unwrap(),

        // Accounts
        &[
            source_wallet,
            destination_wallet,
            pda_authority
        ],

        // pda_authority seeds
        &[pda_authority_seeds]
    ).unwrap();
}


pub fn mint_tokens_using_pda_authority<'info> (
    amount              : u64,
    destination_wallet  : AccountInfo<'info>,
    mint                : AccountInfo<'info>,
    pda_authority       : AccountInfo<'info>,
    pda_authority_seeds : &[&[u8]],
    token_program       : AccountInfo<'info>    // TODO Is this necessary?
) {
    anchor_lang::solana_program::program::invoke_signed(
        //Transfer instruction
        &spl_token::instruction::mint_to(  //TODO use mint_to_checked??
            &token_program.key(),
            &mint.key(),
            &destination_wallet.key(),
            &pda_authority.key(),
            &[],
            amount
        ).unwrap(),

        // Accounts
        &[
            mint,
            destination_wallet,
            pda_authority
        ],

        // pda_authority seeds
        &[pda_authority_seeds]
    ).unwrap();
    
}


pub fn burn_tokens<'info> (
    amount        : u64,
    target_wallet : AccountInfo<'info>,
    mint          : AccountInfo<'info>,
    authority     : AccountInfo<'info>,
    token_program : AccountInfo<'info>    // TODO Is this necessary?
) {
    anchor_lang::solana_program::program::invoke(
        //Transfer instruction
        &spl_token::instruction::burn(  //TODO use burn_checked??
            &token_program.key(),
            &target_wallet.key(),
            &mint.key(),
            &authority.key(),
            &[],
            amount
        ).unwrap(),

        // Accounts
        &[
            target_wallet,
            mint,
            authority,
        ],
    ).unwrap();
}


pub fn burn_tokens_using_pda_authority<'info> (
    amount              : u64,
    target_wallet       : AccountInfo<'info>,
    mint                : AccountInfo<'info>,
    pda_authority       : AccountInfo<'info>,
    pda_authority_seeds : &[&[u8]],
    token_program       : AccountInfo<'info>,
) {
    anchor_lang::solana_program::program::invoke_signed(
        //Transfer instruction
        &spl_token::instruction::burn(  //TODO use burn_checked??
            &token_program.key(),
            &target_wallet.key(),
            &mint.key(),
            &pda_authority.key(),
            &[],
            amount
        ).unwrap(),

        // Accounts
        &[
            target_wallet,
            mint,
            pda_authority,
        ],

        // pda_authority seeds
        &[pda_authority_seeds]
    ).unwrap();
}


pub fn create_pda_token_account<'info> (
    account        : AccountInfo<'info>,
    account_seeds  : &[&[u8]],
    mint           : AccountInfo<'info>,
    authority      : AccountInfo<'info>,
    payer          : AccountInfo<'info>,
    rent           : Sysvar<'info, Rent>,
    token_program  : AccountInfo<'info>,
    system_program : AccountInfo<'info>,
) {
    
    let rent_lamports: u64 = rent.minimum_balance(TokenAccount::LEN);

    let create_account_ix = solana_program::system_instruction::create_account(
        &payer.key(),
        &account.key(),
        rent_lamports,
        TokenAccount::LEN as u64,
        &token_program.key()
    );

    invoke_signed(
        &create_account_ix,
        &[
            payer,
            account.clone(),
            system_program.clone()
        ],
        &[account_seeds]
    ).unwrap();

    let cpi_ctx = CpiContext::new(
        token_program,
        InitializeAccount {
            account,
            mint,
            authority,
            rent : rent.to_account_info()
        }
    );

    initialize_account(cpi_ctx).unwrap();
}
