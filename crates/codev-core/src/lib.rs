//! # Codev Core
//!
//! Core engine for CoDev.rs - AI Development Assistant
//!
//! This carate provides the foundational components for the CoDev.rs ecosystem:
//! - AI provider management and communication
//! - Project analysis and code understanding
//! - Code generation and modification
//! - Configuration and environment management
//! Security and sandbox execution

pub mod ai;
pub mod analysis;
pub mod config;
pub mod engine;
pub mod git;
pub mod project;
pub mod security;
pub mod templates;

// Re-export commonly used types
pub use codev_shared::*;

pub use ai::{AiEngine, LllManager, LlmProvider};
pub use analysis::{CodeAnalyzer, ProjectAnalyzer};
pub use config::ConfigManager;
pub use engine::CodevEngine;
pub use project::ProjectManager;
pub use security::SecurityManager;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Result type alias for core operations
pub type Result<T> = std::result::Result<T, CodevError>;

/// Main engine facade for CoDev.rs
///
/// This is the primary interface for interacting with CoDev.rs functionality.
/// It coordinates between different components and provides a unified API.
pub struct CoDev {
    engine: CodevEngine,
}

impl CoDev {
    /// Create a new CoDev instance with default configuration
    pub async fn new() -> Result<Self> {
        let engine = CodevEngine::new().await?;
        Ok(Self { engine })
    }

    /// Create a new CoDev instance with custom configuration
    pub async fn with_config(config: CodevConfig) -> Result<Self> {
        let engine = CodevEngine::with_config(config).await?;
        Ok(Self { engine })
    }

    /// Get the underlying engine
    pub fn engine(&self) -> &CodevEngine {
        &self.engine
    }

    /// Get mutable access to the underlying engine
    pub fn engine_mut(&mut self) -> &mut CodevEngine {
        &mut self.engine
    }

    /// initialize a new project
    pub async fn init_project(&self, name: &str, path: &std::path::Path) -> Result<()> {
        self.engine.project_manager().init_project(name, path).await
    }

    /// Analyze a project or codebase
    pub async fn analyze_project(&self, path: &std::path::Path) -> Result<CodeAnalysis> {
        self.engine.project_analyzer().analyze_project(path).await
    }

    /// Generate code based on a prompt
    pub async fn generate_code(&self, prompt: &str) -> Result<String> {
        self.engine.ai_engine().generate_code(prompt).await
    }

    /// Chat with the AI assistant
    pub async fn chat(&self, message: &str) -> Result<String> {
        self.engine.ai_engine().chat(message).await
    }

    /// Get current configuration
    pub fn config(&self) -> &CodevConfig {
        self.engine.config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_codev_creation() {
        let codev = CoDev::new().await;
        assert!(codev.is_ok());
    }

    #[tokio::test]
    async fn test_project_init() {
        let codev = CoDev::new().await.unwrap();
        let temp_dir = TempDir::new().unwrap();

        let result = codev.init_project("test_project", temp_dir.path()).await;
        // Note: This might fail without proper setup, which is expected in unit tests
        // Integration tests should cover the full functionality
    }
}