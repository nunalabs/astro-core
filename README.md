# Astro Core

> Shared infrastructure contracts for the Astro DeFi ecosystem on Stellar/Soroban

[![Soroban SDK](https://img.shields.io/badge/soroban--sdk-22.0.0-blue)](https://github.com/stellar/rs-soroban-sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Tests](https://img.shields.io/badge/tests-27%20passing-brightgreen)]()

## Overview

Astro Core provides the foundational smart contracts and shared utilities for the Astro Protocol ecosystem on Stellar. This repository is designed as a multi-repo dependency, allowing other Astro projects (Astro-Shiba, AstroSwap) to consume these contracts via git tags while maintaining separation of concerns.

**Core Principles:**
- Modular architecture with reusable components
- Professional Rust patterns with comprehensive error handling
- Gas-optimized implementations
- Extensive test coverage
- Production-ready security practices

## Architecture

```
astro-core/
├── Cargo.toml (workspace)
│
├── contracts/
│   ├── shared/              # Foundation Layer
│   │   ├── types.rs         # Shared types & error handling
│   │   ├── math.rs          # Safe arithmetic operations
│   │   ├── events.rs        # Standardized event emission
│   │   └── interfaces.rs    # Cross-contract interfaces
│   │
│   ├── treasury/            # Multi-Token Vault
│   │   └── lib.rs           # Secure treasury management
│   │
│   ├── fee-distributor/     # Protocol Fee Handler
│   │   └── lib.rs           # 50/30/20 distribution model
│   │
│   ├── staking/             # Reward Pool
│   │   └── lib.rs           # Reward-per-token accounting
│   │
│   └── locker/              # LP Lock Manager
│       └── lib.rs           # Time-based & permanent locks
│
└── target/
    └── wasm32-unknown-unknown/
        └── release/         # Compiled .wasm contracts
```

## Contracts

### 1. Shared Library (`astro-core-shared`)

**Purpose:** Foundation layer providing common functionality across all contracts.

**Features:**
- `SharedError`: Unified error handling with 15+ error types
- `safe_add`, `safe_mul`, `safe_div`: Overflow-safe arithmetic
- `EventBuilder`: Standardized event emission
- Shared types: `DistributionConfig`, `UserStake`, `LockInfo`
- TTL management utilities

**Usage:** Included as a local dependency in all other contracts.

```rust
use astro_core_shared::{
    types::{SharedError, extend_instance_ttl},
    math::{safe_add, BPS_DENOMINATOR},
    events::emit_distribution,
};
```

### 2. Treasury (`astro-treasury`)

**Purpose:** Multi-token vault for protocol revenue storage and management.

**Key Functions:**
- `initialize(admin)` - Set up treasury with admin
- `deposit(token, amount)` - Accept protocol fees
- `withdraw(token, amount, recipient)` - Admin-controlled withdrawals
- `get_balance(token)` - Query treasury balances
- Emergency withdrawal capabilities

**Features:**
- Multi-token support (USDC, XLM, custom tokens)
- Admin-only withdrawals with event tracking
- Pausable for security incidents
- TTL management for persistent storage

**Tests:** 5 passing

### 3. Fee Distributor (`astro-fee-distributor`)

**Purpose:** Automated fee distribution engine implementing the protocol's economic model.

**Distribution Model:**
```
Total Fees (100%)
├── Treasury: 50%  → Protocol reserves
├── Staking:  30%  → Reward pool
└── Burn:     20%  → Deflationary mechanism
```

**Key Functions:**
- `initialize(admin, config)` - Configure distribution ratios (basis points)
- `distribute(token, amount)` - Execute distribution logic
- `update_config(new_config)` - Adjust distribution percentages
- `add_supported_token(token)` - Enable new token support

**Features:**
- Configurable distribution ratios (validated to sum to 10,000 bps)
- Support for multiple fee tokens
- Batch distribution support
- Event emission for transparency

**Tests:** 5 passing

### 4. Staking Pool (`astro-staking`)

**Purpose:** Single-sided staking contract with multi-token reward distribution.

**Algorithm:** Reward-per-token accounting for fair, gas-efficient distribution.

**Key Functions:**
- `stake(amount)` - Deposit ASTRO tokens
- `unstake(amount)` - Withdraw staked tokens
- `claim_rewards(tokens)` - Harvest accumulated rewards
- `compound()` - Auto-restake rewards
- `add_rewards(token, amount)` - Inject new rewards (fee distributor)

**Features:**
- No lockup period (flexible staking)
- Multi-token rewards (USDC, XLM, etc.)
- Precision: 1e18 for accurate calculations
- Time-weighted reward distribution
- Emergency unstake capabilities

**Tests:** 6 passing

### 5. Liquidity Locker (`astro-locker`)

**Purpose:** LP token locking mechanism for graduated tokens from Astro-Shiba launchpad.

**Lock Types:**
- **Time-based locks:** Configurable duration (1 day to 4 years)
- **Permanent locks:** Irreversible burns for maximum trust
- **Early unlock:** Optional with penalty (configurable percentage)

**Key Functions:**
- `lock(lp_token, amount, duration, permanent)` - Create lock
- `unlock(lock_id)` - Release after duration
- `extend_lock(lock_id, additional_time)` - Extend duration
- `transfer_lock(lock_id, new_owner)` - Transfer ownership
- `early_unlock(lock_id)` - Unlock with penalty

**Features:**
- Multiple locks per user/token
- Lock ID system for tracking
- Penalty mechanism (treasury forwarding)
- Permanent burn capability
- Query functions for lock info

**Tests:** 4 passing

## Quick Start

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add Wasm target
rustup target add wasm32-unknown-unknown

# Install Soroban CLI
cargo install --locked soroban-cli --version 22.0.0
```

### Build All Contracts

```bash
# Clone repository
git clone https://github.com/AstroFinance/astro-core.git
cd astro-core

# Build all contracts
cargo build --release --target wasm32-unknown-unknown

# Output: target/wasm32-unknown-unknown/release/*.wasm
```

### Run Tests

```bash
# Run all tests (27 total)
cargo test

# Run specific contract tests
cargo test -p astro-treasury
cargo test -p astro-fee-distributor
cargo test -p astro-staking
cargo test -p astro-locker

# Run with output
cargo test -- --nocapture
```

### Optimize Contracts

```bash
# Install optimizer
cargo install soroban-cli

# Optimize all contracts
soroban contract optimize --wasm target/wasm32-unknown-unknown/release/astro_treasury.wasm
soroban contract optimize --wasm target/wasm32-unknown-unknown/release/astro_fee_distributor.wasm
soroban contract optimize --wasm target/wasm32-unknown-unknown/release/astro_staking.wasm
soroban contract optimize --wasm target/wasm32-unknown-unknown/release/astro_locker.wasm
```

## Usage from Other Repositories

Astro Core is designed to be consumed as a git dependency by other projects in the Astro ecosystem.

### Add to Cargo.toml

```toml
[dependencies]
astro-core-shared = { git = "https://github.com/AstroFinance/astro-core.git", tag = "v1.0.0" }
```

### Version Pinning

```toml
# Pin to specific version
astro-core-shared = { git = "https://github.com/AstroFinance/astro-core.git", tag = "v1.0.0" }

# Pin to branch (development)
astro-core-shared = { git = "https://github.com/AstroFinance/astro-core.git", branch = "develop" }

# Pin to commit (maximum stability)
astro-core-shared = { git = "https://github.com/AstroFinance/astro-core.git", rev = "abc123def456" }
```

### Example Integration

```rust
// In your contract
use astro_core_shared::{
    types::{SharedError, DistributionConfig},
    math::{safe_add, safe_mul, BPS_DENOMINATOR},
    events::emit_distribution,
};

#[contract]
pub struct MyContract;

#[contractimpl]
impl MyContract {
    pub fn my_function(env: Env, amount: i128) -> Result<i128, SharedError> {
        let fee = safe_mul(amount, 30)?; // 0.3%
        let fee_amount = safe_div(fee, BPS_DENOMINATOR)?;
        Ok(fee_amount)
    }
}
```

## Deployment

### Testnet Deployment

```bash
# Set network
soroban config network add --global testnet \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015"

# Deploy contracts
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/astro_treasury.wasm \
  --network testnet

# Initialize contracts (example: fee-distributor)
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- initialize \
  --admin <ADMIN_ADDRESS> \
  --config '{"treasury_bps": 5000, "staking_bps": 3000, "burn_bps": 2000}'
```

### Mainnet Deployment

```bash
# Set mainnet network
soroban config network add --global mainnet \
  --rpc-url https://soroban-mainnet.stellar.org:443 \
  --network-passphrase "Public Global Stellar Network ; September 2015"

# Deploy (same process as testnet)
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/astro_treasury.wasm \
  --network mainnet
```

## Testing

### Test Coverage

| Contract | Tests | Coverage |
|----------|-------|----------|
| `shared` | 7 | Core utilities |
| `treasury` | 5 | Deposit/withdraw flows |
| `fee-distributor` | 5 | Distribution logic |
| `staking` | 6 | Stake/unstake/rewards |
| `locker` | 4 | Lock/unlock mechanics |
| **Total** | **27** | **All passing** |

### Running Specific Test Suites

```bash
# Integration tests
cargo test --test integration

# Unit tests only
cargo test --lib

# Test with verbose output
cargo test -- --nocapture --test-threads=1
```

## Error Handling

All contracts use `Result<T, SharedError>` with comprehensive error types:

```rust
pub enum SharedError {
    AlreadyInitialized,      // Contract already set up
    NotInitialized,          // Must call initialize() first
    Unauthorized,            // Caller not authorized
    InsufficientBalance,     // Not enough tokens
    InvalidAmount,           // Amount must be > 0
    InvalidConfig,           // Configuration validation failed
    Paused,                  // Contract paused by admin
    Overflow,                // Arithmetic overflow
    LockNotExpired,          // Lock still active
    InvalidLockId,           // Lock ID not found
    PermanentLock,           // Cannot unlock permanent lock
    TransferFailed,          // Token transfer failed
    InvalidBps,              // Basis points invalid (>10000)
    TokenNotSupported,       // Token not in whitelist
    EmergencyMode,           // Emergency mode active
}
```

## Security Features

- **Access Control:** Admin-only functions with `require_auth`
- **Pausability:** Emergency pause mechanism
- **Safe Math:** All arithmetic uses overflow-checked operations
- **Input Validation:** Comprehensive parameter validation
- **Event Emission:** Full audit trail via events
- **TTL Management:** Automatic storage extension
- **Emergency Withdrawals:** Admin recovery mechanisms

## Contributing

Contributions are welcome! Please follow these guidelines:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Write tests for new functionality
4. Ensure all tests pass (`cargo test`)
5. Format code (`cargo fmt`)
6. Run linter (`cargo clippy`)
7. Commit changes (`git commit -m 'Add amazing feature'`)
8. Push to branch (`git push origin feature/amazing-feature`)
9. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Links

- **Documentation:** [docs.astro.finance](https://docs.astro.finance)
- **Website:** [astro.finance](https://astro.finance)
- **Discord:** [discord.gg/astro](https://discord.gg/astro)
- **Twitter:** [@AstroFinance](https://twitter.com/AstroFinance)

## Related Repositories

- [Astro-Shiba](https://github.com/AstroFinance/Astro-Shiba) - Launchpad with bonding curves
- [AstroSwap](https://github.com/AstroFinance/astroswap) - Professional DEX AMM
- [Astro SDK](https://github.com/AstroFinance/astro-sdk) - TypeScript SDK for all contracts

---

**Built with Rust + Soroban for Stellar**
