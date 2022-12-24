use anchor_lang::prelude::*;

declare_id!("4Zo7npJHcpHhnj9dUXAY2ZzpuZT75AFn1z3uD6WqgBKu");

#[program]
pub mod polymerase_sender {
    use super::*;

    // Create account for IBC packet data
    pub fn send_ibc_packet(
        ctx: Context<SendIbcPacket>,
        channel_id: [u8; 32],
        payload: Vec<u8>,
        timeout_block_height: u64,
    ) -> Result<()> {

        if payload.len() > MAX_PAYLOAD_LENGTH {
            return Err(Errors::DataTooLong.into());
        }

        // TODO: if channel_id is targeting Solana chain this is where
        //        one should wrap the payload as [data, accounts]

        let ibc_data: &mut Account<IbcData> = &mut ctx.accounts.ibc_data;

        ibc_data.sender               = *ctx.accounts.sender.key;
        ibc_data.payer                = *ctx.accounts.payer.key;
        ibc_data.channel_id           = channel_id;
        ibc_data.payload              = payload;
        ibc_data.timeout_block_height = timeout_block_height;

        Ok(())
    }

    // TODO: make sure only only the relayer can call this!!!
    // Close Ibc data account. 
    pub fn close_account(_ctx: Context<CloseAccount>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct SendIbcPacket<'info> {
    #[account(
        init,
        payer = payer,
        space = IbcData::LEN,
    )]
    pub ibc_data: Account<'info, IbcData>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account()]
    pub sender: Signer<'info>,
    pub system_program: Program<'info, System>,
}

// TODO: test that only the relayer can call this!!!
#[derive(Accounts)]
pub struct CloseAccount<'info> {
    #[account(
        mut, 
        close = payer,
        has_one = payer
    )]
    pub ibc_data: Account<'info, IbcData>,
    #[account(mut)]
    /// CHECK: must match the one saved in ibc_data
    pub payer: AccountInfo<'info>,
    #[account(mut)]
    pub authority: Signer<'info>, // relayer
    pub system_program: Program<'info, System>,
}

// Accounts ****************************************************************************************

const DISCRIMINATOR_LENGTH        : usize = 8;
const PUBLIC_KEY_LENGTH           : usize = 32;
const CHANNEL_ID_LENGTH           : usize = 32;
const VECTOR_LENGTH_PREFIX        : usize = 4;   // stores the size of the vector.
const MAX_PAYLOAD_LENGTH          : usize = 512; // max length
const TIMEOUT_BLOCK_HEIGHT_LENGTH : usize = 8;

#[account]
pub struct IbcData {
    pub sender: Pubkey,            // 32 bytes
    pub payer: Pubkey,             // 32 bytes
    pub channel_id: [u8; 32],      // 32 bytes
    pub payload: Vec<u8>,          // 1024 bytes (arbitrary)
    pub timeout_block_height: u64, // 8 bytes
}

impl IbcData {
    const LEN: usize = DISCRIMINATOR_LENGTH
        + PUBLIC_KEY_LENGTH                         // sender
        + PUBLIC_KEY_LENGTH                         // payer
        + CHANNEL_ID_LENGTH                         // channel id
        + VECTOR_LENGTH_PREFIX + MAX_PAYLOAD_LENGTH // payload
        + TIMEOUT_BLOCK_HEIGHT_LENGTH;              // timeout block height
}

#[error_code]
pub enum Errors {
    #[msg("The provided data should be 512 chars long maximum")]
    DataTooLong,
}
