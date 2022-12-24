use anchor_lang::prelude::*;
use anchor_lang::error::Error;

use shared_lib::u256::U256;

declare_id!("FixedPointMathsPoLymer1111111111111111111111");

#[program]
pub mod fixed_point_math {
    use super::*;

    pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }

    pub fn mul_x64(ctx: Context<MathOperation>, a: [u64; 4], b: [u64; 4]) -> Result<()> {
        let calculation_data = &mut ctx.accounts.calculation_data;
        calculation_data.result = shared_lib::fixed_point_math_x64::mul_x64(U256(a), U256(b)).unwrap().0;
        Ok(())
    }
        
    pub fn div_x64(ctx: Context<MathOperation>, a: [u64; 4], b: [u64; 4]) -> Result<()> {
        let calculation_data = &mut ctx.accounts.calculation_data;
        calculation_data.result = shared_lib::fixed_point_math_x64::div_x64(U256(a), U256(b)).unwrap().0;
        Ok(())
    }
        
    pub fn log2_x64(ctx: Context<MathOperation>, x: [u64; 4]) -> Result<()> {
        let calculation_data = &mut ctx.accounts.calculation_data;
        calculation_data.result = shared_lib::fixed_point_math_x64::log2_x64(U256(x)).unwrap().0;
        Ok(())
    }
        
    pub fn ln_x64(ctx: Context<MathOperation>, x: [u64; 4]) -> Result<()> {
        let calculation_data = &mut ctx.accounts.calculation_data;
        calculation_data.result = shared_lib::fixed_point_math_x64::ln_x64(U256(x)).unwrap().0;
        Ok(())
    }
        
    pub fn pow2_x64(ctx: Context<MathOperation>, x: [u64; 4]) -> Result<()> {
        let calculation_data = &mut ctx.accounts.calculation_data;
        calculation_data.result = shared_lib::fixed_point_math_x64::pow2_x64(U256(x)).unwrap().0;
        Ok(())
    }
        
    pub fn inv_pow2_x64(ctx: Context<MathOperation>, x: [u64; 4]) -> Result<()> {
        let calculation_data = &mut ctx.accounts.calculation_data;
        calculation_data.result = shared_lib::fixed_point_math_x64::inv_pow2_x64(U256(x)).unwrap().0;
        Ok(())
    }
        
    pub fn pow_x64(ctx: Context<MathOperation>, x: [u64; 4], p: [u64; 4]) -> Result<()> {
        let calculation_data = &mut ctx.accounts.calculation_data;
        calculation_data.result = shared_lib::fixed_point_math_x64::pow_x64(U256(x), U256(p)).unwrap().0;
        Ok(())
    }
        
    pub fn inv_pow_x64(ctx: Context<MathOperation>, x: [u64; 4], p: [u64; 4]) -> Result<()> {
        let calculation_data = &mut ctx.accounts.calculation_data;
        calculation_data.result = shared_lib::fixed_point_math_x64::inv_pow_x64(U256(x), U256(p)).unwrap().0;
        Ok(())
    }
        
    pub fn exp_x64(ctx: Context<MathOperation>, x: [u64; 4]) -> Result<()> {
        let calculation_data = &mut ctx.accounts.calculation_data;
        calculation_data.result = shared_lib::fixed_point_math_x64::exp_x64(U256(x)).unwrap().0;
        Ok(())
    }
        
    pub fn inv_exp_x64(ctx: Context<MathOperation>, x: [u64; 4]) -> Result<()> {
        let calculation_data = &mut ctx.accounts.calculation_data;
        calculation_data.result = shared_lib::fixed_point_math_x64::inv_exp_x64(U256(x)).unwrap().0;
        Ok(())
    }
        
    pub fn safe_pow_x64(ctx: Context<MathOperation>, a: [u64; 4], b: [u64; 4], p: [u64; 4]) -> Result<()> {
        let calculation_data = &mut ctx.accounts.calculation_data;
        calculation_data.result = shared_lib::fixed_point_math_x64::safe_pow_x64(U256(a), U256(b), U256(p)).unwrap().0;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer=deployer, space=256 + 8)]
    pub calculation_data: Account<'info, CalculationData>,
    #[account(mut)]
    pub deployer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct MathOperation<'info> {
    #[account(mut)]
    pub calculation_data: Account<'info, CalculationData>,
}

#[account]
#[derive(Default)]
pub struct CalculationData {
    result: [u64; 4]
}

#[error_code]
pub enum ErrorCode {
    #[msg("Arithmetic Error. Possible overflow/underflow.")]
    ArithmeticError,
}