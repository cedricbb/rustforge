//! LLM Provider Implementations
//!
//! This module contains implementations for various LLM providers that CoDev.rs supports.
//! Each provider implements the LlmProvider trait to ensure consistent behavior.

pub mod ollama;

// External providers (will be implemented in Phase 3)
pub mod openai;
pub mod claude;
pub mod mistral;
pub mod gemini;

// Re-export provider implementations
pub use ollama::OllamaProvider;
pub use openai::OpenAIProvider;
pub use claude::ClaudeProvider;
pub use mistral::MistralProvider;
pub use gemini::GeminiProvider;

use crate::ai::{LlmProvider, ProviderCapabilities};
use codev_shared::{ProviderId, Result};
use serde::{ Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Type alias for boxed provider
pub type BoxedProvider = Box<dyn LlmProvider + Send + Sync>;

/// Provider configuration from config file
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub enabled: bool,
    pub model: String,
    pub endpoint: Option<String>,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
    pub timeout_seconds: Option<u64>,
    pub max_retries: Option<u32>,
}

/// Provider registry for managing all available providers
pub struct ProviderRegistry {
    providers: HashMap<ProviderId, BoxedProvider>,
    configs: HashMap<ProviderId, ProviderConfig>,
}

impl ProviderRegistry {
    /// Create a new provider registry
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            configs: HashMap::new(),
        }
    }

    /// Register a provider with its configuration
    pub fn register(&mut self, provider: BoxedProvider, config: ProviderConfig) {
        let id = provider.id();
        self.configs.insert(id, config);
        self.providers.insert(id, provider);
    }

    /// Get a provider by ID
    pub fn get(&self, id: &ProviderId) -> Option<BoxedProvider> {
        self.providers.get(id)
    }

    /// Get all registered providers
    pub fn list(&self) -> Vec<BoxedProvider> {
        self.providers.values().collect()
    }

    /// Get enabled providers only
    pub fn enabled_providers(&self) -> Vec<BoxedProvider> {
        self.providers
            .iter()
            .filter(|(id, _)| {
                self.configs
                .get(id)
                .map(|config| config.enabled)
                .unwrap_or(false)
            })
            .map(|(_, provider)| provider.as_ref())
            .collect()
    }

    /// Check if a provider is registered and enabled
    pub fn is_enabled(&self, id: &ProviderId) -> bool {
        self.configs
            .get(id)
            .map(|config| config.enabled)
            .unwrap_or(false)
    }

    /// Get a provider configuration
    pub fn get_config(&self, id: &ProviderId) -> Option<&ProviderConfig> {
        self.configs.get(id)
    }

    /// Update provider configuration
    pub fn update_config(&mut self, id: &ProviderId, config: ProviderConfig) {
        self.configs.insert(id, config);
    }

    /// Remove a provider
    pub fn remove(&mut self, id: &ProviderId) -> Option<BoxedProvider> {
        self.providers.remove(id);
        self.configs.remove(id)
    }

    /// Get provider capabilities
    pub fn get_capabilities(&self, id: &ProviderId) -> Option<ProviderCapabilities> {
        self.providers.get(id).map(|p| p.capabilities())
    }

    /// Find providers supporting specific capabilities
    pub fn find_providers_with_capability<F>(&self, predicate: F) -> Vec<&BoxedProvider>
    where
        F: Fn(&ProviderCapabilities) -> bool,
    {
        self.enabled_providers()
            .into_iter()
            .filter(|provider| predicate(&provider.capabilities()))
            .collect()
    }
}

/// Provider factory for creating providers from configuration
pub struct ProviderFactory;

