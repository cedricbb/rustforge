//! Configuration management for CoDev.rs

use crate::error::{ConfigError, Result};
use crate::types::{Environment, ProviderId, SecurityLevel};
use serde::{ Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{ Path, PathBuf};

/// Main configuration structure for CoDev.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodevConfig {
    /// Application environment
    pub environment: Environment,

    /// AI provider configuration
    pub ai: AiConfig,

    /// Security settings
    pub security: SecurityConfig,

    /// Development settings
    pub development: Option<DevelopmentConfig>,

    /// Logging configuration
    pub logging: LoggingConfig,

    /// Workspace settings
    pub workspace: WorkspaceConfig,
}

/// AI provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// Default provider to use
    pub default_provider: ProviderId,

    /// Fallback chan when providers fail
    pub fallback_chain: Vec<ProviderId>,

    /// Auto-detect environment for provider selection
    pub auto_detect_environment: bool,

    /// Provider-specific configurations
    pub providers: HashMap<ProviderId, ProviderConfig>,

    /// Environment-specific provider preferences
    pub environment_providers: Option<HashMap<String, Vec<ProviderId>>>,
}