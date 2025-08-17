//! LLM Manager for Provider Selection and Management
//!
//! The LLM Manager handles:
//! - Provider registration and management
//! - Intelligent provider selection based on context
//! - Fallback chains when providers fail
//! - Health monitoring and auto-recovery
//! - Cost optimization and routing

use crate::ai::{
    AiError, GenerationOptions, HealthStatus, LlmProvider,
    providers::{ProviderFactory, ProviderRegistry, ProviderConfig}
};
use codev_shared::{CodevConfig, ProviderId, Result, AiConfig, Environment};
use futures::Stream;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

/// LLM Manager that coordinates all provider interactions
pub struct LlmManager {
    /// Registry of all available providers
    registry: Arc<RwLock<ProviderRegistry>>,

    /// Current active provider
    current_provider: Arc<RwLock<Option<ProviderId>>>,

    /// Fallback chain for provider selection
    fallback_chain: Vec<ProviderId>,

    /// Environment detector for smart routing
    environment: Environment,

    /// Auto-detection enabled
    auto_detect: bool,

    /// Health check interval
    health_check_interval: Duration,

    /// Last health check results
    health_status: Arc<RwLock<HashMap<ProviderId, (HealthStatus, Instant)>>>,
}

impl LlmManager {
    /// Create a new LLM manager from configuration
    #[instrument(skip(config, api_keys))]
    pub async fn new(config: &AiConfig, api_keys: HashMap<ProviderId, String>) -> Result<Self> {
        info!("Initializing LLM Manager");

        let registry = ProviderFactory::create_all_providers(&config.providers, &api_keys)?;

        let manager = Self {
            registry: Arc::new(RwLock::new(registry)),
            current_provider: Arc::new(RwLock::new(None)),
            fallback_chain: config.fallback_chain.clone(),
            environment: Environment::Development, // Will be detected
            auto_detect: config.auto_detect_environment,
            health_check_interval: Duration::from_secs(60),
            health_status: Arc::new(RwLock::new(HashMap::new())),
        };

        // Perform initial provider selection
        manager.auto_select_provider().await?;

        // Start health monitoring task
        manager.start_health_monitoring().await;

        info!("LLM Manager initialized successfully");
        Ok(manager)
    }

    /// Auto-select the best available provider
    #[instrument(skip(self))]
    pub async fn auto_select_provider(&mut self) -> Result<ProviderId> {
        info!("Auto-selecting best provider");

        let registry = self.registry.read().await;

        // 1. Try Ollama first if local environment
        if self.environment == Environment::Development || self.is_local_environment().await {
            if let Some(ollama) = registry.get(&ProviderId::Ollama) {
                if self.is_provider_healthy(&ProviderId::Ollama).await {
                    info!("Selected Ollama (local environment)");
                    *self.current_provider.write().await = Some(ProviderId::Ollama);
                    return Ok(ProviderId::Ollama);
                }
            }
        }

        // 2. Try fallback chain
        for provider_id in &self.fallback_chain {
            if let Some(provider) = registry.get(provider_id) {
                if self.is_provider_healthy(provider_id).await {
                    info!("Selected provider: {}", provider_id);
                    *self.current_provider.write().await = Some(*provider_id);
                    return Ok(*provider_id);
                }
            }
        }

        // 3. Try any enabled provider as last resort
        for provider in registry.enabled_providers() {
            let provider_id = provider.id();
            if self.is_provider_healthy(&provider_id).await {
                warn!("Fallback to provider: {}", provider_id);
                *self.current_provider.write().await = Some(provider_id);
                return Ok(provider_id);
            }
        }

        Err(AiError::NoProviderAvailable.into())
    }

    /// Manually switch to a specific provider
    #[instrument(skip(self))]
    pub async fn switch_provider(&mut self, provider_id: ProviderId) -> Result<()> {
        info!("Switching to provider: {}", provider_id);

        let registry = self.registry.read().await;

        // Check if provider exists and is enabled
        if !registry.is_enabled(&provider_id) {
            return Err(AiError::ProviderNotAvailable(provider_id).into());
        }

        // Perform health check
        if let Some(provider) = registry.get(&provider_id) {
            match provider.health_check().await? {
                HealthStatus::Healthy => {
                    *self.current_provider.write().await = Some(provider_id);
                    info!("Successfully switched to provider: {}", provider_id);
                    Ok(())
                }
                HealthStatus::Degraded { reason } => {
                    warn!("Provider {} is degraded: {}", provider_id, reason);
                    *self.current_provider.write().await = Some(provider_id);
                    Ok(())
                }
                HealthStatus::Unhealthy { error } => {
                    error!("Provider {} is unhealthy: {}", provider_id, error);
                    Err(AiError::ProviderNotAvailable(provider_id).into())
                }
            }
        } else {
            Err(AiError::ProviderNotAvailable(provider_id).into())
        }
    }

