use anchor_lang::prelude::*;
use crate::error::ErrorCode;

pub(crate) fn checked_div_u64(a: u64, b: u64) -> Result<u64> {
    if b == 0 {
        return Err(ErrorCode::MathError.into());
    }
    Ok(a / b)
}

pub(crate) fn checked_div_f64(a: f64, b: f64) -> Result<f64> {
    if b == 0.0 {
        return Err(ErrorCode::MathError.into());
    }
    Ok(a / b)
}

pub(crate) fn checked_mul_u64(a: u64, b: u64) -> Result<u64> {
    a.checked_mul(b).ok_or(ErrorCode::MathError.into())
}