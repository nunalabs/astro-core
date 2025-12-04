//! # Safe Math Module
//!
//! Safe arithmetic operations with overflow/underflow protection.
//! All operations return Result types for proper error handling.

use crate::types::SharedError;

/// High precision constant (1e18)
pub const PRECISION: i128 = 1_000_000_000_000_000_000;

/// Basis points denominator (10,000 = 100%)
pub const BPS_DENOMINATOR: i128 = 10_000;

/// Stellar decimals (7)
pub const STELLAR_DECIMALS: u32 = 7;

/// One token with 7 decimals
pub const ONE_TOKEN: i128 = 10_000_000;

/// Minimum trade amount: 0.1 XLM = 1,000,000 stroops
/// Prevents dust attacks and ensures fees don't round to 0
pub const MIN_TRADE_AMOUNT: i128 = 1_000_000;

// ════════════════════════════════════════════════════════════════════════════
// Safe Arithmetic Operations
// ════════════════════════════════════════════════════════════════════════════

/// Safe addition with overflow check
#[inline]
pub fn safe_add(a: i128, b: i128) -> Result<i128, SharedError> {
    a.checked_add(b).ok_or(SharedError::Overflow)
}

/// Safe subtraction with underflow check (also rejects negative results for token math)
#[inline]
pub fn safe_sub(a: i128, b: i128) -> Result<i128, SharedError> {
    let result = a.checked_sub(b).ok_or(SharedError::Underflow)?;
    if result < 0 {
        return Err(SharedError::Underflow);
    }
    Ok(result)
}

/// Safe multiplication with overflow check
#[inline]
pub fn safe_mul(a: i128, b: i128) -> Result<i128, SharedError> {
    a.checked_mul(b).ok_or(SharedError::Overflow)
}

/// Safe division with zero check
#[inline]
pub fn safe_div(a: i128, b: i128) -> Result<i128, SharedError> {
    if b == 0 {
        return Err(SharedError::DivisionByZero);
    }
    a.checked_div(b).ok_or(SharedError::Overflow)
}

// ════════════════════════════════════════════════════════════════════════════
// Phantom Overflow Safe Arithmetic
// ════════════════════════════════════════════════════════════════════════════

/// Multiply then divide with phantom overflow protection: (a * b) / c
/// Rounds DOWN (floor) - favors the protocol
///
/// Handles phantom overflow where a*b overflows but (a*b)/c fits in i128.
/// Uses u128 intermediate calculation to prevent overflow.
#[inline]
pub fn mul_div_down(a: i128, b: i128, c: i128) -> Result<i128, SharedError> {
    if c == 0 {
        return Err(SharedError::DivisionByZero);
    }
    if a == 0 || b == 0 {
        return Ok(0);
    }

    // Handle negative numbers - for token math we expect positive values
    if a < 0 || b < 0 || c < 0 {
        return Err(SharedError::InvalidAmount);
    }

    // Try direct calculation first (most common case)
    if let Some(product) = a.checked_mul(b) {
        return product.checked_div(c).ok_or(SharedError::Overflow);
    }

    // Phantom overflow: use u128 for intermediate calculation
    let a_u = a as u128;
    let b_u = b as u128;
    let c_u = c as u128;

    // For very large numbers, use decomposition: a*b/c = (a/c)*b + (a%c)*b/c
    let quotient = a_u / c_u;
    let remainder = a_u % c_u;

    // result = quotient * b + (remainder * b) / c
    let term1 = quotient.checked_mul(b_u).ok_or(SharedError::Overflow)?;
    let term2_num = remainder.checked_mul(b_u).ok_or(SharedError::Overflow)?;
    let term2 = term2_num / c_u;

    let result = term1.checked_add(term2).ok_or(SharedError::Overflow)?;

    if result > i128::MAX as u128 {
        return Err(SharedError::Overflow);
    }

    Ok(result as i128)
}

