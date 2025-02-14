use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token_interface::{Mint, TokenAccount, TokenInterface, TransferChecked, transfer_checked}};
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};

use crate::{constants, math::checked_mul_u64, state::{Bank, User}, utils::calculate_accrued_interest};
use crate::error::ErrorCode;

#[derive(Accounts)]
pub struct Liquidate<'info> {
    #[account(mut)]
    pub liquidator: Signer<'info>,

    pub price_update: Account<'info, PriceUpdateV2>,

    pub collateral_mint: InterfaceAccount<'info, Mint>,
    pub borrowed_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds = [collateral_mint.key().as_ref()],
        bump,
    )]
    pub collateral_bank: Account<'info, Bank>,

    #[account(
        mut,
        seeds = [borrowed_mint.key().as_ref()],
        bump,
    )]
    pub borrowed_bank: Account<'info, Bank>,

    #[account(
        mut,
        seeds = [b"treasury", collateral_mint.key().as_ref()],
        bump,
    )]
    pub collateral_bank_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"treasury", borrowed_mint.key().as_ref()],
        bump,
    )]
    pub borrowed_bank_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [liquidator.key().as_ref()],
        bump,
    )]
    pub user_account: Account<'info, User>,

    #[account(
        init_if_needed,
        payer = liquidator,
        associated_token::mint = collateral_mint,
        associated_token::authority = liquidator,
        associated_token::token_program = token_program,
    )]
    pub liquidator_collateral_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = liquidator,
        associated_token::mint = borrowed_mint,
        associated_token::authority = liquidator,
        associated_token::token_program = token_program,
    )]
    pub liquidator_borrowed_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn process_liquidate(
    ctx: Context<Liquidate>,
) -> Result<()> {
    let collateral_bank = &mut ctx.accounts.collateral_bank;
    let borrowed_bank = &mut ctx.accounts.borrowed_bank;
    let user = &mut ctx.accounts.user_account;

    let price_update = &ctx.accounts.price_update;
    let sol_feed_id = get_feed_id_from_hex(constants::SOL_USD_FEED_ID)?;
    let sol_price = price_update.get_price_no_older_than(&Clock::get()?, constants::MAX_AGE, &sol_feed_id)?;
    let usdc_feed_id = get_feed_id_from_hex(constants::USDC_USD_FEED_ID)?;
    let usdc_price = price_update.get_price_no_older_than(&Clock::get()?, constants::MAX_AGE, &usdc_feed_id)?;

    let (total_collateral, total_borrowed) = match ctx.accounts.collateral_mint.key() {
        key if key == user.usdc_address => {
            let new_usdc =  calculate_accrued_interest(
                user.deposited_usdc,
                collateral_bank.interest_rate,
                user.last_updated_deposit,
            )?;
            
            let new_sol =  calculate_accrued_interest(
                user.borrowed_sol,
                borrowed_bank.interest_rate,
                user.last_updated_borrow,
            )?;
            
            (usdc_price.price as u64 * new_usdc, sol_price.price as u64 * new_sol)
        }
        _ => {
            let new_sol =  calculate_accrued_interest(
                user.deposited_sol,
                collateral_bank.interest_rate,
                user.last_updated_deposit,
            )?;

            let new_usdc =  calculate_accrued_interest(
                user.borrowed_usdc,
                borrowed_bank.interest_rate,
                user.last_updated_borrow,
            )?;

            (sol_price.price as u64 * new_sol, usdc_price.price as u64 * new_usdc)
        }
    };

    let health_factor = (total_collateral as f64 * collateral_bank.liquidation_threshold as f64) / total_borrowed as f64;

    if health_factor > 1.0 {
        return Err(ErrorCode::InvalidLiquidation.into());
    }

    let transfer_to_bank = TransferChecked {
        from: ctx.accounts.liquidator_borrowed_token_account.to_account_info(),
        to: ctx.accounts.borrowed_bank_token_account.to_account_info(),
        authority: ctx.accounts.liquidator_borrowed_token_account.to_account_info(),
        mint: ctx.accounts.borrowed_mint.to_account_info(),
    };

    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, transfer_to_bank);
    let decimals = ctx.accounts.borrowed_mint.decimals;

    let liquidation_amount = checked_mul_u64(total_borrowed, borrowed_bank.liquidation_close_factor)?;

    transfer_checked(
        cpi_ctx,
        liquidation_amount,
        decimals,
    )?;

    let liquidator_amount = (liquidation_amount * collateral_bank.liquidation_bonus) + liquidation_amount;

    let transfer_to_liquidator = TransferChecked {
        from: ctx.accounts.collateral_bank_token_account.to_account_info(),
        to: ctx.accounts.liquidator_collateral_token_account.to_account_info(),
        authority: ctx.accounts.collateral_bank_token_account.to_account_info(),
        mint: ctx.accounts.collateral_mint.to_account_info(),
    };

    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, transfer_to_liquidator);
    let decimals = ctx.accounts.collateral_mint.decimals;
    let mint_key = ctx.accounts.collateral_mint.key();

    let signer_seeds: &[&[&[u8]]] = &[
        &[
            b"treasury",
            mint_key.as_ref(),
            &[ctx.bumps.collateral_bank_token_account]
        ]
    ];

    transfer_checked(
        cpi_ctx.with_signer(signer_seeds),
        liquidator_amount,
        decimals,
    )?;

    Ok(())
}