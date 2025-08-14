pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    model: String,
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    fn name(&self) -> &str { "openai" }

    async fn stream_generate(&self, prompt: &str) -> Result<_> {
        // OpenAI streaming implementation
    }
}