/// Multiply then divide with phantom overflow protection: (a * b) / c
/// Rounds UP (ceiling) - favors the user paying more / receiving less
/// Used for get_amount_in calculations
#[inline]
pub fn mul_div_up(a: i128, b: i128, c: i128) -> Result<i128, SharedError> {
    if c == 0 {
        return Err(SharedError::DivisionByZero);
    }
    if a == 0 || b == 0 {
        return Ok(0);
    }

    // Handle negative numbers
    if a < 0 || b < 0 || c < 0 {
        return Err(SharedError::InvalidAmount);
    }

    // floor((a * b + c - 1) / c) = ceil(a * b / c)
    let floor_result = mul_div_down(a, b, c)?;

    // Check if there's a remainder
    let a_u = a as u128;
    let b_u = b as u128;
    let c_u = c as u128;

    // Check remainder without overflow
    let quotient = a_u / c_u;
    let remainder_a = a_u % c_u;
    let term1_remainder = (quotient * b_u) % c_u;
    let term2_product = remainder_a * b_u;
    let term2_remainder = term2_product % c_u;

    // If there's any remainder, round up
    if term1_remainder > 0 || term2_remainder > 0 {
        safe_add(floor_result, 1)
    } else {
        Ok(floor_result)
    }
}

/// Calculate k = reserve_0 * reserve_1 with overflow protection
/// Used for constant product invariant verification
#[inline]
pub fn calculate_k(reserve_0: i128, reserve_1: i128) -> Result<i128, SharedError> {
    if reserve_0 < 0 || reserve_1 < 0 {
        return Err(SharedError::InvalidAmount);
    }
    safe_mul(reserve_0, reserve_1)
}

/// Update reserves after deposit with overflow check
#[inline]
pub fn update_reserves_add(
    reserve_0: i128,
    reserve_1: i128,
    amount_0: i128,
    amount_1: i128,
) -> Result<(i128, i128), SharedError> {
    let new_reserve_0 = safe_add(reserve_0, amount_0)?;
    let new_reserve_1 = safe_add(reserve_1, amount_1)?;
    Ok((new_reserve_0, new_reserve_1))
}

/// Update reserves after withdrawal with underflow check
/// Also validates that results are non-negative (AMM safety)
#[inline]
pub fn update_reserves_sub(
    reserve_0: i128,
    reserve_1: i128,
    amount_0: i128,
    amount_1: i128,
) -> Result<(i128, i128), SharedError> {
    let new_reserve_0 = safe_sub(reserve_0, amount_0)?;
    let new_reserve_1 = safe_sub(reserve_1, amount_1)?;
    Ok((new_reserve_0, new_reserve_1))
}

/// Update reserves after swap with overflow/underflow check
/// Also validates that reserve_out doesn't go negative (AMM safety)
#[inline]
pub fn update_reserves_swap(
    reserve_in: i128,
    reserve_out: i128,
    amount_in: i128,
    amount_out: i128,
    is_token_0_in: bool,
) -> Result<(i128, i128), SharedError> {
    let new_reserve_in = safe_add(reserve_in, amount_in)?;
    let new_reserve_out = safe_sub(reserve_out, amount_out)?;

    // AMM safety: reserves must never go negative
    if new_reserve_out < 0 {
        return Err(SharedError::Underflow);
    }

    if is_token_0_in {
        Ok((new_reserve_in, new_reserve_out))
    } else {
        Ok((new_reserve_out, new_reserve_in))
    }
}

/// Verify k invariant: k_new >= k_old (with overflow protection)
#[inline]
pub fn verify_k_invariant(
    new_reserve_0: i128,
    new_reserve_1: i128,
    old_reserve_0: i128,
    old_reserve_1: i128,
) -> Result<bool, SharedError> {
    let k_new = calculate_k(new_reserve_0, new_reserve_1)?;
    let k_old = calculate_k(old_reserve_0, old_reserve_1)?;
    Ok(k_new >= k_old)
}

// ════════════════════════════════════════════════════════════════════════════
// Basis Points Operations
// ════════════════════════════════════════════════════════════════════════════

/// Apply basis points to an amount
/// Example: apply_bps(1000, 500) = 50 (5% of 1000)
#[inline]
pub fn apply_bps(amount: i128, bps: u32) -> Result<i128, SharedError> {
    safe_div(safe_mul(amount, bps as i128)?, BPS_DENOMINATOR)
}

