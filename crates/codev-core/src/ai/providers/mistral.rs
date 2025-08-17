//! Mistral Provider Implementation (Stub)

use crate::ai::{GenerationOptions, HealthStatus, LlmProvider, ProviderCapabilities};
use async_trait::async_trait;
use codev_shared::{ProviderId, Result};
use futures::Stream;
use std::pin::Pin;

/// Mistral provider (stub implementation)
pub struct MistralProvider {
    api_key: String,
    model: String,
}

impl MistralProvider {
    pub fn new(api_key: String, model: String) -> Result<Self> {
        Ok(Self { api_key, model })
    }
}

#[async_trait]
impl LlmProvider for MistralProvider {
    fn id(&self) -> ProviderId { ProviderId::Mistral }

    fn name(&self) -> &str { "Mistral" }

    fn is_available(&self) -> bool { !self.api_key.is_empty() }

    async fn health_check(&self) -> Result<HealthStatus> {
        Ok(HealthStatus::Degraded {
            reason: "Mistral provider not yet implemented".to_string(),
        })
    }

    async fn stream_generate(
        &self,
        _prompt: &str,
        _options: &GenerationOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        todo!("Mistral streaming will be implemented in Phase 3")
    }

    async fn generate(&self, _prompt: &str, _options: GenerationOptions) -> Result<String> {
        todo!("Mistral generation will be implemented in Phase 3")
    }

    fn max_context_length(&self) -> usize { 32000 }

    fn cost_per_token(&self) -> f64 { 0.000007 }

    fn capabilities(&self) -> ProviderCapabilities { ProviderCapabilities::default() }
}