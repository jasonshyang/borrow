use std::f64::consts::E;

use anchor_lang::prelude::*;

pub fn calculate_accrued_interest(
    deposited_value: u64,
    interest_rate: u64,
    last_updated: i64,
) -> Result<u64> {
    let current_time = Clock::get()?.unix_timestamp;
    let time_diff = current_time - last_updated;
    let new_value = (deposited_value as f64 * E.powf(interest_rate as f64 * time_diff as f64)) as u64;
    Ok(new_value)
}