/// Apply basis points with ROUND UP (for fee calculations)
/// Ensures protocol always receives at least 1 stroop on any trade above MIN_TRADE_AMOUNT
/// Formula: ceil((amount * bps) / 10000) = (amount * bps + 9999) / 10000
///
/// # Arguments
/// * `amount` - The base amount
/// * `bps` - Basis points (e.g., 5 for 0.05%, 25 for 0.25%)
///
/// # Returns
/// The percentage of the amount, rounded up
///
/// # Example
/// ```rust,ignore
/// // 0.05% of 1000 = 0.5 → rounds UP to 1
/// let fee = apply_bps_round_up(1000, 5)?; // = 1
/// // Compare with round down
/// let fee_down = apply_bps(1000, 5)?; // = 0
/// ```
#[inline]
pub fn apply_bps_round_up(amount: i128, bps: u32) -> Result<i128, SharedError> {
    if bps == 0 {
        return Ok(0);
    }
    let numerator = safe_mul(amount, bps as i128)?;
    // Add (denominator - 1) before dividing to round up
    let numerator_adjusted = safe_add(numerator, BPS_DENOMINATOR - 1)?;
    safe_div(numerator_adjusted, BPS_DENOMINATOR)
}

/// Calculate percentage in basis points
/// Example: calculate_bps(50, 1000) = 500 (50 is 5% of 1000)
#[inline]
pub fn calculate_bps(part: i128, total: i128) -> Result<u32, SharedError> {
    if total == 0 {
        return Err(SharedError::DivisionByZero);
    }
    let bps = safe_div(safe_mul(part, BPS_DENOMINATOR)?, total)?;
    Ok(bps as u32)
}

/// Subtract basis points from amount
/// Example: sub_bps(1000, 500) = 950 (1000 - 5%)
#[inline]
pub fn sub_bps(amount: i128, bps: u32) -> Result<i128, SharedError> {
    let fee = apply_bps(amount, bps)?;
    safe_sub(amount, fee)
}

// ════════════════════════════════════════════════════════════════════════════
// Price & Slippage Calculations
// ════════════════════════════════════════════════════════════════════════════

/// Calculate price with precision
/// price = (reserve_quote * PRECISION) / reserve_base
#[inline]
pub fn calculate_price(reserve_base: i128, reserve_quote: i128) -> Result<i128, SharedError> {
    if reserve_base == 0 {
        return Err(SharedError::DivisionByZero);
    }
    safe_div(safe_mul(reserve_quote, PRECISION)?, reserve_base)
}

/// Calculate slippage in basis points
/// slippage = ((price_after - price_before) * 10000) / price_before
#[inline]
pub fn calculate_slippage_bps(price_before: i128, price_after: i128) -> Result<i128, SharedError> {
    if price_before == 0 {
        return Ok(0);
    }
    let diff = safe_sub(price_after, price_before)?;
    safe_div(safe_mul(diff, BPS_DENOMINATOR)?, price_before)
}

// ════════════════════════════════════════════════════════════════════════════
// AMM Calculations
// ════════════════════════════════════════════════════════════════════════════

/// Calculate output amount for AMM swap (constant product)
/// amount_out = (reserve_out * amount_in_with_fee) / (reserve_in * 10000 + amount_in_with_fee)
/// Uses mul_div_down for phantom overflow protection (rounds down, favors protocol)
pub fn get_amount_out(
    amount_in: i128,
    reserve_in: i128,
    reserve_out: i128,
    fee_bps: u32,
) -> Result<i128, SharedError> {
    if amount_in <= 0 {
        return Err(SharedError::InvalidAmount);
    }
    if reserve_in <= 0 || reserve_out <= 0 {
        return Err(SharedError::InsufficientBalance);
    }

    // Apply fee: amount_in_with_fee = amount_in * (10000 - fee)
    let fee_factor = BPS_DENOMINATOR - (fee_bps as i128);
    let amount_in_with_fee = safe_mul(amount_in, fee_factor)?;

    // denominator = reserve_in * 10000 + amount_in_with_fee
    let denominator = safe_add(safe_mul(reserve_in, BPS_DENOMINATOR)?, amount_in_with_fee)?;

    // amount_out = (amount_in_with_fee * reserve_out) / denominator
    // Use mul_div_down for phantom overflow protection (rounds down, favors protocol)
    mul_div_down(amount_in_with_fee, reserve_out, denominator)
}

