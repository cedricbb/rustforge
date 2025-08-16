//! AI Engine and LLM Provider Management
//!
//! This module provides the core AI functionality for CoDev.rs including:
//! - Abstract LLM Provider interface
//! - Multiple provider implementations (Ollama, OpenAI, Claude, etc.)
//! - Intelligent provider selection and fallback
//! - Streaming response handling
//! - Cost optimization and routing

pub mod engine;
pub mod manager;
pub mod providers;
pub mod streaming;

// Re-export main types
pub use engine::AiEngine;
pub use manager::LlmManager;
pub use providers::{LlmProvider, ProviderType};
pub use streaming::{StreamingResponse, TokenStream};

use async_trait::async_trait;
use codev_shared::{CodevError, ProviderId, Result};
use futures::Stream;
use serde::{ Deserialize, Serialize};
use std::pin::Pin;
use std::time::Duration;

/// Abstract trait for all LLM providers
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Get the provider identifier
    fn id(&self) -> ProviderId;

    /// Get the provider name
    fn name(&self) -> &str;

    /// Check if the provider is currently available
    fn is_available(&self) -> bool;

    /// Perform a health check
    async fn health_check(&self) -> Result<(HealthStatus)>;

    /// Generate a streaming response
    async fn stream_generate(
        &self,
        prompt: &str,
        options: &GenerationOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>>;

    /// Generate a complete response (non-streaming)
    async fn generate(&self, prompt: &str, options: GenerationOptions) -> Result<String>;

    /// Get the maximum context length for this provider
    fn max_content_length(&self) -> usize;

    /// Get the cost per token (for optimization)
    fn cost_per_token(&self) -> f64;

    /// Get supported capabilities
    fn capabilities(&self) -> ProviderCapabilities;
}

/// Health status of a provider
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { error: String },
}

impl HealthStatus {
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy)
    }

    pub fn is_available(&self) -> bool {
        !matches!(self, HealthStatus::Unhealthy { .. })
    }
}

/// Options for test generation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GenerationOptions {
    /// Maximum number of tokens to generate
    pub max_tokens: Option<usize>,

    /// Temperature for randomness (0.0 - 2.0)
    pub temperature: Option<f32>,

    /// Top-p sampling parameter
    pub top_p: Option<f32>,

    /// Frequency penalty
    pub frequency_penalty: Option<f32>,

    /// Presence penalty
    pub presence_penalty: Option<f32>,

    /// Stop sequences
    pub stop: Option<Vec<String>>,

    /// Whether to stream the response
    pub stream: bool
}

impl Default for GenerationOptions {
    fn default() -> Self {
        Self {
            max_tokens: Some(4096),
            temperature: Some(0.1),
            top_p: Some(0.9),
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            stream: false,
        }
    }
}

/// Capabilities supported by a provider
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ProviderCapabilities {
    /// Supports streaming responses
    pub streaming: bool,

    /// Supports chat/conversation format
    pub chat: bool,

    /// Supports code generation
    pub code_generation: bool,

    /// Supports code analysis
    pub code_analysis: bool,

    /// Supports function calling
    pub function_calling: bool,

    /// Maximum supported context length
    pub max_context_length: usize,

    /// Supported languages for code tasks
    pub supported_languages: Vec<String>,
}

impl Default for ProviderCapabilities {
    fn default() -> Self {
        Self {
            streaming: true,
            chat: true,
            code_generation: true,
            code_analysis: true,
            function_calling: false,
            max_context_length: 4096,
            supported_languages: vec![
                "rust".to_string(),
                "python".to_string(),
                "javascript".to_string(),
                "typescript".to_string(),
                "go".to_string(),
                "react".to_string(),
            ],
        }
    }
}

/// Context for AI requests
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AiContext {
    /// Current project information
    pub project_context: Option<ProjectContext>,

    /// Conversation history
    pub conversation_history: Vec<ChatMessage>,

    /// User preferences
    pub user_preferences: UserPreferences,

    /// Task type being performed
    pub task_type: TaskType,
}

/// Type of task being performed
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TaskType {
    Chat,
    CodeGeneration,
    CodeAnalysis,
    CodeReview,
    Documentation,
    Debugging,
    Refactoring,
}

/// Project context information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectContext {
    pub name: String,
    pub language: String,
    pub framework: Option<String>,
    pub dependencies: Vec<String>,
    pub files: Vec<FileContext>,
}

