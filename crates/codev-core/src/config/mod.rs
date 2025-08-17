//! Configuration Management
//!
//! This module handles all configuration aspects of CoDev.rs including:
//! - Loading and saving configuration files
//! - Environment variable integration
//! - API key management
//! - Runtime configuration updates

use codev_shared::{CodevConfig, ProviderId, Result, CodevError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};

/// Configuration manager for CoDev.rs
pub struct ConfigManager {
    /// Current configuration
    config: Arc<RwLock<CodevConfig>>,

    /// Configuration file path
    config_path: Option<PathBuf>,

    /// API keys (stored separately for security)
    api_keys: Arc<RwLock<HashMap<ProviderId, String>>>,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new(config: CodevConfig) -> Self {
        let api_keys = config.load_api_keys();

        Self {
            config: Arc::new(RwLock::new(config)),
            config_path: None,
            api_keys: Arc::new(RwLock::new(api_keys)),
        }
    }

    /// Create configuration manager from file
    #[instrument]
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        info!("Loading configuration from: {}", path.display());

        let config = if path.exists() {
            CodevConfig::load_from_file(path)?
        } else {
            warn!("Configuration file not found, using defaults");
            CodevConfig::default()
        };

        let api_keys = config.load_api_keys();

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            config_path: Some(path.to_path_buf()),
            api_keys: Arc::new(RwLock::new(api_keys)),
        })
    }

    /// Create with automatic configuration discovery
    #[instrument]
    pub async fn auto_discover() -> Result<Self> {
        info!("Auto-discovering configuration");

        // Try multiple locations in order of preference
        let search_paths = Self::get_config_search_paths();

        for path in search_paths {
            if path.exists() {
                debug!("Found configuration at: {}", path.display());
                return Self::from_file(path).await;
            }
        }

        // No config file found, create default
        info!("No configuration file found, creating default");
        let config = CodevConfig::load_with_env()?;
        let manager = Self::new(config);

        // Try to save default config to first writable location
        if let Some(default_path) = Self::get_default_config_path() {
            if let Some(parent) = default_path.parent() {
                if let Err(e) = tokio::fs::create_dir_all(parent).await {
                    warn!("Failed to create config directory: {}", e);
                } else {
                    match manager.save_to_file(&default_path).await {
                        Ok(_) => {
                            info!("Created default configuration at: {}", default_path.display());
                            return Ok(Self {
                                config_path: Some(default_path),
                                ..manager
                            });
                        }
                        Err(e) => warn!("Failed to save default config: {}", e),
                    }
                }
            }
        }

        Ok(manager)
    }

    /// Get current configuration (read-only)
    pub async fn get_config(&self) -> CodevConfig {
        self.config.read().await.clone()
    }

    /// Update configuration
    #[instrument(skip(self, new_config))]
    pub async fn update_config(&self, new_config: CodevConfig) -> Result<()> {
        info!("Updating configuration");

        // Validate new configuration
        new_config.validate()?;

        // Update API keys
        let new_api_keys = new_config.load_api_keys();
        *self.api_keys.write().await = new_api_keys;

        // Update configuration
        *self.config.write().await = new_config;

        // Save to file if we have a path
        if let Some(ref path) = self.config_path {
            self.save_to_file(path).await?;
        }

        info!("Configuration updated successfully");
        Ok(())
    }

    /// Get API keys
    pub async fn get_api_keys(&self) -> HashMap<ProviderId, String> {
        self.api_keys.read().await.clone()
    }

    /// Update API keys from environment
    #[instrument(skip(self))]
    pub async fn refresh_api_keys(&self) -> Result<()> {
        debug!("Refreshing API keys from environment");

        let config = self.config.read().await;
        let new_api_keys = config.load_api_keys();
        *self.api_keys.write().await = new_api_keys;

        Ok(())
    }

    /// Save current configuration to file
    #[instrument(skip(self))]
    pub async fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        debug!("Saving configuration to: {}", path.display());

        let config = self.config.read().await;

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Save configuration (excluding API keys for security)
        config.save_to_file(path)?;

        info!("Configuration saved to: {}", path.display());
        Ok(())
    }

    /// Get configuration value by key path
    pub async fn get_value(&self, key_path: &str) -> Option<ConfigValue> {
        let config = self.config.read().await;
        Self::extract_value_by_path(&*config, key_path)
    }

    /// Set configuration value by key path
    #[instrument(skip(self, value))]
    pub async fn set_value(&self, key_path: &str, value: ConfigValue) -> Result<()> {
        debug!("Setting config value: {} = {:?}", key_path, value);

        let mut config = self.config.write().await;
        Self::set_value_by_path(&mut *config, key_path, value)?;

        // Save to file if we have a path
        if let Some(ref path) = self.config_path {
            config.save_to_file(path)?;
        }

        Ok(())
    }

    /// Watch for configuration file changes
    #[instrument(skip(self))]
    pub async fn start_file_watcher(&self) -> Result<()> {
        if let Some(ref config_path) = self.config_path {
            let config_path = config_path.clone();
            let config = Arc::clone(&self.config);
            let api_keys = Arc::clone(&self.api_keys);

            tokio::spawn(async move {
                // Simple polling-based file watcher
                let mut last_modified = None;

                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

                    if let Ok(metadata) = tokio::fs::metadata(&config_path).await {
                        if let Ok(modified) = metadata.modified() {
                            if last_modified.map_or(true, |last| modified > last) {
                                last_modified = Some(modified);

                                if let Ok(new_config) = CodevConfig::load_from_file(&config_path) {
                                    debug!("Configuration file changed, reloading");

                                    let new_api_keys = new_config.load_api_keys();
                                    *api_keys.write().await = new_api_keys;
                                    *config.write().await = new_config;

                                    info!("Configuration reloaded from file");
                                }
                            }
                        }
                    }
                }
            });
        }

        Ok(())
    }

    /// Get configuration search paths
    fn get_config_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // 1. Current directory
        paths.push(PathBuf::from("./codev.toml"));
        paths.push(PathBuf::from("./config/codev.toml"));

        // 2. User config directory
        if let Some(config_dir) = dirs::config_dir() {
            paths.push(config_dir.join("codev").join("codev.toml"));
        }

        // 3. Home directory
        if let Some(home_dir) = dirs::home_dir() {
            paths.push(home_dir.join(".codev").join("codev.toml"));
            paths.push(home_dir.join(".codev.toml"));
        }

        // 4. System-wide config
        paths.push(PathBuf::from("/etc/codev/codev.toml"));

        paths
    }

    /// Get default configuration path for saving
    fn get_default_config_path() -> Option<PathBuf> {
        if let Some(config_dir) = dirs::config_dir() {
            Some(config_dir.join("codev").join("codev.toml"))
        } else if let Some(home_dir) = dirs::home_dir() {
            Some(home_dir.join(".codev").join("codev.toml"))
        } else {
            Some(PathBuf::from("./codev.toml"))
        }
    }

    /// Extract configuration value by dot-separated path
    fn extract_value_by_path(config: &CodevConfig, key_path: &str) -> Option<ConfigValue> {
        let parts: Vec<&str> = key_path.split('.').collect();

        match parts.as_slice() {
            ["ai", "default_provider"] => {
                Some(ConfigValue::String(config.ai.default_provider.to_string()))
            }
            ["security", "default_level"] => {
                Some(ConfigValue::String(format!("{:?}", config.security.default_level)))
            }
            ["logging", "level"] => {
                Some(ConfigValue::String(config.logging.level.clone()))
            }
            ["workspace", "default_path"] => {
                Some(ConfigValue::String(config.workspace.default_path.display().to_string()))
            }
            _ => None, // TODO: Implement more paths as needed
        }
    }

    /// Set configuration value by dot-separated path
    fn set_value_by_path(config: &mut CodevConfig, key_path: &str, value: ConfigValue) -> Result<()> {
        let parts: Vec<&str> = key_path.split('.').collect();

        match (parts.as_slice(), value) {
            (["ai", "default_provider"], ConfigValue::String(provider)) => {
                config.ai.default_provider = provider.parse()
                    .map_err(|_| CodevError::Config {
                        message: format!("Invalid provider: {}", provider),
                    })?;
            }
            (["logging", "level"], ConfigValue::String(level)) => {
                config.logging.level = level;
            }
            (["workspace", "default_path"], ConfigValue::String(path)) => {
                config.workspace.default_path = PathBuf::from(path);
            }
            _ => {
                return Err(CodevError::Config {
                    message: format!("Unsupported configuration path: {}", key_path),
                });
            }
        }

        Ok(())
    }
}

