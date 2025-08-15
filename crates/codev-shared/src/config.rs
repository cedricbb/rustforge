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

/// Sandbox security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Maximum memory usage in bytes
    pub max_memory: Option<usize>,

    /// Maximum CPU time in seconds
    pub max_cpu_time: Option<u64>,

    /// Network access allowed
    pub network_access: bool,

    /// Temporary directory for sandbox
    pub temp_dir: Option<PathBuf>,

    /// User ID for sandbox execution
    pub user_id: Option<u32>,

    /// Group ID for sandbox execution
    pub group_id: Option<u32>,
}

/// File access configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAccessConfig {
    /// Paths that are read-only
    pub read_only_path: Vec<PathBuf>,

    /// Paths where writing is allowed
    pub write_allowed_paths: Vec<PathBuf>,

    /// Paths that are completely forbidden
    pub forbidden_paths: Vec<PathBuf>,

    /// Maximum file size in bytes
    pub max_file_size: usize,
}

/// Development-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentConfig {
    /// Enable hot reload
    pub hot_reload: bool,

    /// Debug mode enabled
    pub debug_mode: bool,

    /// Development server port
    pub dev_server_port: Option<u16>,

    /// Mock responses for testing
    pub mock_response: bool,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,

    /// Log format (json, pretty, compact)
    pub format: String,

    /// Log to file
    pub file_enabled: bool,

    /// Log file path
    pub file_path: Option<PathBuf>,

    /// Maximum log file size in bytes
    pub max_file_size: Option<usize>,

    /// Number of log files to keep
    pub max_files: Option<usize>,
}

/// Workspace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Default workspace directory
    pub default_path: PathBuf,

    /// Auto-detect project type
    pub auto_detect_project: bool,

    /// Ignore patterns for file watching
    pub ignore_patterns: Vec<String>,

    /// Maximum project size in bytes
    pub max_project_size: Option<usize>,
}

impl Default for CodevConfig {
    fn default() -> Self {
        Self {
            environment: Environment::Development,
            ai: AiConfig::default(),
            security: SecurityConfig::default(),
            development: Some(DevelopmentConfig::default()),
            logging: LoggingConfig::default(),
            workspace: WorkspaceConfig::default(),
        }
    }
}

impl Default for AiConfig {
    fn default() -> Self {
        let mut providers = HashMap::new();

        // Ollama configuration
        providers.insert(
            ProviderId::Ollama,
            ProviderConfig {
                enabled: true,
                model: "codellama:7b".to_string(),
                max_tokens: Some(4096),
                temperature: Some(0.1),
                endpoint: Some("http://localhost:11434".to_string()),
                timeout_seconds: Some(60),
                max_retries: Some(3),
            }
        );

        // Mistral configuration
        providers.insert(
            ProviderId::Mistral,
            ProviderConfig {
                enabled: false, // Requires API key
                model: "mistral-medium".to_string(),
                max_tokens: Some(4096),
                temperature: Some(0.1),
                endpoint: None,
                timeout_seconds: Some(60),
                max_retries: Some(3),
            }
        );

        Self {
            default_provider: ProviderId::Ollama,
            fallback_chain: vec![
                ProviderId::Ollama,
                ProviderId::Mistral,
                ProviderId::Claude,
                ProviderId::OpenAI,
                ProviderId::Gemini,
            ],
            auto_detect_environment: true,
            providers,
            environment_providers: None,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            default_level: SecurityLevel::Development,
            sandbox_enabled: false,
            sandbox: SandboxConfig::default(),
            allowed_commands: vec![
                "cargo".to_string(),
                "rustc".to_string(),
                "git".to_string(),
                "npm".to_string(),
                "node".to_string(),
                "python".to_string(),
                "python3".to_string(),
                "go".to_string(),
            ],
            file_access: FileAccessConfig::default(),
        }
    }
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            max_memory: Some(1024 * 1024 * 1024), // 1GB
            max_cpu_time: Some(600), // 10 minutes
            network_access: false,
            temp_dir: None,
            user_id: None,
            group_id: None,
        }
    }
}

