//! # Reentrancy Guard Module
//!
//! Provides protection against reentrancy attacks using the RAII pattern.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use astro_core_shared::reentrancy::ReentrancyGuard;
//!
//! pub fn critical_operation(env: Env) -> Result<(), Error> {
//!     // Guard automatically acquires lock and releases on drop
//!     let _guard = ReentrancyGuard::acquire(&env, is_locked, set_locked)?;
//!
//!     // Do critical work here...
//!     // Lock is automatically released when _guard goes out of scope
//!     Ok(())
//! }
//! ```
//!
//! ## Benefits of RAII Pattern
//!
//! 1. **Automatic cleanup**: Lock is always released, even on error
//! 2. **No forgotten releases**: Compiler ensures cleanup via Drop trait
//! 3. **Cleaner code**: No manual acquire/release calls scattered throughout

use soroban_sdk::Env;

use crate::types::SharedError;

/// RAII-based reentrancy guard
///
/// Acquires a lock on creation and automatically releases it when dropped.
/// This ensures the lock is always released, even if the function returns an error.
///
/// # Type Parameters
///
/// * `'a` - Lifetime of the environment reference
/// * `F` - Type of the lock setter function
///
/// # Example
///
/// ```rust,ignore
/// fn swap(env: Env) -> Result<i128, MyError> {
///     let _guard = ReentrancyGuard::acquire(&env, is_locked, set_locked)?;
///     // Critical section - lock is held
///     // ...
///     Ok(amount)
/// } // Lock automatically released here
/// ```
pub struct ReentrancyGuard<'a, F>
where
    F: Fn(&Env, bool),
{
    env: &'a Env,
    set_locked: F,
}

impl<'a, F> ReentrancyGuard<'a, F>
where
    F: Fn(&Env, bool),
{
    /// Acquire the reentrancy lock and return a guard
    ///
    /// # Arguments
    ///
    /// * `env` - Soroban environment
    /// * `is_locked` - Function to check if lock is held: `fn(&Env) -> bool`
    /// * `set_locked` - Function to set lock state: `fn(&Env, bool)`
    ///
    /// # Errors
    ///
    /// Returns `SharedError::Reentrancy` if the lock is already held.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// fn is_locked(env: &Env) -> bool {
    ///     env.storage().instance().get(&DataKey::Locked).unwrap_or(false)
    /// }
    ///
    /// fn set_locked(env: &Env, locked: bool) {
    ///     env.storage().instance().set(&DataKey::Locked, &locked);
    /// }
    ///
    /// let _guard = ReentrancyGuard::acquire(&env, is_locked, set_locked)?;
    /// ```
    pub fn acquire<G>(env: &'a Env, is_locked: G, set_locked: F) -> Result<Self, SharedError>
    where
        G: Fn(&Env) -> bool,
    {
        if is_locked(env) {
            return Err(SharedError::Reentrancy);
        }

        set_locked(env, true);

        Ok(Self { env, set_locked })
    }
}

impl<'a, F> Drop for ReentrancyGuard<'a, F>
where
    F: Fn(&Env, bool),
{
    fn drop(&mut self) {
        (self.set_locked)(self.env, false);
    }
}

/// Simple reentrancy guard using temporary storage
///
/// This is a convenience wrapper that uses temporary storage with a fixed key.
/// Use this when you don't need custom storage management.
///
/// # Example
///
/// ```rust,ignore
/// use astro_core_shared::reentrancy::SimpleReentrancyGuard;
///
/// fn critical_fn(env: Env) -> Result<(), SharedError> {
///     let _guard = SimpleReentrancyGuard::acquire(&env)?;
///     // Do work...
///     Ok(())
/// }
/// ```
pub struct SimpleReentrancyGuard<'a> {
    env: &'a Env,
}

impl<'a> SimpleReentrancyGuard<'a> {
    /// Default storage key for the lock
    const LOCK_KEY: &'static str = "REENTRY_LOCK";

    /// Acquire a simple reentrancy lock using temporary storage
    ///
    /// # Errors
    ///
    /// Returns `SharedError::Reentrancy` if already locked.
    pub fn acquire(env: &'a Env) -> Result<Self, SharedError> {
        let is_locked: bool = env
            .storage()
            .temporary()
            .get(&soroban_sdk::Symbol::new(env, Self::LOCK_KEY))
            .unwrap_or(false);

        if is_locked {
            return Err(SharedError::Reentrancy);
        }

        env.storage()
            .temporary()
            .set(&soroban_sdk::Symbol::new(env, Self::LOCK_KEY), &true);

        Ok(Self { env })
    }
}

impl<'a> Drop for SimpleReentrancyGuard<'a> {
    fn drop(&mut self) {
        self.env
            .storage()
            .temporary()
            .remove(&soroban_sdk::Symbol::new(self.env, Self::LOCK_KEY));
    }
}

// Note: Tests require contract context and are covered in integration tests
