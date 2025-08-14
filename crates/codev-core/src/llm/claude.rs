pub struct ClaudeProvider {
    client: Client,
    api_key: String,
}

#[async_trait]
impl LLMProvider for ClaudeProvider {
    fn name(&self) -> &str { "claude" }

    async fn stream_generate(&self, prompt: &str) -> Result<_> {
        // Claude streaming implementation
    }
}