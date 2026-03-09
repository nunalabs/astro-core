# astro-core - Shared Infrastructure

> **Role**: SINGLE SOURCE OF TRUTH for shared Rust/Soroban code

## Context

**Purpose**: Canonical math, types, and error definitions for the entire Astro ecosystem

**Consumers**: astro-launchpad, astro-swap (NEVER duplicate this code)

**Current Version**: v1.5.0 (Protocol 25, soroban-sdk 25.2.0)

## Critical: Shared Library

```
contracts/shared/src/
├── math/       # Safe arithmetic, AMM calculations
├── types.rs    # SharedError, common types
└── lib.rs      # Re-exports (public API)
```

**Key Functions**:
- `apply_bps_round_up(amount, bps)` - Fees (round UP)
- `apply_bps(amount, bps)` - Percentage (round DOWN)
- `get_amount_out(in, r_in, r_out, fee)` - AMM swap output
- `MIN_TRADE_AMOUNT` - 0.1 XLM minimum

## Commands

```bash
cargo build --release --target wasm32-unknown-unknown
cargo test --workspace
cargo clippy --workspace
cargo fmt --all
```

## Versioning

```bash
# Tag new version
git tag v1.6.0 && git push --tags

# Consumers auto-update via:
cd .. && make sync-deps
```

## Rules

1. **Never breaking changes** without coordinating with consumers
2. **Always round fees UP** (protocol protection)
3. **Always round outputs DOWN** (liquidity protection)
4. **Add tests** for all math functions

---

**Lines**: ~60 | **Type**: Library | **Consumers**: 2
