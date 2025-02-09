use anchor_lang::prelude::*;

mod state;
mod instructions;
mod error;

use instructions::*;

declare_id!("HmgqrHLhTBizDVFWfypYzwX7ngKofNLdbnX3feD73FzU");

#[program]
pub mod borrow {
    use super::*;

    pub fn init_bank(
        ctx: Context<InitBank>,
        liquidation_threshold: u64,
        max_ltv: u64,
    ) -> Result<()> {
        process_init_bank(ctx, liquidation_threshold, max_ltv)
    }
    
    pub fn init_user(
        ctx: Context<InitUser>,
        usdc_address: Pubkey,
    ) -> Result<()> {
        process_init_user(ctx, usdc_address)
    }

    pub fn deposit(
        ctx: Context<Deposit>,
        amount: u64,
    ) -> Result<()> {
        process_deposit(ctx, amount)
    }

    pub fn withdraw(
        ctx: Context<Withdraw>,
        amount: u64,
    ) -> Result<()> {
        process_withdraw(ctx, amount)
    }
    
}
