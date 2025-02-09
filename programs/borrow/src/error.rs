use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Deposit ratio overflow")]
    DepositRatioOverflow,
    #[msg("Insufficient funds")]
    InsufficientFunds,
}