/// Context about a specific file
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileContext {
    pub path: String,
    pub language: String,
    pub content: Option<String>, // Only included if relevant
    pub summary: Option<String>,
}

/// Chat message in conversation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: Option<MessageMetadata>,
}

/// Role of a message
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Additional metadata for message
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessageMetadata {
    pub provider: ProviderId,
    pub model: String,
    pub tokens_used: Option<usize>,
    pub response_time: Option<Duration>,
    pub cost: Option<f64>,
}

/// User preferences for AI interactions
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserPreferences {
    /// Preferred coding style
    pub coding_style: CodingStyle,

    /// Verbosity level for explanations
    pub verbosity: VerbosityLevel,

    /// Preferred languages
    pub preferred_languages: Vec<String>,

    /// Whether to include explanations with code
    pub include_explanations: bool,

    /// Whether to include tests with generated code
    pub include_tests: bool,
}

/// Coding style preferences
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CodingStyle {
    Minimal,
    Standard,
    Verbose,
    Custom(String),
}

/// Verbosity level for AI responses
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum VerbosityLevel {
    Brief,
    Normal,
    Detailed,
    Comprehensive,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            coding_style: CodingStyle::Standard,
            verbosity: VerbosityLevel::Normal,
            preferred_languages: vec!["rust".to_string()],
            include_explanations: true,
            include_tests: false,
        }
    }
}

/// AI request for code generation or analysis
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AiRequest {
    /// The main prompt or query
    pub prompt: String,

    /// Type of task
    pub task_type: TaskType,

    /// Generation options
    pub options: GenerationOptions,

    /// Context information
    pub context: AiContext,

    /// Priority level (for routing decisions)
    pub priority: Priority,
}

/// Priority level for requests
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, PartialOrd)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

/// Response from AI provider
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AiResponse {
    /// Generated content
    pub content: String,

    /// Provider that generated the response
    pub provider: ProviderId,

    /// Model used
    pub model: String,

    /// Usage statistics
    pub usage: UsageStats,

    /// Response metadata
    pub metadata: ResponseMetadata,
}

/// Usage statistics for a response
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UsageStats {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
    pub estimated_cost: Option<f64>,
}

/// Metadata about the response
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResponseMetadata {
    pub response_time: Duration,
    pub model_version: Option<String>,
    pub finish_reason: Option<String>,
    pub safety_filtered: bool,
}

/// Error types specific to AI operations
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AiError {
    ProviderNotAvailable(ProviderId),
    NoProviderAvailable,
    ContextTooLong { tokens: usize, max_tokens: usize },
    RateLimited(ProviderId),
    InvalidApiKey(ProviderId),
    ModelNotFound { provider: ProviderId, model: String },
    StreamingError(String),
    NetworkTimeout(ProviderId),
    ServerError { provider: ProviderId, status: u16, message: String },
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiError::ProviderNotAvailable(provider) => {
                write!(f, "provider '{}' is not available", provider)
            }
            AiError::NoProviderAvailable => {
                write!(f, "no AI provider is currently available")
            }
            AiError::ContextTooLong { tokens, max_tokens } => {
                write!(f, "context too long: {} tokens > {} max", tokens, max_tokens)
            }
            AiError::RateLimited(provider) => {
                write!(f, "Rate limited by provider: {}", provider)
            }
            AiError::InvalidApiKey(provider) => {
                write!(f, "Invalid API key for provider {}", provider)
            }
            AiError::ModelNotFound { provider, model } => {
                write!(f, "Model '{}' not found for provider {}", model, provider)
            }
            AiError::StreamingError(msg) => {
                write!(f, "Streaming error: {}", msg)
            }
            AiError::NetworkTimeout(provider) => {
                write!(f, "Network timeout for provider {}", provider)
            }
            AiError::ServerError { provider, status, message } => {
                write!(f, "Server error from {}: {} - {}", provider, status, message)
            }
        }
    }
}

impl std::error::Error for AiError {}

impl From<AiError> for CodevError {
    fn from(error: AiError) -> Self {
        match error {
            AiError::ProviderNotAvailable (provider) => {
                CodevError::LlmProvider {
                    provider,
                    message: "Provider not available".to_string(),
                }
            }
            _ => CodevError::LlmProvider {
                provider: ProviderId::Ollama, // Default fallback
                message: error.to_string(),
            },
        }
    }
}