impl ProviderFactory {
    /// Create a provider from configuration
    pub fn create_provider(
        id: ProviderId,
        config: &ProviderConfig,
        api_keys: &HashMap<ProviderId, String>,
    ) -> Result<BoxedProvider> {
        match id {
            ProviderId::Ollama => {
                let endpoint = config
                    .endpoint
                    .clone()
                    .unwrap_or_else(|| "http://localhost:11434".to_string());

                let timeout = std::time::Duration::from_secs(
                    config.timeout_seconds.unwrap_or(30)
                );

                let max_retries = config.max_retries.unwrap_or(3);

                let provider = OllamaProvider::with_config(
                    endpoint,
                    config.model.clone(),
                    timeout,
                    max_retries,
                );

                Ok(Box::new(provider))
            }
            ProviderId::OpenAI => {
                let api_key = api_keys
                    .get(&ProviderId::OpenAi)
                    .ok_or_else(|| codev_shared::CodevError::Config {
                        message: "OpenAi API key not found".to_string(),
                    })?;

                let provider = OpenAIProvider::new(
                    api_key.clone(),
                    config.model.clone(),
                    config.endpoint.clone(),
                )?;

                Ok(Box::new(provider))
            }
            ProviderId::Claude => {
                let api_key = api_keys
                    .get(&ProviderId::Claude)
                    .ok_or_else(|| codev_shared::CodevError::Config {
                        message: "Claude API key not found".to_string(),
                    })?;

                let provider = ClaudeProvider::new(
                    api_key.clone(),
                    config.model.clone(),
                    config.endpoint.clone(),
                )?;

                Ok(Box::new(provider))
            }
            ProviderId::Mistral => {
                let api_key = api_keys
                    .get(&ProviderId::Mistral)
                    .ok_or_else(|| codev_shared::CodevError::Config {
                        message: "Mistral API key not found".to_string(),
                    })?;

                let provider = MistralProvider::new(
                    api_key.clone(),
                    config.model.clone(),
                    config.endpoint.clone(),
                )?;

                Ok(Box::new(provider))
            }
            ProviderId::Gemini => {
                let api_key = api_keys
                    .get(&ProviderId::Gemini)
                    .ok_or_else(|| codev_shared::CodevError::Config {
                        message: "Gemini API key not found".to_string(),
                    })?;

                let provider = GeminiProvider::new(
                    api_key.clone(),
                    config.model.clone(),
                    config.endpoint.clone(),
                )?;

                Ok(Box::new(provider))
            }
        }
    }

    /// Create all enabled providers from configuration
    pub fn create_all_providers(
        configs: &HashMap<ProviderId, ProviderConfig>,
        api_keys: &HashMap<ProviderId, String>,
    ) -> Result<ProviderRegistry> {
        let mut registry = ProviderRegistry::new();

        for (id, config) in configs{
            if !config.enabled {
                continue;
            }

            match Self::create_provider(*id, config, api_keys) {
                Ok(provider) => {
                    registry.register(provider, config.clone());
                }
                Err(e) => {
                    tracing::warn!("Failed to create provider {}: {}", id, e);
                    // continue with other providers instead of failing completely
                }
            }
        }

        Ok(registry)
    }
}

/// Provider type enumeration for pattern matching
#[derive(Debug, Clone, PartialEq)]
pub enum ProviderType {
    Local(ProviderId), // Local providers like Ollama
    Cloud(ProviderId), // Cloud providers like OpenAi, Claude
    SelfHosted(ProviderId), // Self-hosted instances
}

impl ProviderType {
    /// Get the provider ID
    pub fn id(&self) -> ProviderId {
        match self {
            ProviderType::Local(id) | ProviderType::Cloud(id) | ProviderType::SelfHosted(id) => *id,
        }
    }

    /// Check if this is a local provider
    pub fn is_local(&self) -> bool {
        matches!(self, ProviderType::Local(_))
    }

    /// Check if this is a cloud provider
    pub fn is_cloud(&self) -> bool {
        matches!(self, ProviderType::Cloud(_))
    }

    /// Check if this is a cloud provider
    pub fn is_self_hosted(&self) -> bool {
        matches!(self, ProviderType::SelfHosted(_))
    }

    /// Get the provider type from ID
    pub fn from_id(id: ProviderId) -> Self {
        match id {
            ProviderId::Ollama => ProviderType::Local(id),
            ProviderId::OpenAI | ProviderId::Claude | ProviderId::Mistral  | ProviderId::Gemini => {
                ProviderType::Cloud(id)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_registry() {
        let mut registry = ProviderRegistry::new();

        // initially empty
        assert!(registry.list().is_empty());
        assert!(registry.enabled_providers().is_empty());
    }

    #[test]
    fn test_provider_type() {
        assert!(ProviderType::from_id(ProviderId::Ollama).is_local());
        assert!(ProviderType::from_id(ProviderId::OpenAI).is_cloud());
        assert!(ProviderType::from_id(ProviderId::Claude).is_cloud());
    }

    #[test]
    fn test_provider_config() {
        let config = ProviderConfig {
            enabled: true,
            model: "test-model".to_string(),
            endpoint: Some("http://localhost:8080".to_string()),
            max_tokens: Some(1000),
            temperature: Some(0.7),
            timeout_seconds: Some(30),
            max_retries: Some(3),
        };

        assert!(config.enabled);
        assert_eq!(config.model, "test-model");
        assert_eq!(config.endpoint, Some("http://localhost:8080".to_string()));
    }
}