/// Calculate input amount required for desired output (constant product)
/// Uses mul_div_up for phantom overflow protection (rounds up, user pays more)
pub fn get_amount_in(
    amount_out: i128,
    reserve_in: i128,
    reserve_out: i128,
    fee_bps: u32,
) -> Result<i128, SharedError> {
    if amount_out <= 0 {
        return Err(SharedError::InvalidAmount);
    }
    if reserve_in <= 0 || reserve_out <= 0 {
        return Err(SharedError::InsufficientBalance);
    }
    if amount_out >= reserve_out {
        return Err(SharedError::InsufficientBalance);
    }

    // denominator = (reserve_out - amount_out) * (10000 - fee)
    let reserve_diff = safe_sub(reserve_out, amount_out)?;
    let fee_factor = BPS_DENOMINATOR - (fee_bps as i128);
    let denominator = safe_mul(reserve_diff, fee_factor)?;

    // numerator_base = reserve_in * amount_out
    // We need: (reserve_in * amount_out * 10000) / denominator, rounded up
    // Use mul_div_up for phantom overflow protection
    let numerator_base = safe_mul(reserve_in, amount_out)?;
    let result = mul_div_up(numerator_base, BPS_DENOMINATOR, denominator)?;

    // Add 1 for additional safety margin (ensures we always have enough input)
    safe_add(result, 1)
}

/// Calculate liquidity quote
/// amount_b = (amount_a * reserve_b) / reserve_a
/// Uses mul_div_down for phantom overflow protection
pub fn quote(amount_a: i128, reserve_a: i128, reserve_b: i128) -> Result<i128, SharedError> {
    if amount_a <= 0 {
        return Err(SharedError::InvalidAmount);
    }
    if reserve_a <= 0 || reserve_b <= 0 {
        return Err(SharedError::InsufficientBalance);
    }
    mul_div_down(amount_a, reserve_b, reserve_a)
}

// ════════════════════════════════════════════════════════════════════════════
// Square Root (Newton's Method)
// ════════════════════════════════════════════════════════════════════════════

/// Calculate square root using Newton's method
/// Returns 0 for non-positive values (safe for AMM calculations)
pub fn sqrt(value: i128) -> i128 {
    if value <= 0 {
        return 0;
    }

    let mut x = value;
    let mut y = (x + 1) / 2;

    while y < x {
        x = y;
        y = (x + value / x) / 2;
    }

    x
}

// ════════════════════════════════════════════════════════════════════════════
// Min/Max Helpers
// ════════════════════════════════════════════════════════════════════════════

/// Return minimum of two values
#[inline]
pub fn min(a: i128, b: i128) -> i128 {
    if a < b {
        a
    } else {
        b
    }
}

/// Return maximum of two values
#[inline]
pub fn max(a: i128, b: i128) -> i128 {
    if a > b {
        a
    } else {
        b
    }
}

