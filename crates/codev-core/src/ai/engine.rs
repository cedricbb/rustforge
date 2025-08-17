//! AI Engine - High-level interface for AI operations
//!
//! The AI Engine provides a simplified interface for common AI tasks while
//! handling the complexity of provider management, context awareness, and
//! intelligent routing.

use crate::ai::{
    AiContext, AiRequest, AiResponse, GenerationOptions, LlmManager,
    TaskType, UserPreferences, Priority
};
use crate::config::ConfigManager;
use codev_shared::{AiConfig, CodevError, ProviderId, Result};
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};

/// High-level AI Engine that provides simplified interfaces for AI operations
pub struct AiEngine {
    /// LLM manager for provider operations
    llm_manager: Arc<RwLock<LlmManager>>,

    /// Configuration manager
    config_manager: Arc<ConfigManager>,

    /// Default user preferences
    default_preferences: UserPreferences,

    /// Request cache for optimization
    cache: Arc<RwLock<RequestCache>>,

    /// Usage statistics
    stats: Arc<RwLock<EngineStats>>,
}

impl AiEngine {
    /// Create a new AI Engine
    #[instrument(skip(config, config_manager))]
    pub async fn new(config: &AiConfig, config_manager: Arc<ConfigManager>) -> Result<Self> {
        info!("Initializing AI Engine");

        let api_keys = config_manager.get_api_keys();
        let llm_manager = LlmManager::new(config, api_keys).await?;

        let engine = Self {
            llm_manager: Arc::new(RwLock::new(llm_manager)),
            config_manager,
            default_preferences: UserPreferences::default(),
            cache: Arc::new(RwLock::new(RequestCache::new())),
            stats: Arc::new(RwLock::new(EngineStats::new())),
        };

        info!("AI Engine initialized successfully");
        Ok(engine)
    }

    /// Generate code with context awareness
    #[instrument(skip(self, prompt))]
    pub async fn generate_code(&self, prompt: &str) -> Result<String> {
        debug!("Generating code for prompt");

        let context = self.build_default_context(TaskType::CodeGeneration).await;
        let options = self.build_generation_options(&context).await;

        let request = AiRequest {
            prompt: prompt.to_string(),
            task_type: TaskType::CodeGeneration,
            options,
            context,
            priority: Priority::Normal,
        };

        self.process_request(request).await.map(|response| response.content)
    }

    /// Analyze code with detailed feedback
    #[instrument(skip(self, code))]
    pub async fn analyze_code(&self, code: &str) -> Result<String> {
        debug!("Analyzing code");

        let analysis_prompt = format!(
            "Analyze this code for potential issues, improvements, and best practices:\n\n```\n{}\n```\n\n\
            Please provide:\n\
            1. Code quality assessment\n\
            2. Potential bugs or issues\n\
            3. Performance optimization suggestions\n\
            4. Best practices recommendations\n\
            5. Security considerations",
            code
        );

        let context = self.build_default_context(TaskType::CodeAnalysis).await;
        let options = self.build_generation_options(&context).await;

        let request = AiRequest {
            prompt: analysis_prompt,
            task_type: TaskType::CodeAnalysis,
            options,
            context,
            priority: Priority::Normal,
        };

        self.process_request(request).await.map(|response| response.content)
    }

    /// Chat with the AI assistant
    #[instrument(skip(self, message))]
    pub async fn chat(&self, message: &str) -> Result<String> {
        debug!("Processing chat message");

        let context = self.build_default_context(TaskType::Chat).await;
        let options = self.build_generation_options(&context).await;

        let request = AiRequest {
            prompt: message.to_string(),
            task_type: TaskType::Chat,
            options,
            context,
            priority: Priority::Normal,
        };

        self.process_request(request).await.map(|response| response.content)
    }

    /// Stream a response for real-time display
    #[instrument(skip(self, prompt))]
    pub async fn stream_chat(
        &self,
        prompt: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        debug!("Starting streaming chat");

        let context = self.build_default_context(TaskType::Chat).await;
        let mut options = self.build_generation_options(&context).await;
        options.stream = true;

        let manager = self.llm_manager.read().await;
        manager.stream_generate(prompt, &options).await
    }

