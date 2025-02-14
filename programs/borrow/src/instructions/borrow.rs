use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token_interface::{Mint, TokenAccount, TokenInterface, TransferChecked, transfer_checked}};
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};

use crate::{constants, math::checked_mul_u64, state::{Bank, User}, utils::calculate_accrued_interest};
use crate::error::ErrorCode;

#[derive(Accounts)]
pub struct Borrow<'info> {
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
        init_if_needed,
        payer = signer,
        associated_token::mint = mint,
        associated_token::authority = signer,
        associated_token::token_program = token_program,
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    pub price_update: Account<'info, PriceUpdateV2>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn process_borrow(
    ctx: Context<Borrow>,
    amount: u64,
) -> Result<()> {
    let bank = &mut ctx.accounts.bank;
    let user = &mut ctx.accounts.user_account;
    let price_update = &ctx.accounts.price_update;

    let total_collateral: u64 = match ctx.accounts.mint.key() {
        key if key == user.usdc_address => {
            let sol_feed_id = get_feed_id_from_hex(constants::SOL_USD_FEED_ID)?;
            let sol_price = price_update.get_price_no_older_than(&Clock::get()?, constants::MAX_AGE, &sol_feed_id)?;
            let new_value =  calculate_accrued_interest(
                user.deposited_usdc,
                bank.interest_rate,
                user.last_updated_deposit,
            )?;
            sol_price.price as u64 * new_value
        }
        _ => {
            let usdc_feed_id = get_feed_id_from_hex(constants::USDC_USD_FEED_ID)?;
            let sol_price = price_update.get_price_no_older_than(&Clock::get()?, constants::MAX_AGE, &usdc_feed_id)?;
            let new_value =  calculate_accrued_interest(
                user.deposited_usdc,
                bank.interest_rate,
                user.last_updated_deposit,
            )?;
            sol_price.price as u64 * new_value
        }
    };



    let borrowable_value = checked_mul_u64(total_collateral, bank.liquidation_threshold)?;

    if borrowable_value < amount {
        return Err(ErrorCode::InsufficientCollateral.into());
    }

    let transfer_cpi_accounts = TransferChecked {
        from: ctx.accounts.bank_token_account.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        authority: ctx.accounts.bank_token_account.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
    };

    let cpi_program = ctx.accounts.token_program.to_account_info();

    let mint_key = ctx.accounts.mint.key();

    let signer_seeds: &[&[&[u8]]] = &[
        &[
            b"treasury",
            mint_key.as_ref(),
            &[ctx.bumps.bank_token_account]
        ]
    ];

    let cpi_ctx = CpiContext::new(cpi_program, transfer_cpi_accounts)
        .with_signer(signer_seeds);

    let decimals = ctx.accounts.mint.decimals;

    transfer_checked(
        cpi_ctx,
        amount,
        decimals,
    )?;

    if bank.total_borrowed == 0 {
        bank.total_borrowed = amount;
        bank.total_borrowed_shares = amount;
    }

    let borrow_ratio = match amount.checked_div(bank.total_borrowed) {
        Some(ratio) => ratio,
        None => return Err(ErrorCode::MathError.into()),
    };

    let user_shares = match bank.total_borrowed_shares.checked_mul(borrow_ratio) {
        Some(shares) => shares,
        None => return Err(ErrorCode::MathError.into()),
    };

    match ctx.accounts.mint.key() {
        key if key == user.usdc_address => {
            user.borrowed_usdc += amount;
            user.borrowed_usdc_shares += user_shares;
        }
        _ => {
            user.borrowed_sol += amount;
            user.borrowed_sol_shares += user_shares;
        }
    }


    bank.total_borrowed += amount;
    bank.total_borrowed_shares += user_shares;

    user.last_updated_borrow = Clock::get()?.unix_timestamp;

    Ok(())
}
