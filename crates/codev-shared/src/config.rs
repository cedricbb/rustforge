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

/// Configuration for a specific AI provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Whether this provider is enabled
    pub enabled: bool,

    /// Model to use for this provider
    pub model: String,

    /// Maximum tokens for requests
    pub max_tokens: Option<usize>,

    /// Temperature settings (0.0 - 2.0)
    pub temperature: Option<f32>,

    /// Custom endpoint (for self-hosted models)
    pub endpoint: Option<String>,

    /// Timeout for requests in seconds
    pub timeout_seconds: Option<u64>,

    /// Maximum retries on failure
    pub max_retries: Option<u32>,
}

/// Ollama-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// Ollama API endpoint
    pub endpoint: String,

    /// Whether to auto-install missing models
    pub auto_install_models: bool,

    /// Preferred models for different tasks
    pub models: OllamaModels,

    /// Maximum context length
    pub max_content_length: usize,

    /// Keep alive duration in seconds
    pub keep_alive: Option<u64>,
}

/// Ollama model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModels {
    /// Model for code generation
    pub code_generation: String,

    /// Model for chat conversations
    pub chat: String,

    /// Model for code analysis
    pub analysis: String,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Default security level
    pub default_level: SecurityLevel,

    /// Whether sandbox is enabled
    pub sandbox: SandboxConfig,

    /// Allowed commands whitelist
    pub allowed_commands: Vec<String>,

    /// File access restrictions
    pub file_access: FileAccessConfig,
}