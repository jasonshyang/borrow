use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Deposit ratio overflow")]
    DepositRatioOverflow,
    #[msg("Insufficient funds")]
    InsufficientFunds,
    #[msg("Overpayment")]
    Overpayment,
    #[msg("Invalid token account")]
    InvalidTokenAccount,
    #[msg("Math error")]
    MathError,
    #[msg("Insufficient collateral")]
    InsufficientCollateral,
    #[msg("Invalid liquidation")]
    InvalidLiquidation,
}