use async_trait::async_trait;
use futures::Stream;

#[async_trait]
pub trait LLMProvider: Send + Sync {
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;
    async fn health_check(&self) -> Result<HealthStatus>;

    async fn stream_generate(
        &self,
        prompt: &str
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>>;

    async fn generate(&self, prompt: &str) -> Result<String>;
    fn max_context_length(&self) -> usize;
    fn cost_per_token(&self) -> f64;
}

#[derive(Debug, Clone)]
pub enum LLMProvider {
    Ollama(OllamaProvider),
    OpenAI(OpenAIProvider),
    Claude(ClaudeProvider),
    Mistral(MistralProvider),
    Gemini(GeminiProvider),
}

pub struct LLMManager {
    providers: HashMap<String, Box<dyn LLMProvider>>,
    current_provider: String,
    fallback_chain: Vec<String>,
    environment: Environment,
}

impl LLMManager {
    pub async fn auto_select_provider(&mut self) -> Result<String> {
        // 1. Try Ollama first if local environment
        if self.environment.is_local() {
            if let Some(ollama) = self.providers.get("ollama") {
                if ollama.is_available() {
                    return Ok("ollama".to_string());
                }
            }
        }

        // 2. Try user preferred provider
        if let Some(preferred) = self.get_user_preference() {
            if self.providers.get(&preferred).unwrap().is_available() {
                return Ok(preferred);
            }
        }

        // 3. Fallback chain
        for provider_name in &self.fallback_chain {
            if let Some(provider) = self.providers.get(provider_name) {
                if provider.health_check().await.is_ok() {
                    return Ok(provider_name.clone());
                }
            }
        }

        Err(LLMError::NoProviderAvailable)
    }

    pub async fn switch_provider(&mut self, name: &str) -> Result<()> {
        if !self.providers.contains_key(name) {
            return Err(LLMError::ProviderNotFound(name.to_string()));
        }

        // Test provider availability
        let provider = self.providers.get(name).unwrap();
        provider.health_check().await?;

        self.current_provider = name.to_string();
        println!("✅ Switched to provider: {}", name);
        Ok(())
    }
}