// ════════════════════════════════════════════════════════════════════════════
// Tests
// ════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_add() {
        assert_eq!(safe_add(100, 200).unwrap(), 300);
        assert!(safe_add(i128::MAX, 1).is_err());
    }

    #[test]
    fn test_safe_sub() {
        assert_eq!(safe_sub(300, 100).unwrap(), 200);
        assert!(safe_sub(100, 200).is_err());
    }

    #[test]
    fn test_safe_mul() {
        assert_eq!(safe_mul(100, 200).unwrap(), 20000);
        assert!(safe_mul(i128::MAX, 2).is_err());
    }

    #[test]
    fn test_safe_div() {
        assert_eq!(safe_div(200, 100).unwrap(), 2);
        assert!(safe_div(100, 0).is_err());
    }

    #[test]
    fn test_mul_div_down() {
        // Basic case
        assert_eq!(mul_div_down(100, 200, 50).unwrap(), 400);
        // Rounds down
        assert_eq!(mul_div_down(10, 3, 2).unwrap(), 15); // (10*3)/2 = 15
        assert_eq!(mul_div_down(10, 3, 4).unwrap(), 7);  // (10*3)/4 = 7.5 -> 7
        // Division by zero
        assert!(mul_div_down(100, 200, 0).is_err());
        // Zero inputs
        assert_eq!(mul_div_down(0, 200, 50).unwrap(), 0);
        assert_eq!(mul_div_down(100, 0, 50).unwrap(), 0);
        // Negative inputs rejected
        assert!(mul_div_down(-1, 200, 50).is_err());
        assert!(mul_div_down(100, -1, 50).is_err());
        assert!(mul_div_down(100, 200, -1).is_err());
    }

    #[test]
    fn test_mul_div_up() {
        // Basic case (exact division)
        assert_eq!(mul_div_up(100, 200, 50).unwrap(), 400);
        // Rounds up
        assert_eq!(mul_div_up(10, 3, 4).unwrap(), 8);  // (10*3)/4 = 7.5 -> 8
        // Division by zero
        assert!(mul_div_up(100, 200, 0).is_err());
    }

    #[test]
    fn test_mul_div_phantom_overflow() {
        // Test with large numbers that would overflow in direct multiplication
        let large = 10_i128.pow(30);
        let result = mul_div_down(large, large, large).unwrap();
        assert_eq!(result, large);
    }

    #[test]
    fn test_calculate_k() {
        assert_eq!(calculate_k(1000, 2000).unwrap(), 2_000_000);
        assert!(calculate_k(-1, 1000).is_err());
        assert!(calculate_k(1000, -1).is_err());
    }

    #[test]
    fn test_update_reserves_add() {
        let (r0, r1) = update_reserves_add(1000, 2000, 100, 200).unwrap();
        assert_eq!(r0, 1100);
        assert_eq!(r1, 2200);
    }

    #[test]
    fn test_update_reserves_sub() {
        let (r0, r1) = update_reserves_sub(1000, 2000, 100, 200).unwrap();
        assert_eq!(r0, 900);
        assert_eq!(r1, 1800);
        // Underflow
        assert!(update_reserves_sub(100, 200, 200, 100).is_err());
    }

    #[test]
    fn test_update_reserves_swap() {
        // Token 0 in
        let (r0, r1) = update_reserves_swap(1000, 2000, 100, 150, true).unwrap();
        assert_eq!(r0, 1100); // reserve_in increases
        assert_eq!(r1, 1850); // reserve_out decreases

        // Token 1 in
        let (r0, r1) = update_reserves_swap(1000, 2000, 100, 150, false).unwrap();
        assert_eq!(r0, 1850); // reserve_out decreases
        assert_eq!(r1, 1100); // reserve_in increases

        // Underflow protection
        assert!(update_reserves_swap(1000, 100, 50, 200, true).is_err());
    }

    #[test]
    fn test_verify_k_invariant() {
        // k increases (valid)
        assert!(verify_k_invariant(1100, 1900, 1000, 2000).unwrap());
        // k equal (valid)
        assert!(verify_k_invariant(1000, 2000, 1000, 2000).unwrap());
        // k decreases (invalid)
        assert!(!verify_k_invariant(900, 1800, 1000, 2000).unwrap());
    }

    #[test]
    fn test_apply_bps() {
        // 5% of 1000 = 50
        assert_eq!(apply_bps(1000, 500).unwrap(), 50);
        // 0.3% of 10000 = 30
        assert_eq!(apply_bps(10000, 30).unwrap(), 30);
    }

    #[test]
    fn test_apply_bps_round_up() {
        // 0.05% of 1,000,000 = 500 (exact: 1_000_000 * 5 / 10_000)
        assert_eq!(apply_bps_round_up(1_000_000, 5).unwrap(), 500);
        // 0.05% of 1000 = 0.5 → rounds UP to 1
        assert_eq!(apply_bps_round_up(1000, 5).unwrap(), 1);
        // 0.25% of 1000 = 2.5 → rounds UP to 3
        assert_eq!(apply_bps_round_up(1000, 25).unwrap(), 3);
        // 0% of anything = 0
        assert_eq!(apply_bps_round_up(1000, 0).unwrap(), 0);
        // 1% of 1000 = 10 (exact)
        assert_eq!(apply_bps_round_up(1000, 100).unwrap(), 10);
    }

    #[test]
    fn test_apply_bps_round_up_vs_down() {
        // Compare round up vs round down for small amounts
        let amount = 1000i128;
        let protocol_fee_bps = 5u32; // 0.05%

        // Round down: 1000 * 5 / 10000 = 0
        let fee_down = apply_bps(amount, protocol_fee_bps).unwrap();
        // Round up: (1000 * 5 + 9999) / 10000 = 1
        let fee_up = apply_bps_round_up(amount, protocol_fee_bps).unwrap();

        assert_eq!(fee_down, 0);
        assert_eq!(fee_up, 1);
        assert!(fee_up >= fee_down, "Round up should always be >= round down");
    }

    #[test]
    fn test_min_trade_amount() {
        // Verify MIN_TRADE_AMOUNT is 0.1 XLM
        assert_eq!(MIN_TRADE_AMOUNT, 1_000_000);
        // At MIN_TRADE_AMOUNT, protocol fee (0.05%) should be non-zero with round up
        let protocol_fee = apply_bps_round_up(MIN_TRADE_AMOUNT, 5).unwrap();
        assert!(protocol_fee > 0, "Protocol fee should be > 0 at min trade amount");
        // At MIN_TRADE_AMOUNT, LP fee (0.25%) should be non-zero
        let lp_fee = apply_bps_round_up(MIN_TRADE_AMOUNT, 25).unwrap();
        assert!(lp_fee > 0, "LP fee should be > 0 at min trade amount");
    }

    #[test]
    fn test_sqrt() {
        assert_eq!(sqrt(0), 0);
        assert_eq!(sqrt(1), 1);
        assert_eq!(sqrt(4), 2);
        assert_eq!(sqrt(9), 3);
        assert_eq!(sqrt(100), 10);
        assert_eq!(sqrt(10000), 100);
        // Negative values return 0
        assert_eq!(sqrt(-1), 0);
        assert_eq!(sqrt(-100), 0);
    }

    #[test]
    fn test_quote() {
        // 100 A with 1000/2000 reserves = 200 B
        assert_eq!(quote(100, 1000, 2000).unwrap(), 200);
        // Invalid inputs
        assert!(quote(0, 1000, 2000).is_err());
        assert!(quote(100, 0, 2000).is_err());
        assert!(quote(100, 1000, 0).is_err());
    }

    #[test]
    fn test_get_amount_out() {
        // 100 in, 1000/1000 reserves, 0.3% fee
        let out = get_amount_out(100, 1000, 1000, 30).unwrap();
        assert!(out > 0 && out < 100); // Should get less than input due to fee
        // Invalid inputs
        assert!(get_amount_out(0, 1000, 1000, 30).is_err());
        assert!(get_amount_out(100, 0, 1000, 30).is_err());
        assert!(get_amount_out(100, 1000, 0, 30).is_err());
    }

    #[test]
    fn test_get_amount_in() {
        // Want 90 out from 1000/1000 reserves with 0.3% fee
        let amount_in = get_amount_in(90, 1000, 1000, 30).unwrap();
        assert!(amount_in > 90); // Need more in due to fee
        // Invalid inputs
        assert!(get_amount_in(0, 1000, 1000, 30).is_err());
        assert!(get_amount_in(90, 0, 1000, 30).is_err());
        assert!(get_amount_in(90, 1000, 0, 30).is_err());
        // Insufficient liquidity
        assert!(get_amount_in(1000, 1000, 1000, 30).is_err());
        assert!(get_amount_in(1001, 1000, 1000, 30).is_err());
    }

    #[test]
    fn test_min_max() {
        assert_eq!(min(100, 200), 100);
        assert_eq!(min(200, 100), 100);
        assert_eq!(max(100, 200), 200);
        assert_eq!(max(200, 100), 200);
    }
}
