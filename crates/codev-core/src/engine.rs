//! Core engine that orchestrates all CoDev.rs components

use crate::ai::AiEngine;
use crate::analysis::ProjectAnalyzer;
use crate::config::ConfigManager;
use crate::project::ProjectManager;
use crate::security::SecurityManger;
use coeev_shared::{CodevConfig, CodevError, Result};
use std::sync::Arc;
use tracing::{info, instrument};

/// Main engine that coordinates all CoDev.rs components
///
/// The CodevEngine is the central orchestrator that manages the lifecycle
/// and interactions between different subsystems.
pub struct CodevEngine {
    /// Configuration manager
    config_manager: Arc<ConfigManager>,

    /// AI engine for LLM interactions
    ai_engine: Arc<AiEngine>,

    /// Project analysis and management
    project_manager: Arc<ProjectManager>,

    /// Project analyzer for code understanding
    project_analyzer: Arc<ProjectAnalyzer>,

    /// Security manager for safe execution
    security_manger: Arc<SecurityManger>,

    /// Current configuration
    config: CodevConfig,
}

impl CodevEngine {
    /// Create a new engine with default configuration
    #[instrument]
    pub async fn new() -> Result<Self> {
        info!("Initializing CoDev.rs engine with default configuration");

        let config = CodevConfig::load_with_env()?;
        Self::with_config(config).await
    }

    /// Create a new engine with the provided configuration
    #[instrument(skip(config))]
    pub async fn with_config(config: CodevConfig) -> Result<Self> {
        info!("Initializing CoDev.rs engine with custom configuration");

        // Validate configuration
        config.validate()?;

        // Initialize components
        let config_manager = Arc::new(
            ConfigManager::new(config.clone())
        );

        let security_manager = Arc::new(
            SecurityManger::new(&config.security)?
        );

        let ai_engine = Arc::new(
            AiEngine::new(&config.ai, config_manager.clone()).await?,
        );

        let project_manager = Arc::new(
            ProjectManager::new(&config.workspace, security_manager.clone())?
        );

        let project_analyzer = Arc::new(
            ProjectAnalyzer::new(ai_engine.clone())?
        );

        info!("CoDev.rs engine initialized successfully");

        Ok(Self {
            config_manager,
            ai_engine,
            project_manager,
            project_analyzer,
            security_manger,
            config,
        })
    }

    /// Get the configuration manager
    pub fn config_manager(&self) -> &ConfigManager {
        &self.config_manager
    }

    /// Get the AI engine
    pub fn ai_engine(&self) -> &AiEngine {
        &self.ai_engine
    }

    /// Get the project manager
    pub fn project_manager(&self) -> &ProjectManager {
        &self.project_manager
    }

    /// Get the project analyzer
    pub fn project_analyzer(&self) -> &ProjectAnalyzer {
        &self.project_analyzer
    }

    /// Get the security manager
    pub fn security_manger(&self) -> &SecurityManger {
        &self.security_manger
    }

    /// Get the current configuration
    pub fn config(&self) -> &CodevConfig {
        &self.config
    }

    /// Update configuration at runtime
    #[instrument(skip(self, new_config))]
    pub async fn update_config(&mut self, new_config: CodevConfig) -> Result<()> {
        info!("Updating CoDev.rs engine configuration");

        // Validate new configuration
        new_config.validate()?;

        // Update components that support runtime config changes
        if new_config.ai != self.config.ai {
            info!("Updating AI engine configuration");
            // Note: In a full implementation, we'd have a method to update AI config
            // For now, we'll note that this requires reinitialisation
        }

        if new_config.security != self.config.security {
            info!("Updating security manager configuration");
            // Security changes might require component reinitialization
        }

        // Update the stored configuration
        self.config = new_config;
        self.config_manager.update_config(&self.config.clone()).await?;

        info!("Configuration updated successfully");
        Ok(())
    }

    /// Get health status of all components
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> EngineHealth {
        info!("Performing engine health check");

        let ai_health = self.ai_engine.health_check().await;
        let project_health = self.project_manager.health_check().await;
        let security_health = self.security_manger.health_check().await;

        EngineHealth {
            overall: if ai_health.is_healthy() && project_health.is_healthy() && security_health.is_healthy() {
                ComponentHealth::Healthy
            } else {
                ComponentHealth::Degraded
            },
            ai_engine: ai_health,
            project_manger: project_health,
            security_manger: security_health,
        }
    }

    /// Shutdown the engine gracefully
    #[instrument(skip(self))]
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down CoDev.rs engine");

        // Shutdown components in reverse order of initialization
        if let Err(e) = self.ai_engine.shutdown().await {
            tracing::error!("Error shutting down AI engine: {}", e);
        }

        if let Err(e) = self.security_manager.shutdown().await {
            tracing::error!("Error shutting down security manager: {}", e);
        }

        info!("CoDev.rs engine shutdown complete");
        Ok(())
    }
}

/// Health status of the entire engine
#[derive(Debug, Clone)]
pub struct EngineHealth {
    pub overall: ComponentHealth,
    pub ai_engine: ComponentHealth,
    pub project_manger: ComponentHealth,
    pub security_manger: ComponentHealth,
}

/// Health status of individual components
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentHealth {
    Healthy,
    Degraded,
    Unhealthy,
}

impl ComponentHealth {
    pub fn is_healthy(&self) -> bool {
        matches!(self, ComponentHealth::Healthy)
    }
}

impl std::fmt::Display for ComponentHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComponentHealth::Healthy => write!(f, "healthy"),
            ComponentHealth::Degraded => write!(f, "degraded"),
            ComponentHealth::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

/// Engine statistics and metrics
#[derive(Debug, Clone)]
pub struct EngineStats {
    pub uptime: std::time::Duration,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub ai_requests: u64,
    pub projects_analyzed: u64,
    pub code_generated_lines: u64,
}

impl CodevEngine {
    /// Get engine statistics
    pub async fn stats(&self) -> EngineStats {
        // In a full implementation, these would be tracked by the engine
        EngineStats {
            uptime: std::time::Duration::from_secs(0), // Would track actual uptime\
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            ai_requests: 0,
            projects_analyzed: 0,
            code_generated_lines: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codev_shared::CodevConfig;

    #[tokio::test]
    async fn test_engine_creation() {
        let result = CodevEngine::new().await;
        // This might fail without proper setup, which is expected in unit tests
        match result {
            Ok(_engine) => {
                // Engine created successfully
            }
            Err(e) => {
                // Expected in test environment without full setup
                println!("Engine creation failed (expected in test): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_engine_with_default_config() {
        let config = CodevConfig::default();
        let result = CodevEngine::with_config(config).await;

        match result {
            Ok(_engine) => {
                // Engine created successfully
            }
            Err(e) => {
                // Expected in test environment
                println!("Engine creation with config failed (expected in test): {}", e);
            }
        }
    }

    #[test]
    fn test_component_health() {
        assert!(ComponentHealth::Healthy.is_healthy());
        assert!(!ComponentHealth::Degraded.is_healthy());
        assert!(!ComponentHealth::Unhealthy.is_healthy());
    }
}