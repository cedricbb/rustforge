//! # CoDev Shared
//!
//! Shared types, traits, and utilities for the CoDev.
//! This crate provides the foundation typs used across all CoDev components.

pub mod config;
pub mod error;
pub mod types;

// Re-export commonly used types
pub use config::*;
pub use error::*;
pub use types::*;

/// Version information for CoDev.rs
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GIT_HASH: &str = env!("VERGEN_GIT_SHA_SHORT");
pub const BUILD_DATE: &str = env!("VERGEN_BUILD_DATE");
