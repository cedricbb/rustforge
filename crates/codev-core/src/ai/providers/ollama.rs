//! Ollama Provider Implementation
//!
//! Provides integration with Ollama for local LLM inference.
//! This is the primary provider for CoDev.rs, offering privacy-first AI capabilities.

use crate::ai:: {
    AiError, GenerationOptions, HealthStatus, LlmProvider, ProviderCapabilities,
};
use async_trait::async_trait;
use codev_shared::{ProviderId, Result};
use futures::{Stream, StreamExt};
use reqwest::{Client, Response};
use serde::{ Serialize, Deserialize};
use std::pin::Pin;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, instrument, warn};

/// Ollama provider for local LLM inference
pub struct OllamaProvider {
    client: Client,
    endpoint: String,
    model: String,
    timeout: Duration,
    max_retries: u32,
}

/// Request payload for Ollama API
#[derive(Serialize, Debug)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

/// Options specific to Ollama
#[derive(Serialize, Debug)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<f32>, // max_tokens equivalent
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Opttion<Vec<String>>,
}

/// Response from Ollama API
#[derive(Deserialize, Debug)]
struct OllamaResponse {
    #[serde(default)]
    response: String,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    context: Option<Vec<i32>>,
    #[serde(default)]
    total_duration: Option<u64>,
    #[serde(default)]
    load_duration: Option<u64>,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
    #[serde(default)]
    eval_count: Option<u32>,
}

/// Information about available models
#[derive(Deserialize, Debug)]
struct ModelsResponse {
    models: Vec<ModelInfo>,
}

/// Information about a single model
#[derive(Deserialize, Debug)]
struct ModelInfo {
    name: String,
    size: u64,
    digest: String,
    #[serde(default)]
    details: ModelDetails,
}

/// Detailed model information
#[derive(Deserialize, Debug, Default)]
struct ModelDetails {
    #[serde(default)]
    format: String,
    #[serde(default)]
    family: String,
    #[serde(default)]
    families: Option<Vec<String>>,
    #[serde(default)]
    parameter_size: String,
    #[serde(default)]
    quantization_level: String,
}

