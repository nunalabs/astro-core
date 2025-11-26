#![no_std]

//! # Treasury Vault Contract
//!
//! Multi-token Treasury Vault for Astro Shiba Protocol.
//!
//! ## Features
//! - Receives protocol fees from any SAC token WITHOUT requiring trustlines
//! - Admin-controlled withdrawals
//! - Multi-token balance tracking
//! - Event emission for transparency and auditing
//! - Integration with Fee Distributor
//! - Pause functionality for emergencies
//!
//! ## Why Contract instead of Classic Account?
//! - Contract addresses (C...) don't need trustlines - they store balances in ContractData
//! - 128-bit balance support vs 64-bit for classic accounts
//! - Programmable access control
//! - Governance-ready

use astro_core_shared::{
    events::{emit_admin_changed, emit_deposit, emit_paused, emit_withdraw, EventBuilder},
    types::{extend_instance_ttl, RateLimitConfig, SharedError, TreasuryConfig, WithdrawalTracker},
};
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, Vec};

// ════════════════════════════════════════════════════════════════════════════
// Constants
// ════════════════════════════════════════════════════════════════════════════

/// Seconds in a day for rate limit reset
const SECONDS_PER_DAY: u64 = 86400;

// ════════════════════════════════════════════════════════════════════════════
// Storage Keys
// ════════════════════════════════════════════════════════════════════════════

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// Admin address that can withdraw funds
    Admin,
    /// List of token addresses that have been received
    TokenList,
    /// Whether the contract is initialized
    Initialized,
    /// Whether the contract is paused
    Paused,
    /// Fee distributor address (authorized to send funds)
    FeeDistributor,
    /// Allowed spenders (addresses that can withdraw on behalf of treasury)
    AllowedSpenders,
    /// Treasury configuration (rate limits, token limits, etc.)
    Config,
    /// Withdrawal tracker per token (Address -> WithdrawalTracker)
    WithdrawalTracker(Address),
}

// ════════════════════════════════════════════════════════════════════════════
// Contract Implementation
// ════════════════════════════════════════════════════════════════════════════

#[contract]
pub struct TreasuryVault;

#[contractimpl]
impl TreasuryVault {
    // ────────────────────────────────────────────────────────────────────────
    // Initialization
    // ────────────────────────────────────────────────────────────────────────

    /// Initialize the treasury vault with an admin address.
    ///
    /// # Arguments
    /// * `admin` - Address that will have withdrawal permissions
    pub fn initialize(env: Env, admin: Address) -> Result<(), SharedError> {
        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(SharedError::AlreadyInitialized);
        }

        // Store admin
        env.storage().instance().set(&DataKey::Admin, &admin);

        // Initialize empty token list
        let empty_list: Vec<Address> = Vec::new(&env);
        env.storage()
            .instance()
            .set(&DataKey::TokenList, &empty_list);
        env.storage()
            .instance()
            .set(&DataKey::AllowedSpenders, &empty_list);

        // Initialize default config with limits
        let default_config = TreasuryConfig {
            rate_limit: RateLimitConfig {
                max_per_tx: 0,
                daily_limit: 0,
                cooldown_seconds: 0,
                enabled: false,
            },
            max_tokens: TreasuryConfig::DEFAULT_MAX_TOKENS,
            max_spenders: TreasuryConfig::DEFAULT_MAX_SPENDERS,
        };
        env.storage()
            .instance()
            .set(&DataKey::Config, &default_config);

        // Initialize state
        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().set(&DataKey::Paused, &false);

        extend_instance_ttl(&env);

        let events = EventBuilder::new(&env);
        events.publish(
            "treasury",
            "initialized",
            (admin.clone(), env.ledger().timestamp()),
        );

