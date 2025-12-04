# CLAUDE.md - Astro Core

> **Astro Core is the SINGLE SOURCE OF TRUTH for shared code across the Astro ecosystem.**

## Overview

Astro Core provides shared Rust/Soroban libraries used by:
- **astro-launchpad**: Token factory, bonding curves, AMM
- **astro-swap**: DEX factory, pair pools, router

**Current Version**: Check latest git tag (e.g., `v1.1.0`)

## Repository Structure

```
astro-core/
├── contracts/
│   ├── shared/           # CANONICAL SHARED LIBRARY
│   │   └── src/
│   │       ├── lib.rs    # Re-exports all modules
│   │       ├── math/     # Safe arithmetic, AMM calculations
│   │       ├── types.rs  # SharedError, common types
│   │       ├── interfaces.rs
│   │       └── events.rs
│   │
│   ├── treasury/         # Treasury vault contract
│   ├── fee-distributor/  # Fee distribution (50/30/20)
│   ├── staking/          # ASTRO staking contract
│   └── locker/           # LP token locker
│
└── CLAUDE.md             # This file
```

## Key Module: math

**Location**: `contracts/shared/src/math/mod.rs`

This is the canonical math library. **Never duplicate these functions elsewhere.**

### Constants

```rust
pub const PRECISION: i128 = 1_000_000_000_000_000_000;  // 1e18
pub const BPS_DENOMINATOR: i128 = 10_000;               // 100%
pub const STELLAR_DECIMALS: u32 = 7;
pub const ONE_TOKEN: i128 = 10_000_000;                 // 1 token with 7 decimals
pub const MIN_TRADE_AMOUNT: i128 = 1_000_000;           // 0.1 XLM (dust prevention)
```

### Safe Arithmetic

```rust
safe_add(a, b) -> Result<i128, SharedError>    // Overflow protected
safe_sub(a, b) -> Result<i128, SharedError>    // Underflow protected (rejects negative)
safe_mul(a, b) -> Result<i128, SharedError>    // Overflow protected
safe_div(a, b) -> Result<i128, SharedError>    // Division by zero protected
```

### Phantom Overflow Protection

```rust
mul_div_down(a, b, c) -> Result<i128>  // (a * b) / c, rounds DOWN
mul_div_up(a, b, c) -> Result<i128>    // (a * b) / c, rounds UP
```

Handles cases where `a * b` overflows but `(a * b) / c` fits in i128.

### Basis Points Operations

```rust
apply_bps(amount, bps) -> Result<i128>          // Rounds DOWN (standard)
apply_bps_round_up(amount, bps) -> Result<i128> // Rounds UP (for fees)
calculate_bps(part, total) -> Result<u32>       // Calculate percentage
sub_bps(amount, bps) -> Result<i128>            // amount - percentage
```

### AMM Calculations

```rust
get_amount_out(amount_in, reserve_in, reserve_out, fee_bps) -> Result<i128>
get_amount_in(amount_out, reserve_in, reserve_out, fee_bps) -> Result<i128>
quote(amount_a, reserve_a, reserve_b) -> Result<i128>
sqrt(value) -> i128
```

### Reserve Operations

```rust
update_reserves_add(r0, r1, a0, a1) -> Result<(i128, i128)>
update_reserves_sub(r0, r1, a0, a1) -> Result<(i128, i128)>
update_reserves_swap(r_in, r_out, a_in, a_out, is_0_in) -> Result<(i128, i128)>
calculate_k(reserve_0, reserve_1) -> Result<i128>
verify_k_invariant(new_r0, new_r1, old_r0, old_r1) -> Result<bool>
```

## Commands

```bash
# Build
cargo build --release --target wasm32-unknown-unknown

# Test
cargo test --workspace

# Format
cargo fmt --all

# Lint
cargo clippy --workspace
```

## Usage in Consumer Repos

### Cargo.toml

```toml
[dependencies]
astro-core-shared = { git = "https://github.com/nunalabs/astro-core", tag = "v1.1.0" }
```

### Import

```rust
use astro_core_shared::{
    SharedError,
    math::{
        safe_add, safe_sub, safe_mul, safe_div,
        apply_bps, apply_bps_round_up,
        get_amount_out, get_amount_in,
        MIN_TRADE_AMOUNT, BPS_DENOMINATOR,
    },
};
```

## Rounding Philosophy

| Operation | Rounding | Rationale |
|-----------|----------|-----------|
| Fee calculation | UP | Protocol always receives ≥1 stroop |
| Token output | DOWN | Protect liquidity pool |
| XLM output | DOWN | Protect liquidity pool |
| Price quotes | DOWN | Conservative estimate |

## Version History

| Version | Changes |
|---------|---------|
| v1.0.0 | Initial release |
| v1.0.2 | Production stability |
| v1.1.0 | Added `apply_bps_round_up`, `MIN_TRADE_AMOUNT` |

## Error Types

`SharedError` (from `types.rs`):
- `Overflow` - Arithmetic overflow
- `Underflow` - Arithmetic underflow or negative result
- `DivisionByZero` - Division by zero
- `InvalidAmount` - Invalid input (negative, etc.)
- `InsufficientBalance` - Not enough funds

## Contributing

1. Changes to shared code require careful review
2. Add tests for all new functions
3. Document with examples
4. Tag new versions for consumers to pin
5. Coordinate updates across repos

## Quick Reference

```rust
// Fee calculation (always round up)
let fee = apply_bps_round_up(amount, fee_bps)?;

// Swap output (always round down via mul_div_down internally)
let output = get_amount_out(input, reserve_in, reserve_out, fee_bps)?;

// Validate minimum trade
if amount < MIN_TRADE_AMOUNT {
    return Err(Error::AmountBelowMinimum);
}
```
