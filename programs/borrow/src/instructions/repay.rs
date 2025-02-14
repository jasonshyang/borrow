use std::f64::consts::E;

use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token_interface::{Mint, TokenAccount, TokenInterface, TransferChecked, transfer_checked}};

use crate::{math::{checked_div_f64, checked_div_u64}, state::{Bank, User}};
use crate::error::ErrorCode;

#[derive(Accounts)]
pub struct Repay<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds = [mint.key().as_ref()],
        bump,
    )]
    pub bank: Account<'info, Bank>,

    #[account(
        mut,
        seeds = [b"treasury", mint.key().as_ref()],
        bump,
    )]
    pub bank_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [signer.key.as_ref()],
        bump,
    )]
    pub user_account: Account<'info, User>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = signer,
        associated_token::token_program = token_program,
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn process_repay(
    ctx: Context<Repay>,
    amount: u64,
) -> Result<()> {
    let user = &mut ctx.accounts.user_account;
    let bank = &mut ctx.accounts.bank;

    let borrowed_value: u64 = match ctx.accounts.mint.key() {
        key if key == user.usdc_address => user.borrowed_usdc,
        _ => user.borrowed_sol,
    };

    let time_diff = user.last_updated_borrow - Clock::get()?.unix_timestamp;

    bank.total_borrowed = (bank.total_borrowed as f64 
        * E.powf(bank.interest_rate as f64 * time_diff as f64)) as u64;

    let value_per_share = checked_div_f64(bank.total_borrowed as f64, bank.total_borrowed_shares as f64)?;
    
    let user_value = checked_div_f64(borrowed_value as f64, value_per_share)?;

    if user_value < amount as f64 {
        return Err(ErrorCode::Overpayment.into());
    }

    let transfer_cpi_accounts = TransferChecked {
        from: ctx.accounts.user_token_account.to_account_info(),
        to: ctx.accounts.bank_token_account.to_account_info(),
        authority: ctx.accounts.signer.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
    };

    let cpi_program = ctx.accounts.token_program.to_account_info();

    let cpi_ctx = CpiContext::new(cpi_program, transfer_cpi_accounts);

    let decimals = ctx.accounts.mint.decimals;

    transfer_checked(
        cpi_ctx,
        amount,
        decimals,
    )?;

    let borrow_ratio = checked_div_u64(amount, bank.total_borrowed)?;
    let user_shares = borrow_ratio * bank.total_borrowed_shares;

    match ctx.accounts.mint.key() {
        key if key == user.usdc_address => {
            user.borrowed_usdc -= amount;
            user.borrowed_usdc_shares -= user_shares;
        }
        _ => {
            user.borrowed_sol -= amount;
            user.borrowed_sol_shares -= user_shares;
        }
    }

    Ok(())
}