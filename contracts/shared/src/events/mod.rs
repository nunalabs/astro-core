//! # Standard Events (SDK 25.x compatible)
//!
//! Common event emission helpers for the Astro ecosystem.
//! Using `#[contractevent]` macro for better type safety and indexing.

use soroban_sdk::{contractevent, Address, Env};

// ════════════════════════════════════════════════════════════════════════════
// Contract Events (SDK 25.x pattern)
// ════════════════════════════════════════════════════════════════════════════

/// Contract initialized event
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitializedEvent {
    #[topic]
    pub admin: Address,
    pub timestamp: u64,
}

/// Deposit event
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DepositEvent {
    #[topic]
    pub token: Address,
    pub from: Address,
    pub amount: i128,
    pub timestamp: u64,
}

/// Withdraw event
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithdrawEvent {
    #[topic]
    pub token: Address,
    pub to: Address,
    pub amount: i128,
    pub timestamp: u64,
}

/// Stake event
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StakeEvent {
    #[topic]
    pub user: Address,
    pub amount: i128,
    pub total_staked: i128,
    pub timestamp: u64,
}

/// Unstake event
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnstakeEvent {
    #[topic]
    pub user: Address,
    pub amount: i128,
    pub remaining: i128,
    pub timestamp: u64,
}

/// Claim rewards event
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimEvent {
    #[topic]
    pub user: Address,
    pub token: Address,
    pub amount: i128,
    pub timestamp: u64,
}

/// Lock event
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockEvent {
    #[topic]
    pub owner: Address,
    pub lock_id: u64,
    pub token: Address,
    pub amount: i128,
    pub unlock_time: u64,
}

/// Unlock event
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnlockEvent {
    #[topic]
    pub owner: Address,
    pub lock_id: u64,
    pub token: Address,
    pub amount: i128,
    pub timestamp: u64,
}

/// Fee distribution event
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DistributionEvent {
    #[topic]
    pub token: Address,
    pub total: i128,
    pub treasury: i128,
    pub staking: i128,
    pub burn: i128,
    pub timestamp: u64,
}

/// Admin changed event
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminChangedEvent {
    #[topic]
    pub new_admin: Address,
    pub old_admin: Address,
    pub timestamp: u64,
}

/// Contract paused/unpaused event
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PausedEvent {
    #[topic]
    pub by: Address,
    pub paused: bool,
    pub timestamp: u64,
}

// ════════════════════════════════════════════════════════════════════════════
// Helper Functions (backwards compatible API)
// ════════════════════════════════════════════════════════════════════════════

/// Emit initialization event
pub fn emit_initialized(env: &Env, admin: &Address) {
    InitializedEvent {
        admin: admin.clone(),
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);
}

/// Emit deposit event
pub fn emit_deposit(env: &Env, token: &Address, from: &Address, amount: i128) {
    DepositEvent {
        token: token.clone(),
        from: from.clone(),
        amount,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);
}

/// Emit withdraw event
pub fn emit_withdraw(env: &Env, token: &Address, to: &Address, amount: i128) {
    WithdrawEvent {
        token: token.clone(),
        to: to.clone(),
        amount,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);
}

/// Emit stake event
pub fn emit_stake(env: &Env, user: &Address, amount: i128, total_staked: i128) {
    StakeEvent {
        user: user.clone(),
        amount,
        total_staked,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);
}

/// Emit unstake event
pub fn emit_unstake(env: &Env, user: &Address, amount: i128, remaining: i128) {
    UnstakeEvent {
        user: user.clone(),
        amount,
        remaining,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);
}

/// Emit claim event
pub fn emit_claim(env: &Env, user: &Address, token: &Address, amount: i128) {
    ClaimEvent {
        user: user.clone(),
        token: token.clone(),
        amount,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);
}

/// Emit lock event
pub fn emit_lock(
    env: &Env,
    lock_id: u64,
    owner: &Address,
    token: &Address,
    amount: i128,
    unlock_time: u64,
) {
    LockEvent {
        lock_id,
        owner: owner.clone(),
        token: token.clone(),
        amount,
        unlock_time,
    }
    .publish(env);
}

/// Emit unlock event
pub fn emit_unlock(env: &Env, lock_id: u64, owner: &Address, token: &Address, amount: i128) {
    UnlockEvent {
        lock_id,
        owner: owner.clone(),
        token: token.clone(),
        amount,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);
}

/// Emit distribution event
pub fn emit_distribution(
    env: &Env,
    token: &Address,
    total: i128,
    treasury: i128,
    staking: i128,
    burn: i128,
) {
    DistributionEvent {
        token: token.clone(),
        total,
        treasury,
        staking,
        burn,
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);
}

/// Emit admin change event
pub fn emit_admin_changed(env: &Env, old_admin: &Address, new_admin: &Address) {
    AdminChangedEvent {
        old_admin: old_admin.clone(),
        new_admin: new_admin.clone(),
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);
}

/// Emit pause event
pub fn emit_paused(env: &Env, paused: bool, by: &Address) {
    PausedEvent {
        paused,
        by: by.clone(),
        timestamp: env.ledger().timestamp(),
    }
    .publish(env);
}

// ════════════════════════════════════════════════════════════════════════════
// Custom Event Builder (for contract-specific events)
// ════════════════════════════════════════════════════════════════════════════

use soroban_sdk::Symbol;

/// Builder for custom events (backwards compatible with SDK 23.x style)
/// Use this for contract-specific events not covered by standard events
pub struct EventBuilder<'a> {
    env: &'a Env,
}

impl<'a> EventBuilder<'a> {
    pub fn new(env: &'a Env) -> Self {
        Self { env }
    }

    /// Publish a custom event with symbol topic
    #[allow(deprecated)]
    pub fn publish<
        T: soroban_sdk::IntoVal<Env, soroban_sdk::Val>,
        D: soroban_sdk::IntoVal<Env, soroban_sdk::Val>,
    >(
        &self,
        topic: &str,
        sub_topic: T,
        data: D,
    ) {
        let topics = (Symbol::new(self.env, topic), sub_topic);
        self.env.events().publish(topics, data);
    }
}
