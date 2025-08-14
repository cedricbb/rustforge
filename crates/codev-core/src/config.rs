#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub default_provider: String,
    pub fallback_chain: Vec<String>,
    pub providers: HashMap<String, ProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub enabled: bool,
    pub model: String,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
    // API keys loaded from environment variables only
}

impl LLMConfig {
    pub fn load_api_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();

        // Load from environment variables only (never config files)
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            keys.insert("openai".to_string(), key);
        }
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            keys.insert("claude".to_string(), key);
        }
        if let Ok(key) = std::env::var("MISTRAL_API_KEY") {
            keys.insert("mistral".to_string(), key);
        }
        if let Ok(key) = std::env::var("GOOGLE_API_KEY") {
            keys.insert("gemini".to_string(), key);
        }

        keys
    }
}