    /// Review code with specific focus areas
    #[instrument(skip(self, code, focus_areas))]
    pub async fn review_code(&self, code: &str, focus_areas: &[&str]) -> Result<String> {
        debug!("Reviewing code with focus areas: {:?}", focus_areas);

        let focus_text = if focus_areas.is_empty() {
            "general code quality".to_string()
        } else {
            focus_areas.join(", ")
        };

        let review_prompt = format!(
            "Please review this code with focus on {}:\n\n```\n{}\n```\n\n\
            Provide specific, actionable feedback with examples where applicable.",
            focus_text, code
        );

        let context = self.build_default_context(TaskType::CodeReview).await;
        let options = self.build_generation_options(&context).await;

        let request = AiRequest {
            prompt: review_prompt,
            task_type: TaskType::CodeReview,
            options,
            context,
            priority: Priority::Normal,
        };

        self.process_request(request).await.map(|response| response.content)
    }

    /// Generate documentation for code
    #[instrument(skip(self, code, doc_type))]
    pub async fn generate_documentation(&self, code: &str, doc_type: DocumentationType) -> Result<String> {
        debug!("Generating documentation of type: {:?}", doc_type);

        let doc_prompt = match doc_type {
            DocumentationType::Api => {
                format!("Generate comprehensive API documentation for this code:\n\n```\n{}\n```", code)
            }
            DocumentationType::README => {
                format!("Generate a detailed README.md for this project/module:\n\n```\n{}\n```", code)
            }
            DocumentationType::Inline => {
                format!("Add inline documentation comments to this code:\n\n```\n{}\n```", code)
            }
            DocumentationType::Tutorial => {
                format!("Create a tutorial explaining how to use this code:\n\n```\n{}\n```", code)
            }
        };

        let context = self.build_default_context(TaskType::Documentation).await;
        let options = self.build_generation_options(&context).await;

        let request = AiRequest {
            prompt: doc_prompt,
            task_type: TaskType::Documentation,
            options,
            context,
            priority: Priority::Normal,
        };

        self.process_request(request).await.map(|response| response.content)
    }

    /// Debug code by analyzing errors and suggesting fixes
    #[instrument(skip(self, code, error_message))]
    pub async fn debug_code(&self, code: &str, error_message: &str) -> Result<String> {
        debug!("Debugging code with error message");

        let debug_prompt = format!(
            "Help debug this code that's producing an error:\n\n\
            Error: {}\n\n\
            Code:\n```\n{}\n```\n\n\
            Please:\n\
            1. Identify the likely cause of the error\n\
            2. Suggest specific fixes\n\
            3. Provide corrected code if applicable\n\
            4. Explain why the error occurred",
            error_message, code
        );

        let context = self.build_default_context(TaskType::Debugging).await;
        let options = self.build_generation_options(&context).await;

        let request = AiRequest {
            prompt: debug_prompt,
            task_type: TaskType::Debugging,
            options,
            context,
            priority: Priority::High, // Debugging is usually urgent
        };

        self.process_request(request).await.map(|response| response.content)
    }

    /// Process a generic AI request
    async fn process_request(&self, request: AiRequest) -> Result<AiResponse> {
        let start_time = Instant::now();

        // Check cache first
        if let Some(cached_response) = self.check_cache(&request).await {
            debug!("Returning cached response");
            self.update_stats(true, start_time.elapsed()).await;
            return Ok(cached_response);
        }

        // Process with LLM
        let manager = self.llm_manager.read().await;
        let response_content = manager.generate(&request.prompt, &request.options).await?;

        let current_provider = manager.current_provider().await
            .ok_or_else(|| CodevError::Internal { message: "No current provider".to_string() })?;

        let response = AiResponse {
            content: response_content,
            provider: current_provider,
            model: "unknown".to_string(), // TODO: Get actual model from provider
            usage: crate::ai::UsageStats {
                prompt_tokens: estimate_tokens(&request.prompt),
                completion_tokens: estimate_tokens(&response_content),
                total_tokens: estimate_tokens(&request.prompt) + estimate_tokens(&response_content),
                estimated_cost: None,
            },
            metadata: crate::ai::ResponseMetadata {
                response_time: start_time.elapsed(),
                model_version: None,
                finish_reason: None,
                safety_filtered: false,
            },
        };

        // Cache the response
        self.cache_response(&request, &response).await;

        // Update statistics
        self.update_stats(false, start_time.elapsed()).await;

        Ok(response)
    }