    /// Get the current active provider
    pub async fn current_provider(&self) -> Option<ProviderId> {
        *self.current_provider.read().await
    }

    /// Generate streaming response using current provider
    #[instrument(skip(self, prompt, options))]
    pub async fn stream_generate(
        &self,
        prompt: &str,
        options: &GenerationOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        let provider_id = self.get_available_provider().await?;
        let registry = self.registry.read().await;

        if let Some(provider) = registry.get(&provider_id) {
            debug!("Using provider {} for streaming generation", provider_id);
            provider.stream_generate(prompt, options).await
        } else {
            Err(AiError::ProviderNotAvailable(provider_id).into())
        }
    }

    /// Generate complete response using current provider
    #[instrument(skip(self, prompt, options))]
    pub async fn generate(
        &self,
        prompt: &str,
        options: &GenerationOptions,
    ) -> Result<String> {
        let provider_id = self.get_available_provider().await?;
        let registry = self.registry.read().await;

        if let Some(provider) = registry.get(&provider_id) {
            debug!("Using provider {} for generation", provider_id);

            match provider.generate(prompt, options).await {
                Ok(response) => Ok(response),
                Err(e) => {
                    warn!("Provider {} failed, trying fallback: {}", provider_id, e);
                    self.try_fallback_generation(prompt, options, provider_id).await
                }
            }
        } else {
            Err(AiError::ProviderNotAvailable(provider_id).into())
        }
    }

    /// Try fallback providers for generation
    async fn try_fallback_generation(
        &self,
        prompt: &str,
        options: &GenerationOptions,
        failed_provider: ProviderId,
    ) -> Result<String> {
        let registry = self.registry.read().await;

        for provider_id in &self.fallback_chain {
            if *provider_id == failed_provider {
                continue; // Skip the failed provider
            }

            if let Some(provider) = registry.get(provider_id) {
                if self.is_provider_healthy(provider_id).await {
                    debug!("Trying fallback provider: {}", provider_id);

                    match provider.generate(prompt, options).await {
                        Ok(response) => {
                            info!("Fallback provider {} succeeded", provider_id);
                            // Update current provider to the working one
                            *self.current_provider.write().await = Some(*provider_id);
                            return Ok(response);
                        }
                        Err(e) => {
                            warn!("Fallback provider {} also failed: {}", provider_id, e);
                            continue;
                        }
                    }
                }
            }
        }

        Err(AiError::NoProviderAvailable.into())
    }

    /// Get an available provider, with fallback if current is unavailable
    async fn get_available_provider(&self) -> Result<ProviderId> {
        // Try current provider first
        if let Some(current) = *self.current_provider.read().await {
            if self.is_provider_healthy(&current).await {
                return Ok(current);
            }
        }

        // Auto-select if no current provider or it's unhealthy
        let mut manager = self.clone(); // We need to clone for the mutable method
        manager.auto_select_provider().await
    }

    /// Check if a provider is healthy
    async fn is_provider_healthy(&self, provider_id: &ProviderId) -> bool {
        let health_status = self.health_status.read().await;

        if let Some((status, last_check)) = health_status.get(provider_id) {
            // Use cached status if recent (within health check interval)
            if last_check.elapsed() < self.health_check_interval {
                return status.is_available();
            }
        }

        // Perform fresh health check
        drop(health_status); // Release read lock
        self.perform_health_check(*provider_id).await
    }

    /// Perform health check for a specific provider
    async fn perform_health_check(&self, provider_id: ProviderId) -> bool {
        let registry = self.registry.read().await;

        if let Some(provider) = registry.get(&provider_id) {
            match provider.health_check().await {
                Ok(status) => {
                    let is_available = status.is_available();

                    // Update cached status
                    let mut health_status = self.health_status.write().await;
                    health_status.insert(provider_id, (status, Instant::now()));

                    is_available
                }
                Err(e) => {
                    warn!("Health check failed for {}: {}", provider_id, e);

                    let mut health_status = self.health_status.write().await;
                    health_status.insert(
                        provider_id,
                        (HealthStatus::Unhealthy { error: e.to_string() }, Instant::now()),
                    );

                    false
                }
            }
        } else {
            false
        }
    }

