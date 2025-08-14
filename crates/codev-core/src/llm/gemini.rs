pub struct GeminiProvider {
    client: Client,
    api_key: String,
}

#[async_trait]
impl LLMProvider for GeminiProvider {
    fn name(&self) -> &str { "gemini" }

    async fn stream_generate(&self, prompt: &str) -> Result<_> {
        // Gemini streaming implementation
    }
}