    /// Build default context for requests
    async fn build_default_context(&self, task_type: TaskType) -> AiContext {
        AiContext {
            project_context: None, // TODO: Integrate with project analyzer
            conversation_history: Vec::new(), // TODO: Implement conversation memory
            user_preferences: self.default_preferences.clone(),
            task_type,
        }
    }

    /// Build generation options based on context
    async fn build_generation_options(&self, context: &AiContext) -> GenerationOptions {
        let mut options = GenerationOptions::default();

        // Adjust based on task type
        match context.task_type {
            TaskType::CodeGeneration => {
                options.temperature = Some(0.1); // Lower temperature for more deterministic code
                options.max_tokens = Some(4096);
            }
            TaskType::CodeAnalysis | TaskType::CodeReview => {
                options.temperature = Some(0.1);
                options.max_tokens = Some(2048);
            }
            TaskType::Chat => {
                options.temperature = Some(0.7); // Higher temperature for more creative responses
                options.max_tokens = Some(1024);
            }
            TaskType::Documentation => {
                options.temperature = Some(0.3);
                options.max_tokens = Some(3072);
            }
            TaskType::Debugging => {
                options.temperature = Some(0.1); // Very focused responses for debugging
                options.max_tokens = Some(2048);
            }
            TaskType::Refactoring => {
                options.temperature = Some(0.1);
                options.max_tokens = Some(4096);
            }
        }

        // Adjust based on user preferences
        match context.user_preferences.verbosity {
            crate::ai::VerbosityLevel::Brief => {
                options.max_tokens = options.max_tokens.map(|t| t / 2);
            }
            crate::ai::VerbosityLevel::Comprehensive => {
                options.max_tokens = options.max_tokens.map(|t| t * 2);
            }
            _ => {} // Normal and Detailed use default
        }

        options
    }

    /// Check cache for existing response
    async fn check_cache(&self, request: &AiRequest) -> Option<AiResponse> {
        let cache = self.cache.read().await;
        cache.get(&cache_key(request))
    }

    /// Cache a response
    async fn cache_response(&self, request: &AiRequest, response: &AiResponse) {
        let mut cache = self.cache.write().await;
        cache.insert(cache_key(request), response.clone());
    }

    /// Update engine statistics
    async fn update_stats(&self, was_cached: bool, response_time: Duration) {
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;

        if was_cached {
            stats.cached_responses += 1;
        } else {
            stats.ai_requests += 1;
        }

        stats.total_response_time += response_time;
        stats.average_response_time = stats.total_response_time / stats.total_requests as u32;
    }

    /// Get current provider
    pub async fn current_provider(&self) -> Option<ProviderId> {
        let manager = self.llm_manager.read().await;
        manager.current_provider().await
    }

    /// Switch to a specific provider
    pub async fn switch_provider(&self, provider_id: ProviderId) -> Result<()> {
        let mut manager = self.llm_manager.write().await;
        manager.switch_provider(provider_id).await
    }

    /// Get engine health status
    pub async fn health_check(&self) -> crate::engine::ComponentHealth {
        let manager = self.llm_manager.read().await;

        // Try a simple generation to test health
        let test_options = GenerationOptions {
            max_tokens: Some(10),
            temperature: Some(0.1),
            ..Default::default()
        };

        match manager.generate("test", &test_options).await {
            Ok(_) => crate::engine::ComponentHealth::Healthy,
            Err(_) => crate::engine::ComponentHealth::Degraded,
        }
    }