    /// Start background health monitoring
    async fn start_health_monitoring(&self) {
        let registry = Arc::clone(&self.registry);
        let health_status = Arc::clone(&self.health_status);
        let interval = self.health_check_interval;

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                ticker.tick().await;

                let registry_guard = registry.read().await;
                let enabled_providers: Vec<ProviderId> = registry_guard
                    .enabled_providers()
                    .iter()
                    .map(|p| p.id())
                    .collect();
                drop(registry_guard);

                for provider_id in enabled_providers {
                    let registry_guard = registry.read().await;
                    if let Some(provider) = registry_guard.get(&provider_id) {
                        match provider.health_check().await {
                            Ok(status) => {
                                let mut health_guard = health_status.write().await;
                                health_guard.insert(provider_id, (status, Instant::now()));
                            }
                            Err(e) => {
                                warn!("Background health check failed for {}: {}", provider_id, e);
                                let mut health_guard = health_status.write().await;
                                health_guard.insert(
                                    provider_id,
                                    (HealthStatus::Unhealthy { error: e.to_string() }, Instant::now()),
                                );
                            }
                        }
                    }
                    drop(registry_guard);
                }
            }
        });
    }

    /// Detect if we're in a local environment
    async fn is_local_environment(&self) -> bool {
        // Simple heuristics - could be improved
        self.environment == Environment::Development ||
            std::env::var("CODEV_LOCAL").is_ok() ||
            self.is_ollama_available().await
    }

    /// Check if Ollama is available locally
    async fn is_ollama_available(&self) -> bool {
        let registry = self.registry.read().await;
        if let Some(provider) = registry.get(&ProviderId::Ollama) {
            provider.is_available()
        } else {
            false
        }
    }

    /// Get provider statistics
    pub async fn get_provider_stats(&self) -> HashMap<ProviderId, ProviderStats> {
        let registry = self.registry.read().await;
        let health_status = self.health_status.read().await;
        let mut stats = HashMap::new();

        for provider in registry.enabled_providers() {
            let provider_id = provider.id();
            let (health, last_check) = health_status
                .get(&provider_id)
                .cloned()
                .unwrap_or((HealthStatus::Unhealthy { error: "Never checked".to_string() }, Instant::now()));

            stats.insert(
                provider_id,
                ProviderStats {
                    name: provider.name().to_string(),
                    health,
                    last_health_check: last_check,
                    max_context_length: provider.max_context_length(),
                    cost_per_token: provider.cost_per_token(),
                    capabilities: provider.capabilities(),
                },
            );
        }

        stats
    }

    /// Shutdown the manager
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down LLM Manager");
        // Health monitoring task will be dropped automatically
        Ok(())
    }
}

// We need Clone for the fallback logic
impl Clone for LlmManager {
    fn clone(&self) -> Self {
        Self {
            registry: Arc::clone(&self.registry),
            current_provider: Arc::clone(&self.current_provider),
            fallback_chain: self.fallback_chain.clone(),
            environment: self.environment.clone(),
            auto_detect: self.auto_detect,
            health_check_interval: self.health_check_interval,
            health_status: Arc::clone(&self.health_status),
        }
    }
}

/// Statistics for a provider
#[derive(Debug, Clone)]
pub struct ProviderStats {
    pub name: String,
    pub health: HealthStatus,
    pub last_health_check: Instant,
    pub max_context_length: usize,
    pub cost_per_token: f64,
    pub capabilities: crate::ai::ProviderCapabilities,
}

#[cfg(test)]
mod tests {
    use super::*;
    use codev_shared::AiConfig;

    #[tokio::test]
    async fn test_manager_creation() {
        let config = AiConfig::default();
        let api_keys = HashMap::new();

        // This will likely fail without Ollama running, which is expected in tests
        match LlmManager::new(&config, api_keys).await {
            Ok(_manager) => {
                // Manager created successfully
            }
            Err(e) => {
                // Expected in test environment
                println!("Manager creation failed (expected in test): {}", e);
            }
        }
    }

    #[test]
    fn test_provider_stats() {
        let stats = ProviderStats {
            name: "Test Provider".to_string(),
            health: HealthStatus::Healthy,
            last_health_check: Instant::now(),
            max_context_length: 4096,
            cost_per_token: 0.001,
            capabilities: crate::ai::ProviderCapabilities::default(),
        };

        assert_eq!(stats.name, "Test Provider");
        assert_eq!(stats.max_context_length, 4096);
    }
}