impl Default for FileAccessConfig {
    fn default() -> Self {
        Self {
            read_only_path: vec![
                PathBuf::from("/usr"),
                PathBuf::from("/bin"),
                PathBuf::from("/sbin"),
                PathBuf::from("/lib"),
            ],
            write_allowed_paths: vec![
                PathBuf::from("/tmp"),
                PathBuf::from("/var/tmp"),
            ],
            max_file_size: 100 * 1024 * 1024, // 100MB
        }
    }
}

impl Default for DevelopmentConfig {
    fn default() -> Self {
        Self {
            hot_reload: true,
            debug_mode: true,
            dev_server_port: Some(8080),
            mock_response: false,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "pretty".to_string(),
            file_enabled: true,
            file_path: Some(PathBuf::from("logs/codev.log")),
            max_file_size: Some(10 * 1024 * 1024), // 10MB
            max_files: Some(5),
        }
    }
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            default_path: PathBuf::from("./workspace"),
            auto_detect_project: true,
            ignore_patterns: vec![
                ".git".to_string(),
                "target".to_string(),
                "node_modules".to_string(),
                "__pycache__".to_string(),
                "*.tmp".to_string(),
                ".DS_Store".to_string(),
            ],
            max_project_size: Some(1024 * 1024 * 1024), // 1GB
        }
    }
}

impl CodevConfig {
    /// Load configuration from file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|_| ConfigError::FileNotFound {
                path: path.as_ref().display().to_string(),
            })?;

        toml::from_str(&content)
            .map_err(|e| ConfigError::InvalidFormat {
                message: e.to_string(),
            })
            .map_err(Into::into)
    }

    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::InvalidFormat {
                message: e.to_string(),
            })?;

        std::fs::write(path, content)
            .map_err(Into::into)
    }

    /// Load configuration with environment variable overrides
    pub fn load_with_env() -> Result<Self> {
        let mut config = Self::default();

        // Override with environment variables
        if let Ok(env) = std::env::var("CODEV_ENV") {
            config.environment = match env.as_str() {
                "development" => Environment::Development,
                "production" => Environment::Production,
                "testing" => Environment::Testing,
                _ => Environment::Development,
            };
        }

        // Override default provider
        if let Ok(provider) = std::env::var("CODEV_AI_PROVIDER") {
            match provider.as_str() {
                "ollama" => config.ai.default_provider = ProviderId::Ollama,
                "openai" => config.ai.default_provider = ProviderId::OpenAI,
                "claude" => config.ai.default_provider = ProviderId::Claude,
                "mistral" => config.ai.default_provider = ProviderId::Mistral,
                "gemini" => config.ai.default_provider = ProviderId::Gemini,
                _ => {}
            }
        }

        // Override Ollama endpoint
        if let Ok(endpoint) = std::env::var("OLLAMA_ENDPOINT") {
            if let Some(ollama_config) = config.ai.providers.get_mut(&ProviderId::Ollama) {
                ollama_config.endpoint = Some(endpoint);
            }
        }

        Ok(config)
    }

    /// Get API keys from environment variables
    pub fn load_api_keys(&self) -> HashMap<ProviderId, String> {
        let mut keys = HashMap::new();

        // Load API keys from environment variables only (never from config files)
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            keys.insert(ProviderId::OpenAI, key);
        }
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            keys.insert(ProviderId::Claude, key);
        }
        if let Ok(key) = std::env::var("MISTRAL_API_KEY") {
            keys.insert(ProviderId::Mistral, key);
        }
        if let Ok(key) = std::env::var("GOOGLE_API_KEY") {
            keys.insert(ProviderId::Gemini, key);
        }

        keys
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate AI configuration
        if self.ai.providers.is_empty() {
            return Err(ConfigError::InvalidValue {
                key: "ai.providers".to_string(),
                value: "empty".to_string(),
            }.into());
        }

        // Validate workspace path
        if !self.workspace.default_path.exists() {
            return Err(ConfigError::InvalidValue {
                key: "workspace.default.path".to_string(),
                value: self.workspace.default_path.display().to_string(),
            }.into());
        }

        // Validate security settings
        if let Some(max_memory) = self.security.sandbox.max_memory {
            if max_memory == 0 {
                return Err(ConfigError::InvalidValue {
                    key: "security.sandbox.max_memory".to_string(),
                    value: "0".to_string(),
                }.into())
            }
        }

        Ok(())
    }
}