/// Configuration value types
#[derive(Debug, Clone)]
pub enum ConfigValue {
    String(String),
    Bool(bool),
    Number(f64),
    Array(Vec<ConfigValue>),
}

impl std::fmt::Display for ConfigValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigValue::String(s) => write!(f, "{}", s),
            ConfigValue::Bool(b) => write!(f, "{}", b),
            ConfigValue::Number(n) => write!(f, "{}", n),
            ConfigValue::Array(arr) => {
                write!(f, "[{}]",
                       arr.iter()
                           .map(|v| v.to_string())
                           .collect::<Vec<_>>()
                           .join(", ")
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_config_manager_creation() {
        let config = CodevConfig::default();
        let manager = ConfigManager::new(config);

        let current_config = manager.get_config().await;
        assert_eq!(current_config.environment, codev_shared::Environment::Development);
    }

    #[tokio::test]
    async fn test_config_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        let config = CodevConfig::default();
        let manager = ConfigManager::new(config);

        // Save config
        manager.save_to_file(&config_path).await.unwrap();
        assert!(config_path.exists());

        // Load config
        let loaded_manager = ConfigManager::from_file(&config_path).await.unwrap();
        let loaded_config = loaded_manager.get_config().await;

        assert_eq!(loaded_config.environment, codev_shared::Environment::Development);
    }

    #[tokio::test]
    async fn test_config_value_operations() {
        let config = CodevConfig::default();
        let manager = ConfigManager::new(config);

        // Get value
        let value = manager.get_value("logging.level").await;
        assert!(value.is_some());

        // Set value
        manager.set_value("logging.level", ConfigValue::String("debug".to_string())).await.unwrap();

        let updated_config = manager.get_config().await;
        assert_eq!(updated_config.logging.level, "debug");
    }

    #[test]
    fn test_config_value_display() {
        let string_val = ConfigValue::String("test".to_string());
        assert_eq!(string_val.to_string(), "test");

        let bool_val = ConfigValue::Bool(true);
        assert_eq!(bool_val.to_string(), "true");

        let array_val = ConfigValue::Array(vec![
            ConfigValue::String("a".to_string()),
            ConfigValue::String("b".to_string()),
        ]);
        assert_eq!(array_val.to_string(), "[a, b]");
    }
}