        Ok(())
    }

    // ────────────────────────────────────────────────────────────────────────
    // Deposits
    // ────────────────────────────────────────────────────────────────────────

    /// Receive notification of a deposit.
    /// Note: SAC tokens automatically credit the contract's balance.
    /// This function is for tracking which tokens we've received.
    ///
    /// # Arguments
    /// * `token` - Address of the SAC token
    /// * `from` - Address that sent the tokens
    /// * `amount` - Amount deposited
    pub fn notify_deposit(
        env: Env,
        token: Address,
        from: Address,
        amount: i128,
    ) -> Result<(), SharedError> {
        Self::require_initialized(&env)?;

        if amount <= 0 {
            return Err(SharedError::InvalidAmount);
        }

        // Track this token if not already tracked
        Self::track_token(&env, &token);

        emit_deposit(&env, &token, &from, amount);
        extend_instance_ttl(&env);

        Ok(())
    }

    /// Direct deposit - transfers tokens and notifies in one call
    pub fn deposit(
        env: Env,
        from: Address,
        token: Address,
        amount: i128,
    ) -> Result<(), SharedError> {
        from.require_auth();
        Self::require_initialized(&env)?;
        Self::require_not_paused(&env)?;

        if amount <= 0 {
            return Err(SharedError::InvalidAmount);
        }

        // Transfer tokens to treasury
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&from, &env.current_contract_address(), &amount);

        // Track token
        Self::track_token(&env, &token);

        emit_deposit(&env, &token, &from, amount);
        extend_instance_ttl(&env);

        Ok(())
    }

    // ────────────────────────────────────────────────────────────────────────
    // Withdrawals (admin only)
    // ────────────────────────────────────────────────────────────────────────

    /// Withdraw tokens to a specified address.
    /// Only callable by admin.
    ///
    /// # Arguments
    /// * `token` - SAC token address to withdraw
    /// * `to` - Destination address
    /// * `amount` - Amount to withdraw
    pub fn withdraw(
        env: Env,
        token: Address,
        to: Address,
        amount: i128,
    ) -> Result<(), SharedError> {
        Self::require_initialized(&env)?;
        Self::require_not_paused(&env)?;
        Self::require_admin(&env)?;

        if amount <= 0 {
            return Err(SharedError::InvalidAmount);
        }

        // Check rate limits
        Self::check_and_update_rate_limit(&env, &token, amount)?;

        // Check balance
        let balance = Self::get_balance(&env, &token);
        if balance < amount {
            return Err(SharedError::InsufficientBalance);
        }

        // Transfer tokens
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &to, &amount);

        emit_withdraw(&env, &token, &to, amount);
        extend_instance_ttl(&env);

        Ok(())
    }

    /// Withdraw all tokens of a specific type.
    /// Only callable by admin.
    ///
    /// # Arguments
    /// * `token` - SAC token address to withdraw
    /// * `to` - Destination address
    pub fn withdraw_all(env: Env, token: Address, to: Address) -> Result<i128, SharedError> {
        Self::require_initialized(&env)?;
        Self::require_not_paused(&env)?;
        Self::require_admin(&env)?;

        // Get full balance
        let balance = Self::get_balance(&env, &token);
        if balance <= 0 {
            return Err(SharedError::InsufficientBalance);
        }

        // Check rate limits (withdraw_all respects limits)
        Self::check_and_update_rate_limit(&env, &token, balance)?;

        // Transfer all
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &to, &balance);

        emit_withdraw(&env, &token, &to, balance);
        extend_instance_ttl(&env);

        Ok(balance)
    }

    /// Spend tokens on behalf of treasury (for allowed spenders)
    pub fn spend(
        env: Env,
        spender: Address,
        token: Address,
        to: Address,
        amount: i128,
    ) -> Result<(), SharedError> {
        spender.require_auth();
        Self::require_initialized(&env)?;
        Self::require_not_paused(&env)?;

        // Check if spender is allowed
        if !Self::is_allowed_spender(&env, &spender) {
            return Err(SharedError::Unauthorized);
        }

        if amount <= 0 {
            return Err(SharedError::InvalidAmount);
        }

        // Check rate limits for spender withdrawals too
        Self::check_and_update_rate_limit(&env, &token, amount)?;

        let balance = Self::get_balance(&env, &token);
        if balance < amount {
            return Err(SharedError::InsufficientBalance);
        }

        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &to, &amount);

        let events = EventBuilder::new(&env);
        events.publish(
            "treasury",
            "spent",
            (spender, token, to, amount, env.ledger().timestamp()),
        );

        extend_instance_ttl(&env);

        Ok(())
    }

    // ────────────────────────────────────────────────────────────────────────
    // Admin Management
    // ────────────────────────────────────────────────────────────────────────

    /// Change the admin address.
    /// Only callable by current admin.
    ///
    /// # Arguments
    /// * `new_admin` - New admin address
    pub fn set_admin(env: Env, new_admin: Address) -> Result<(), SharedError> {
        Self::require_initialized(&env)?;
        Self::require_admin(&env)?;

        let old_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(SharedError::NotInitialized)?;

        env.storage().instance().set(&DataKey::Admin, &new_admin);

        emit_admin_changed(&env, &old_admin, &new_admin);
        extend_instance_ttl(&env);

        Ok(())
    }

    /// Set fee distributor address
    pub fn set_fee_distributor(env: Env, fee_distributor: Address) -> Result<(), SharedError> {
        Self::require_initialized(&env)?;
        Self::require_admin(&env)?;

        env.storage()
            .instance()
            .set(&DataKey::FeeDistributor, &fee_distributor);
        extend_instance_ttl(&env);

        Ok(())
    }

    /// Add an allowed spender
    pub fn add_spender(env: Env, spender: Address) -> Result<(), SharedError> {
        Self::require_initialized(&env)?;
        Self::require_admin(&env)?;

        let mut spenders: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::AllowedSpenders)
            .unwrap_or(Vec::new(&env));

        // Check if already exists
        for s in spenders.iter() {
            if s == spender {
                return Ok(());
            }
        }

        // Check max spenders limit
        let config = Self::get_config_internal(&env);
        if spenders.len() >= config.max_spenders {
            return Err(SharedError::LimitExceeded);
        }

        spenders.push_back(spender.clone());
        env.storage()
            .instance()
            .set(&DataKey::AllowedSpenders, &spenders);

        let events = EventBuilder::new(&env);
        events.publish(
            "treasury",
            "spender_added",
            (spender, env.ledger().timestamp()),
        );

        extend_instance_ttl(&env);

        Ok(())
    }

    /// Remove an allowed spender
    pub fn remove_spender(env: Env, spender: Address) -> Result<(), SharedError> {
        Self::require_initialized(&env)?;
        Self::require_admin(&env)?;

        let spenders: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::AllowedSpenders)
            .unwrap_or(Vec::new(&env));

        let mut new_spenders = Vec::new(&env);
        for s in spenders.iter() {
            if s != spender {
                new_spenders.push_back(s);
            }
        }

        env.storage()
            .instance()
            .set(&DataKey::AllowedSpenders, &new_spenders);

        let events = EventBuilder::new(&env);
        events.publish(
            "treasury",
            "spender_removed",
            (spender, env.ledger().timestamp()),
        );

        extend_instance_ttl(&env);

        Ok(())
    }

    /// Pause/unpause the contract
    pub fn set_paused(env: Env, paused: bool) -> Result<(), SharedError> {
        Self::require_initialized(&env)?;
        Self::require_admin(&env)?;

        env.storage().instance().set(&DataKey::Paused, &paused);

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(SharedError::NotInitialized)?;

        emit_paused(&env, paused, &admin);
        extend_instance_ttl(&env);

        Ok(())
    }

    /// Update treasury configuration (rate limits, max tokens/spenders)
    pub fn update_config(env: Env, new_config: TreasuryConfig) -> Result<(), SharedError> {
        Self::require_initialized(&env)?;
        Self::require_admin(&env)?;

        env.storage().instance().set(&DataKey::Config, &new_config);

        let events = EventBuilder::new(&env);
        events.publish(
            "treasury",
            "config_updated",
            (
                new_config.rate_limit.enabled,
                new_config.rate_limit.daily_limit,
                new_config.max_tokens,
                env.ledger().timestamp(),
            ),
        );

        extend_instance_ttl(&env);

        Ok(())
    }

    // ────────────────────────────────────────────────────────────────────────
    // View Functions
    // ────────────────────────────────────────────────────────────────────────

    /// Get the current admin address.
    pub fn get_admin(env: Env) -> Result<Address, SharedError> {
        Self::require_initialized(&env)?;
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(SharedError::NotInitialized)
    }

    /// Get the balance of a specific token.
    pub fn balance(env: Env, token: Address) -> i128 {
        Self::get_balance(&env, &token)
    }

    /// Get list of all tokens that have been deposited.
    pub fn get_tokens(env: Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::TokenList)
            .unwrap_or(Vec::new(&env))
    }

    /// Get all allowed spenders
    pub fn get_spenders(env: Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::AllowedSpenders)
            .unwrap_or(Vec::new(&env))
    }

    /// Check if the contract is initialized.
    pub fn is_initialized(env: Env) -> bool {
        env.storage().instance().has(&DataKey::Initialized)
    }

    /// Check if the contract is paused.
    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
    }

    /// Get fee distributor address
    pub fn fee_distributor(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::FeeDistributor)
    }

    /// Get treasury configuration
    pub fn get_config(env: Env) -> TreasuryConfig {
        Self::get_config_internal(&env)
    }

    // ────────────────────────────────────────────────────────────────────────
    // Internal Functions
    // ────────────────────────────────────────────────────────────────────────

    fn require_initialized(env: &Env) -> Result<(), SharedError> {
        if !env.storage().instance().has(&DataKey::Initialized) {
            return Err(SharedError::NotInitialized);
        }
        Ok(())
    }

    fn require_not_paused(env: &Env) -> Result<(), SharedError> {
        let paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        if paused {
            return Err(SharedError::ContractPaused);
        }
        Ok(())
    }

    fn require_admin(env: &Env) -> Result<(), SharedError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(SharedError::NotInitialized)?;
        admin.require_auth();
        Ok(())
    }

    fn is_allowed_spender(env: &Env, spender: &Address) -> bool {
        // Admin is always allowed
        if let Some(admin) = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::Admin)
        {
            if admin == *spender {
                return true;
            }
        }

        // Check allowed spenders list
        let spenders: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::AllowedSpenders)
            .unwrap_or(Vec::new(env));

        for s in spenders.iter() {
            if s == *spender {
                return true;
            }
        }

        false
    }

    /// Get token balance from SAC
    fn get_balance(env: &Env, token: &Address) -> i128 {
        let token_client = token::Client::new(env, token);
        token_client.balance(&env.current_contract_address())
    }

    /// Track a token if not already tracked
    fn track_token(env: &Env, token: &Address) {
        let mut tokens: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::TokenList)
            .unwrap_or(Vec::new(env));

        // Check if already tracked
        for t in tokens.iter() {
            if t == *token {
                return;
            }
        }

        // Check max tokens limit (silently ignore if limit reached - don't fail deposits)
        let config = Self::get_config_internal(env);
        if tokens.len() >= config.max_tokens {
            // Log warning but don't fail - token still works, just not tracked
            return;
        }

        tokens.push_back(token.clone());
        env.storage().instance().set(&DataKey::TokenList, &tokens);
    }

    /// Get treasury configuration
    fn get_config_internal(env: &Env) -> TreasuryConfig {
        env.storage()
            .instance()
            .get(&DataKey::Config)
            .unwrap_or(TreasuryConfig {
                rate_limit: RateLimitConfig {
                    max_per_tx: 0,
                    daily_limit: 0,
                    cooldown_seconds: 0,
                    enabled: false,
                },
                max_tokens: TreasuryConfig::DEFAULT_MAX_TOKENS,
                max_spenders: TreasuryConfig::DEFAULT_MAX_SPENDERS,
            })
    }

    /// Check and update rate limits for withdrawals
    fn check_and_update_rate_limit(
        env: &Env,
        token: &Address,
        amount: i128,
    ) -> Result<(), SharedError> {
        let config = Self::get_config_internal(env);

        // Skip if rate limiting is disabled
        if !config.rate_limit.enabled {
            return Ok(());
        }

        let current_time = env.ledger().timestamp();

        // Check per-transaction limit
        if config.rate_limit.max_per_tx > 0 && amount > config.rate_limit.max_per_tx {
            return Err(SharedError::TransactionLimitExceeded);
        }

        // Get or create withdrawal tracker
        let mut tracker: WithdrawalTracker = env
            .storage()
            .persistent()
            .get(&DataKey::WithdrawalTracker(token.clone()))
            .unwrap_or(WithdrawalTracker {
                amount_withdrawn: 0,
                period_start: current_time,
                last_withdrawal: 0,
            });

        // Reset daily limit if new day
        if current_time >= tracker.period_start + SECONDS_PER_DAY {
            tracker.amount_withdrawn = 0;
            tracker.period_start = current_time;
        }

        // Check cooldown
        if config.rate_limit.cooldown_seconds > 0 {
            let time_since_last = current_time.saturating_sub(tracker.last_withdrawal);
            if time_since_last < config.rate_limit.cooldown_seconds && tracker.last_withdrawal > 0 {
                return Err(SharedError::CooldownNotElapsed);
            }
        }

        // Check daily limit
        if config.rate_limit.daily_limit > 0 {
            let new_total = tracker.amount_withdrawn + amount;
            if new_total > config.rate_limit.daily_limit {
                return Err(SharedError::DailyLimitExceeded);
            }
            tracker.amount_withdrawn = new_total;
        }

        // Update tracker
        tracker.last_withdrawal = current_time;
        env.storage()
            .persistent()
            .set(&DataKey::WithdrawalTracker(token.clone()), &tracker);

        Ok(())
    }
}

