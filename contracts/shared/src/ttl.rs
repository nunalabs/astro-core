//! # Lazy TTL Refresh Pattern
//!
//! Provides efficient TTL (Time To Live) management by only refreshing when necessary.
//!
//! ## Problem
//!
//! Calling `extend_ttl` on every storage access is expensive because:
//! 1. TTL extension is a storage write operation
//! 2. If TTL is still far from expiration, the extension is wasteful
//!
//! ## Solution
//!
//! Only extend TTL when it's within a threshold of expiration.
//! This reduces unnecessary writes while ensuring data doesn't expire.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use astro_core_shared::ttl;
//!
//! // In your contract functions:
//! pub fn some_operation(env: Env) {
//!     ttl::maybe_extend_instance_ttl(&env);
//!     // ... rest of function
//! }
//! ```

use soroban_sdk::{contracttype, Env};

/// Storage key for last refresh timestamp
#[contracttype]
#[derive(Clone)]
pub enum LazyTtlKey {
    /// Last instance storage refresh sequence number
    LastInstanceRefresh,
    /// Last persistent storage refresh sequence number
    LastPersistentRefresh,
}

/// Refresh threshold as a percentage of max TTL
/// When TTL remaining is less than this fraction of max, we refresh
/// Default: 10% (meaning refresh when 90% of TTL has elapsed)
pub const REFRESH_THRESHOLD_PERCENT: u64 = 10;

/// Minimum ledgers between refreshes to prevent excessive refreshes
pub const MIN_REFRESH_INTERVAL: u32 = 1000;

/// Buffer to subtract from max_ttl for threshold calculations
pub const TTL_BUFFER: u32 = 1000;

/// Conditionally extend instance storage TTL
///
/// Only extends if the storage is approaching expiration.
/// This is much more efficient than extending on every access.
///
/// # How it works
///
/// 1. Check the current ledger sequence
/// 2. Compare with the last refresh ledger (stored in instance)
/// 3. Only refresh if enough ledgers have passed
///
/// # Example
///
/// ```rust,ignore
/// pub fn deposit(env: Env, amount: i128) -> Result<(), Error> {
///     ttl::maybe_extend_instance_ttl(&env);
///     // ... deposit logic
///     Ok(())
/// }
/// ```
pub fn maybe_extend_instance_ttl(env: &Env) {
    let current_sequence = env.ledger().sequence();
    let max_ttl = env.storage().max_ttl();

    // Get last refresh sequence (default to 0 for first call)
    let last_refresh: u32 = env
        .storage()
        .instance()
        .get(&LazyTtlKey::LastInstanceRefresh)
        .unwrap_or(0);

    // Calculate threshold: refresh when we've used up (100 - THRESHOLD)% of the TTL
    let refresh_interval = max_ttl * (100 - REFRESH_THRESHOLD_PERCENT) as u32 / 100;

    // Only refresh if enough time has passed since last refresh
    if current_sequence.saturating_sub(last_refresh)
        >= refresh_interval.min(MIN_REFRESH_INTERVAL)
    {
        // Extend TTL
        env.storage()
            .instance()
            .extend_ttl(max_ttl.saturating_sub(TTL_BUFFER), max_ttl);

        // Update last refresh timestamp
        env.storage()
            .instance()
            .set(&LazyTtlKey::LastInstanceRefresh, &current_sequence);
    }
}

/// Conditionally extend persistent storage TTL for a specific key
///
/// Similar to `maybe_extend_instance_ttl` but for persistent storage.
/// Uses a simpler approach since persistent storage TTL extension
/// is called less frequently by design.
///
/// # Example
///
/// ```rust,ignore
/// pub fn get_balance(env: Env, user: Address) -> i128 {
///     let key = DataKey::Balance(user);
///     ttl::maybe_extend_persistent_ttl(&env, &key);
///     // ... read balance
/// }
/// ```
pub fn maybe_extend_persistent_ttl<K>(env: &Env, key: &K)
where
    K: soroban_sdk::TryFromVal<Env, soroban_sdk::Val>
        + soroban_sdk::IntoVal<Env, soroban_sdk::Val>,
{
    let max_ttl = env.storage().max_ttl();

    // For persistent storage, we use a simpler approach:
    // Just extend with a high threshold to ensure we don't miss expiration
    // Persistent storage TTL extension is called less frequently by design
    env.storage()
        .persistent()
        .extend_ttl(key, max_ttl.saturating_sub(TTL_BUFFER), max_ttl);
}

/// Force extend instance TTL
///
/// Use this when you know you want to extend regardless of last refresh.
/// Useful for initialization and admin operations.
///
/// # Example
///
/// ```rust,ignore
/// pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
///     // ... initialization logic
///     ttl::force_extend_instance_ttl(&env);
///     Ok(())
/// }
/// ```
pub fn force_extend_instance_ttl(env: &Env) {
    let max_ttl = env.storage().max_ttl();
    let current_sequence = env.ledger().sequence();

    env.storage()
        .instance()
        .extend_ttl(max_ttl.saturating_sub(TTL_BUFFER), max_ttl);

    env.storage()
        .instance()
        .set(&LazyTtlKey::LastInstanceRefresh, &current_sequence);
}

/// Check if instance TTL should be refreshed
///
/// Returns true if TTL is approaching expiration.
/// Useful for conditional logic in view functions.
///
/// # Example
///
/// ```rust,ignore
/// pub fn get_info(env: Env) -> Info {
///     if ttl::should_refresh_instance_ttl(&env) {
///         // Log or alert that TTL is low
///     }
///     // ... return info
/// }
/// ```
pub fn should_refresh_instance_ttl(env: &Env) -> bool {
    let current_sequence = env.ledger().sequence();
    let max_ttl = env.storage().max_ttl();

    let last_refresh: u32 = env
        .storage()
        .instance()
        .get(&LazyTtlKey::LastInstanceRefresh)
        .unwrap_or(0);

    let refresh_interval = max_ttl * (100 - REFRESH_THRESHOLD_PERCENT) as u32 / 100;

    current_sequence.saturating_sub(last_refresh)
        >= refresh_interval.min(MIN_REFRESH_INTERVAL)
}

// Note: Tests require contract context and are covered in integration tests
