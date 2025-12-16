---
name: safe-arithmetic
description: Safe arithmetic patterns for astro-core-shared math functions. Critical for financial calculations.
---

# Safe Arithmetic Skill

## Core Principle

**All financial math MUST use safe operations.** No native Rust arithmetic.

## Safe Operations

```rust
// ALWAYS use these from astro-core-shared
use astro_core_shared::math::{
    safe_add,      // a + b with overflow check
    safe_sub,      // a - b with underflow check (rejects negative)
    safe_mul,      // a * b with overflow check
    safe_div,      // a / b with div-by-zero check
    mul_div_down,  // (a * b) / c rounded DOWN
    mul_div_up,    // (a * b) / c rounded UP
};

// Example: Fee calculation
let fee = apply_bps_round_up(amount, fee_bps)?;  // GOOD
let fee = amount * fee_bps / 10000;               // BAD - no safety

// Example: Swap output
let output = get_amount_out(input, r_in, r_out, fee)?;  // GOOD
let output = input * r_out / r_in;                       // BAD - wrong formula
```

## Rounding Rules

| Operation | Round | Why |
|-----------|-------|-----|
| Fee calculation | UP | Protocol receives ≥1 stroop |
| Token output | DOWN | Protect liquidity |
| XLM output | DOWN | Protect liquidity |
| LP mint | DOWN | Protect existing LPs |

```rust
// Fee: Round UP
let fee = apply_bps_round_up(amount, 30)?;  // 0.30% fee

// Output: Round DOWN (via mul_div_down internally)
let output = get_amount_out(input, r_in, r_out, 0)?;

// Custom calculation: Choose appropriate rounding
let share = mul_div_down(amount, total_supply, total_liquidity)?;  // LP tokens
```

## Phantom Overflow

When `a * b` overflows but `(a * b) / c` fits in i128:

```rust
// BAD: Intermediate overflow
let result = a * b / c;  // May overflow at a * b

// GOOD: Phantom overflow protection
let result = mul_div_down(a, b, c)?;  // Handles safely
```

## Constants

```rust
// Always use these
const MIN_TRADE_AMOUNT: i128 = 1_000_000;    // 0.1 XLM
const BPS_DENOMINATOR: i128 = 10_000;         // 100% in bps
const ONE_TOKEN: i128 = 10_000_000;           // 1 token (7 decimals)
const STELLAR_DECIMALS: u32 = 7;

// Validate minimum
if amount < MIN_TRADE_AMOUNT {
    return Err(Error::AmountBelowMinimum);
}
```

## K Invariant

```rust
// AMM constant product
let k = reserve_0 * reserve_1;  // May overflow for large reserves!

// SAFE version
let k = safe_mul(reserve_0, reserve_1)?;

// Verification after swap
verify_k_invariant(new_r0, new_r1, old_r0, old_r1)?;  // K must not decrease
```

## Edge Cases to Handle

```rust
// Zero amounts
if amount == 0 {
    return Err(SharedError::InvalidAmount);
}

// Division by zero
let result = safe_div(a, b)?;  // Returns Err if b == 0

// Negative results (from subtraction)
let diff = safe_sub(a, b)?;  // Returns Err if a < b

// Near-max values
let big = i128::MAX / 2;
let result = safe_add(big, big)?;  // Returns Err (overflow)
```
