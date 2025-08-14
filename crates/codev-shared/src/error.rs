//! Error types for CoDev.rs

use thiserror::Error;
use crate::types::ProviderId;

/// Main error type for CoDev.rs operations
#[derive(Debug, Error)]
pub enum CodevError {
    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("LLM provider error: {provider} - {message}")]
    LlmProvider { provider: ProviderId, message: String },

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Git error: {message}")]
    Git { message: String },

    #[error("Project analysis error: {message}")]
    Analysis { message: String },

    #[error("Security violation: {message}")]
    Security { message: String },

    #[error("Command execution failed: {command} {error}")]
    CommandExecution { command: String, error: String },

    #[error("Timeout occured: {operation}")]
    Timeout { operation: String },

    #[error("Authentication failed for provider: {provider}")]
    Authentication { provider: ProviderId },

    #[error("Rate limit exceeded for provider: {provider}")]
    RateLimit { provider: ProviderId },

    #[error("Invalid input: {message}")]
    InvalidInput { message: String },

    #[error("Not found: {resource}")]
    NotFound { resource: String },

    #[error("Internal error: {message}")]
    Internal { message: String },
}

/// Result type alias for CoDev operations
pub type Result<T> = std::result::Result<T, CodevError>;

/// LLM-specific errors
#[derive(Debug, Error)]
pub enum LlmError {
    #[error("Provider not available: {provider}")]
    ProviderNotAvailable { provider: ProviderId },

    #[error("No provider available for request")]
    NoProviderAvailable,

    #[error("Provider not found: {provider}")]
    ProviderNotFound { provider: String },

    #[error("API key missing for provider: {provider}")]
    ApiKeyMissing { provider: ProviderId },

    #[error("Invalid API key for provider: {provider}")]
    InvalidApiKey { provider: ProviderId },

    #[error("Model not found: {model}")]
    ModelNotFound { model: String },

    #[error("Context length exceeded: {tokens} > {max_tokens}")]
    ContextLengthExceeded { tokens: usize, max_tokens: usize },

    #[error("Streaming error: {message}")]
    Streaming { message: String },

    #[error("Response parsing error: {message}")]
    ResponseParsing { message: String },

    #[error("Network timeout: {provider}")]
    NetworkTimeout { provider: ProviderId },

    #[error("Server error from {provider}: {status_code} - {message}")]
    ServerError {
        provider: ProviderId,
        status_code: u16,
        message: String,
    },
}

/// Security-related errors
#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("Command not allowed: {command}")]
    CommandNotAllowed { command: String },

    #[error("File access denied: {path}")]
    FileAccessDenied { path: String },

    #[error("Network access denied")]
    NetworkAccessDenied,

    #[error("Execution timeout")]
    ExecutionTimeout,

    #[error("Memory limit exceeded")]
    MemoryLimitExceeded,

    #[error("Sandbox escape attempt detected")]
    SandboxEscape,

    #[error("Privilege escalation attempt detected")]
    PrivilegeEscalation,

    #[error("Malicious code detected: {reason}")]
    MaliciousCode { reason: String },
}

/// Configuration-related errors
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Configuration file not found: {path}")]
    FileNotFound { path: String },

    #[error("Invalid configuration format: {message}")]
    InvalidFormat { message: String },

    #[error("Missing required configuration: {key}")]
    MissingRequired { key: String },

    #[error("Invalid configuration value for {key}: {value}")]
    InvalidValue { key: String, value: String },

    #[error("Environment variable not found: {var}")]
    EnvVarNotFound { var: String },

    #[error("Permission denied accessing config: {path}")]
    PermissionDenied { path: String },
}

/// Project analysis errors
#[derive(Debug, Error)]
pub enum AnalysisError {
    #[error("Language detection failed for file: {path}")]
    LanguageDetection { path: String },

    #[error("Parse error in {file} at line {line}: {message}")]
    ParseError {
        file: String,
        line: usize,
        message: String,
    },

    #[error("Dependency resolution failed: {dependency}")]
    DependencyResolution { dependency: String },

    #[error("Build system not detected")]
    BuildSystemNotDetected,

    #[error("Project structure invalid: {reason}")]
    InvalidProjectStructure { reason: String },

    #[error("Git repository error: {message}")]
    GitRepository { message: String },
}

/// Conversion implementations for easier error handling
impl From<LlmError> for CodevError {
    fn from(error: LlmError) -> Self {
        match error {
            LlmError::ProviderNotAvailable { provider } => {
                CodevError::LlmProvider {
                    provider,
                    message: "Provider not available".to_string(),
                }
            }
            LlmError::NoProviderAvailable => {
                CodevError::LlmProvider {
                    provider: ProviderId::Ollama, // Default fallback
                    message: "No provider available".to_string(),
                }
            }
            _ => CodevError::LlmProvider {
                provider: ProviderId::Ollama,
                message: error.to_string(),
            }
        }
    }
}

impl From<SecurityError> for CodevError {
    fn from(error: SecurityError) -> Self {
        CodevError::Security {
            message: error.to_string(),
        }
    }
}

impl From<ConfigError> for CodevError {
    fn from(error: ConfigError) -> Self {
        CodevError::Config {
            message: error.to_string()
        }
    }
}

impl From<AnalysisError> for CodevError {
    fn from(error: AnalysisError) -> Self {
        CodevError::Analysis {
            message: error.to_string(),
        }
    }
}