// ════════════════════════════════════════════════════════════════════════════
// Tests
// ════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn create_token_contract<'a>(
        env: &Env,
        admin: &Address,
    ) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
        let contract_address = env.register_stellar_asset_contract_v2(admin.clone());
        (
            token::Client::new(env, &contract_address.address()),
            token::StellarAssetClient::new(env, &contract_address.address()),
        )
    }

    #[test]
    fn test_initialize() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let treasury_id = env.register(TreasuryVault, ());
        let treasury = TreasuryVaultClient::new(&env, &treasury_id);

        treasury.initialize(&admin);

        assert!(treasury.is_initialized());
        assert_eq!(treasury.get_admin(), admin);
        assert!(!treasury.is_paused());
    }

    #[test]
    fn test_double_initialize_fails() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let treasury_id = env.register(TreasuryVault, ());
        let treasury = TreasuryVaultClient::new(&env, &treasury_id);

        treasury.initialize(&admin);
        let result = treasury.try_initialize(&admin);
        assert!(result.is_err());
    }

    #[test]
    fn test_deposit_and_withdraw() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);

        // Setup treasury
        let treasury_id = env.register(TreasuryVault, ());
        let treasury = TreasuryVaultClient::new(&env, &treasury_id);
        treasury.initialize(&admin);

        // Setup token
        let (token_client, token_admin) = create_token_contract(&env, &admin);

        // Mint tokens to user
        token_admin.mint(&user, &1000);

        // User deposits to treasury
        treasury.deposit(&user, &token_client.address, &500);

        // Check balance
        assert_eq!(treasury.balance(&token_client.address), 500);

        // Admin withdraws
        treasury.withdraw(&token_client.address, &admin, &200);
        assert_eq!(treasury.balance(&token_client.address), 300);

        // Withdraw all remaining
        let withdrawn = treasury.withdraw_all(&token_client.address, &admin);
        assert_eq!(withdrawn, 300);
        assert_eq!(treasury.balance(&token_client.address), 0);
    }

    #[test]
    fn test_multiple_tokens() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);

        // Setup treasury
        let treasury_id = env.register(TreasuryVault, ());
        let treasury = TreasuryVaultClient::new(&env, &treasury_id);
        treasury.initialize(&admin);

        // Setup multiple tokens
        let (token1_client, token1_admin) = create_token_contract(&env, &admin);
        let (token2_client, token2_admin) = create_token_contract(&env, &admin);

        // Mint and deposit both tokens
        token1_admin.mint(&user, &1000);
        token2_admin.mint(&user, &2000);

        treasury.deposit(&user, &token1_client.address, &500);
        treasury.deposit(&user, &token2_client.address, &1000);

        // Check balances
        assert_eq!(treasury.balance(&token1_client.address), 500);
        assert_eq!(treasury.balance(&token2_client.address), 1000);

        // Check token list
        let tokens = treasury.get_tokens();
        assert_eq!(tokens.len(), 2);
    }

    #[test]
    fn test_spender_system() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let spender = Address::generate(&env);
        let recipient = Address::generate(&env);

        let treasury_id = env.register(TreasuryVault, ());
        let treasury = TreasuryVaultClient::new(&env, &treasury_id);
        treasury.initialize(&admin);

        let (token_client, token_admin) = create_token_contract(&env, &admin);
        token_admin.mint(&admin, &1000);

        // Admin deposits
        treasury.deposit(&admin, &token_client.address, &1000);

        // Add spender
        treasury.add_spender(&spender);

        // Spender can spend
        treasury.spend(&spender, &token_client.address, &recipient, &500);
        assert_eq!(treasury.balance(&token_client.address), 500);
        assert_eq!(token_client.balance(&recipient), 500);

        // Remove spender
        treasury.remove_spender(&spender);

        // Spender can no longer spend
        let result = treasury.try_spend(&spender, &token_client.address, &recipient, &100);
        assert!(result.is_err());
    }

    #[test]
    fn test_pause() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);

        let treasury_id = env.register(TreasuryVault, ());
        let treasury = TreasuryVaultClient::new(&env, &treasury_id);
        treasury.initialize(&admin);

        let (token_client, token_admin) = create_token_contract(&env, &admin);
        token_admin.mint(&user, &1000);

        // Pause contract
        treasury.set_paused(&true);
        assert!(treasury.is_paused());

        // Deposit should fail when paused
        let result = treasury.try_deposit(&user, &token_client.address, &500);
        assert!(result.is_err());

        // Unpause
        treasury.set_paused(&false);

        // Deposit should work now
        treasury.deposit(&user, &token_client.address, &500);
        assert_eq!(treasury.balance(&token_client.address), 500);
    }

    #[test]
    fn test_change_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);

        let treasury_id = env.register(TreasuryVault, ());
        let treasury = TreasuryVaultClient::new(&env, &treasury_id);
        treasury.initialize(&admin1);

        assert_eq!(treasury.get_admin(), admin1);

        treasury.set_admin(&admin2);
        assert_eq!(treasury.get_admin(), admin2);
    }
}
