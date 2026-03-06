//! # Astro Core Shared Library
//!
//! Shared types, interfaces, math utilities, and events for the Astro ecosystem.
//!
//! ## Modules
//! - `types` - Common data structures and enums
//! - `math` - Safe arithmetic operations
//! - `interfaces` - Cross-contract call interfaces
//! - `events` - Standard event definitions
//! - `reentrancy` - RAII-based reentrancy protection
//! - `ttl` - Lazy TTL refresh pattern for storage efficiency
//!
//! ## Usage
//! ```rust,ignore
//! use astro_core_shared::{TokenMetadata, safe_add, FeeConfig};
//! use astro_core_shared::reentrancy::ReentrancyGuard;
//! use astro_core_shared::ttl;
//! ```

#![no_std]

pub mod events;
pub mod interfaces;
pub mod math;
pub mod reentrancy;
pub mod ttl;
pub mod types;

// Re-export commonly used items
pub use events::*;
pub use interfaces::*;
pub use math::*;
pub use types::*;