impl OllamaProvider {
    /// Create a new Ollama provider
    pub fn new(endpoint: String, model: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300)) // 5 minutes for model loading
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            endpoint: endpoint.trim_end_matches('/').to_string(),
            model,
            timeout: Duration::from_secs(30),
            max_retries: 3,
        }
    }

    /// Create with custom configuration
    pub fn with_config(
        endpoint: String,
        model: String,
        timeout: Duration,
        max_retries: u32,
    ) -> Self {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            endpoint: endpoint.trim_end_matches('/').to_string(),
            model,
            timeout,
            max_retries,
        }
    }

    /// Check  if Ollama service is running
    async fn is_service_running(&self) -> bool {
        match self.client.get(&format!("{}/api/tags", self.endpoint)).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    /// Get list of available models
    async fn get_available_models(&self) -> Result<Vec<String>> {
        let response = self
            .client
            .get(&format!("{}/api/models", self.endpoint))
            .send()
            .await
            .map_err(|e| AiError::NetworkTimeout(self.id()))?;

        if !response.status().is_success() {
            return Err(AiError::ServerError {
                provider: self.id(),
                status: response.status().as_u16(),
                message: "Failed to get models".to_string(),
            }.into());
        }

        let models_response: ModelsResponse = response
            .json()
            .await
            .map_err(|e| AiError::StreamingError(format!("Failed to parse models response: {}", e)))?;

        Ok(models_response.into_iter().map(|m| m.name).collect())
    }

    /// Check if the configured model is available
    async fn is_model_available(&self) -> Result<bool> {
        let models = self.get_available_models().await?;
        Ok(models.contains(&self.model))
    }

    /// Pull a model if not available
    #[instrument(skip(self))]
    async fn pull_model_if_needed(&self) -> Result<()> {
        if self.is_model_available().await? {
            debug!("Model {} is already available", self.model);
            return Ok(());
        }

        info!("Pulling model {}", self.model);

        let pull_request = serde_json::json!({
            "name": self.model,
        });

        let response = self
            .client
            .post(&format!("{}/api/pull", self.endpoint))
            .json(&pull_request)
            .send()
            .await
            .map_err(|e| AiError::NetworkTimeout(self.id()))?;

        if !response.status().is_success() {
            return Err(AiError::ServerError {
                provider: self.id(),
                status: response.status().as_u16(),
                message: format!("Failed to pulle model {}", self.model),
            }.into());
        }

        // Read the streaming response to completion
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(_) => {
                    // Progress updates could be parsed here
                    debug!("Pulling model...")
                }
                Err(e) => {
                    warn!("Error during model pull: {}", e);
                }
            }
        }

        info!("Model {} pulled successfully", self.model);
        Ok(())
    }

    /// Convert generation options to Ollama format
    fn convert_options(&self, options: &GenerationOptions) -> OllamaOptions {
        OllamaOptions {
            temperature: options.temperature,
            top_p: options.top_p,
            num_predict: options.max_tokens.map(|t| t as i32),
            stop: options.stop.clone(),
        }
    }

    /// Perform request with retries
    async fn request_with_retries<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;

        for attempt in 1..=self.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.max_retries {
                        let delay = Duration::from_millis(1000 * attempt as u64);
                        warn!("Request failed, retrying in {:?} (attempt {}/{})", delay, attempt, self.max_retries);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Ollama
    }

    fn name(&self) -> &str {
        "Ollama"
    }

    fn is_available(&self) -> bool {
        // This is a synchronous check - we'll do a more thorough check in health_check
        true
    }

    #[instrument(skip(self))]
    async fn health_check(&self) -> Result<(HealthStatus)> {
        debug!("Performing Ollama health check");

        // Check if service is running
        if !self.is_service_running().await {
            return Ok(HealthStatus::Unhealthy {
                error: format!("Ollama service not accessible at {}", self.endpoint),
            });
        }

        // Check if model is available
        match self.is_model_available().await {
            Ok(true) => {
                debug!("Ollama health check passed");
                Ok(HealthStatus::Healthy)
            }
            Ok(false) => {
                // Try to pull the model
                match self.pull_model_if_needed().await {
                    Ok(_) => Ok(HealthStatus::Healthy),
                    Err(e) => Ok(HealthStatus::Degraded {
                        reason: format!("Model {} not available and pull failed: {}", self.model, e),
                    }),
                }
            }
            Err(e) => Ok(HealthStatus::Degraded {
                reason: format1("Failed to check model availability: {}", e)
            })
        }
    }

    #[instrument(skip(self, prompt, options))]
    async fn stream_generate(
        &self,
        prompt: &str,
        options: &GenerationOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        debug!("Starting streaming generation with Ollama");

        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: true,
            options: Some(self.convert_options(options)),
        };

        let response = self.request_with_retries(|| async {
            self.client
                .post(&format!("{}/api/generate", self.endpoint))
                .json(&request)
                .send()
                .await
                .map_err(|e| AiError::NetworkTimeout(self.id()).into())
        }).await?;

        if !response.status().is_success() {
            return Err(AiError::ServerError {
                provider: self.id(),
                status: response.status().as_u16(),
                message: "Generate request failed".to_string(),
            }.into());
        }

        let stream = response
            .bytes_stream()
            .map(|chunk| {
                chunk
                    .map_err(|e| AiError::StreamingError(e.to_string()).into())
                    .and_then(|bytes| {
                        let text = String::from_utf8_lossy(&bytes);

                        // Parse each line as JSON
                        for line in text.lines() {
                            if line.trim().is_empty() {
                                continue;
                            }

                            match serde_json::from_str::<OllamaResponse>(line) {
                                Ok(ollama_response) => {
                                    if !ollama_response.response.is_empty() {
                                        return Ok(ollama_response.response);
                                    }
                                }
                                Err(e) => {
                                    debug!("Failed to parse streaming response: {} - Line: {}", e, line);
                                }
                            }
                        }

                        Ok(String::new())
                    })
            })
            .filter(|result| {
                // Filter out empty strings
                futures::future::ready(match result {
                    Ok(text) => !text.is_empty(),
                    Err(_) => true,
                })
            });

        Ok(Box::pin(stream))
    }

    #[instrument(skip(self, prompt, options))]
    async fn generate(&self, prompt: &str, options: &GenerationOptions) -> Result<String> {
        debug!("Starting non-streaming generation with Ollama");

        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
            options: Some(self.convert_options(options)),
        };

        let start_time = Instant::now();

        let response = self.request_with_retries(|| async {
            self.client
                .post(&format!("{}/api/generate", self.endpoint))
                .json(&request)
                .send()
                .await
                .map_err(|e| AiError::NetworkTimeout(self.id()).into())
        }).await?;

        if !response.status().is_success() {
            return Err(AiError::ServerError {
                provider: self.id(),
                status: response.status().as_u16(),
                message: "Generate request failed".to_string(),
            }.into());
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .map_err(|e| AiError::StreamingError(format!("Failed to parse streaming response: {}", e)))?;

        let duration = start_time.elapsed();
        debug!("Generation completed in {:?}", duration);

        Ok(ollama_response.response)
    }

    fn max_context_length(&self) -> usize {
        // Default Ollama context length - this could be made configurable
        match self.model.as_str() {
            m if m.contains("codellama") => 16384,
            m if m.contains("llama2") => 4096,
            m if m.contains("deepseek") => 8192,
            _ => 4096,
        }
    }

    fn cost_per_token(&self) -> f64 {
        // Ollama is free for local usage
        0.0
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            streaming: true,
            chat: true,
            code_generation: self.model.contains("code") || self.model.contains("deepseek"),
            code_analysis: true,
            function_calling: false, // Ollama doesn't support function calling yet
            max_content_length: self.max_context_length(),
            supported_languages: vec![
                "rust".to_string(),
                "python".to_string(),
                "javascript".to_string(),
                "typescript".to_string(),
                "go".to_string(),
                "java".to_string(),
                "cpp".to_string(),
                "c".to_string(),
            ]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_ollama_provider_creation() {
        let provider = OllamaProvider::new(
            "http://localhost:11434".to_string(),
            "codellama:7b".to_string(),
        );

        assert_eq!(provider.id(), ProviderId::Ollama);
        assert_eq!(provider.name(), "Ollama");
        assert_eq!(provider.cost_per_token(), 0.0);
    }

    #[tokio::test]
    async fn test_health_check_service_not_running() {
        let provider = OllamaProvider::new(
            "http://localhost:99999".to_string(), // Invalid port
            "codellama:7b".to_string(),
        );

        let health = provider.health_check().await.unwrap();
        assert!(matches!(health, HealthStatus::Unhealthy { .. }));
    }

    #[test]
    fn test_convert_options() {
        let provider = OllamaProvider::new(
            "http://localhost:11434".to_string(),
            "codellama:7b".to_string(),
        );

        let options = GenerationOptions {
            max_tokens: Some(1000),
            temperature: Some(0.7),
            top_p: Some (0.9),
            stop: Some(vec!["```"].to_string()),
            ..Default::default()
        };

        let ollama_options = provider.convert_options(&options);
        assert_eq!(ollama_options.num_predict, Some(1000));
        assert_eq!(ollama_options.temperature, Some(0.7));
        assert_eq!(ollama_options.top_p, Some(0.9));
        assert_eq!(ollama_options.stop, Some(vec!["```"].to_string()));
    }
}