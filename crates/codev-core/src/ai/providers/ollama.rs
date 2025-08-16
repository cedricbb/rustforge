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