    /// Get provider statistics
    pub async fn get_provider_stats(&self) -> std::collections::HashMap<ProviderId, crate::ai::manager::ProviderStats> {
        let manager = self.llm_manager.read().await;
        manager.get_provider_stats().await
    }

    /// Shutdown the engine
    pub async fn shutdown(&self) -> Result<()> {
        let manager = self.llm_manager.read().await;
        manager.shutdown().await
    }
}

/// Type of documentation to generate
#[derive(Debug, Clone, PartialEq)]
pub enum DocumentationType {
    Api,
    README,
    Inline,
    Tutorial,
}

/// Simple request cache
struct RequestCache {
    cache: std::collections::HashMap<String, AiResponse>,
    max_size: usize,
}

impl RequestCache {
    fn new() -> Self {
        Self {
            cache: std::collections::HashMap::new(),
            max_size: 1000, // Configurable cache size
        }
    }

    fn get(&self, key: &str) -> Option<AiResponse> {
        self.cache.get(key).cloned()
    }

    fn insert(&mut self, key: String, response: AiResponse) {
        if self.cache.len() >= self.max_size {
            // Simple LRU: remove oldest entries
            // In production, use a proper LRU cache
            if let Some(oldest_key) = self.cache.keys().next().cloned() {
                self.cache.remove(&oldest_key);
            }
        }
        self.cache.insert(key, response);
    }
}

/// Engine statistics
#[derive(Debug, Clone)]
struct EngineStats {
    total_requests: u64,
    ai_requests: u64,
    cached_responses: u64,
    total_response_time: Duration,
    average_response_time: Duration,
}

impl EngineStats {
    fn new() -> Self {
        Self {
            total_requests: 0,
            ai_requests: 0,
            cached_responses: 0,
            total_response_time: Duration::ZERO,
            average_response_time: Duration::ZERO,
        }
    }
}

/// Generate a cache key for a request
fn cache_key(request: &AiRequest) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    request.prompt.hash(&mut hasher);
    request.task_type.hash(&mut hasher);
    // Include relevant options in hash
    if let Some(temp) = request.options.temperature {
        temp.to_bits().hash(&mut hasher);
    }
    if let Some(max_tokens) = request.options.max_tokens {
        max_tokens.hash(&mut hasher);
    }

    format!("cache_{:x}", hasher.finish())
}

/// Estimate token count (simple approximation)
fn estimate_tokens(text: &str) -> usize {
    // Rough approximation: 1 token ≈ 4 characters for English
    // This is a simplification - real tokenizers are more complex
    (text.len() / 4).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use codev_shared::AiConfig;
    use crate::config::ConfigManager;

    #[tokio::test]
    async fn test_cache_key_generation() {
        let request = AiRequest {
            prompt: "test prompt".to_string(),
            task_type: TaskType::Chat,
            options: GenerationOptions::default(),
            context: AiContext {
                project_context: None,
                conversation_history: Vec::new(),
                user_preferences: UserPreferences::default(),
                task_type: TaskType::Chat,
            },
            priority: Priority::Normal,
        };

        let key1 = cache_key(&request);
        let key2 = cache_key(&request);
        assert_eq!(key1, key2);

        let mut request2 = request.clone();
        request2.prompt = "different prompt".to_string();
        let key3 = cache_key(&request2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_token_estimation() {
        assert_eq!(estimate_tokens(""), 1);
        assert_eq!(estimate_tokens("test"), 1);
        assert_eq!(estimate_tokens("this is a test"), 3);
        assert_eq!(estimate_tokens("a".repeat(100).as_str()), 25);
    }

    #[test]
    fn test_documentation_type() {
        assert_eq!(DocumentationType::Api, DocumentationType::Api);
        assert_ne!(DocumentationType::Api, DocumentationType::README);
    }
}