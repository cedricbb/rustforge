# Cargo.toml

```toml
[workspace]
members = [
    "crates/codev-core",
    "crates/codev-cli",
    "crates/codev-shared"
]

[workspace.dependencies]
tokio = { version = "1.0", features = ["full"] }
reqwest = { version = "0.11", features = ["json", "stream"] }
serde = { version = "1.0", features = ["derive"] }
clap = { version = "4.0", features = ["derive"] }
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
futures = "0.3"
```

# crates/codev-cli/src/commands/mod.rs

```rs
#[derive(Subcommand)]
enum Commands {
    Chat {
        message: String,
        #[arg(long, help = "LLM provider to use")]
        llm: Option<String>,
    },

    #[command(subcommand)]
    Llm(LlmCommands),
}

#[derive(Subcommand)]
enum LlmCommands {
    Status,
    List,
    Switch { provider: String },
    Benchmark,
}

async fn handle_llm_command(cmd: LlmCommands, manager: &mut LLMManager) -> Result<()> {
    match cmd {
        LlmCommands::Status => {
            println!("Current provider: {}", manager.current_provider());
            // Show health status of all providers
        }
        LlmCommands::Switch { provider } => {
            manager.switch_provider(&provider).await?;
        }
        LlmCommands::List => {
            for (name, provider) in manager.providers() {
                let status = if provider.is_available() { "✅" } else { "❌" };
                println!("{} {} - {}", status, name, provider.name());
            }
        }
        LlmCommands::Benchmark => {
            // Run benchmark across all available providers
        }
    }
    Ok(())
}
```

# crates/codev-cli/src/main.rs

```rs
use clap::{Parser, Subcommand};
use futures::StreamExt;
use std::io::{self, Write};

#[derive(Parser)]
#[command(name = "codev")]
#[command(about = "CoDev.rs - AI Development Assistant")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Chat {
        #[arg(help = "Message to send to AI")]
        message: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Chat { message } => {
            let ollama = OllamaClient::new(
                "http://localhost:11434".to_string(),
                "codellama:7b".to_string()
            );

            let mut stream = ollama.stream_generate(&message).await?;

            print!("🤖 ");
            io::stdout().flush()?;

            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(text) => {
                        print!("{}", text);
                        io::stdout().flush()?;
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            println!(); // Newline at end
        }
    }

    Ok(())
}
```

# crates/codev-core/src/config.rs

```rs
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
```

# crates/codev-core/src/llm/claude.rs

```rs
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
```

# crates/codev-core/src/llm/gemini.rs

```rs
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
```

# crates/codev-core/src/llm/mistral.rs

```rs
pub struct MistralProvider {
    client: Client,
    api_key: String,
}

#[async_trait]
impl LLMProvider for MistralProvider {
    fn name(&self) -> &str { "mistral" }

    async fn stream_generate(&self, prompt: &str) -> Result<_> {
        // Mistral streaming implementation
    }
}
```

# crates/codev-core/src/llm/mod.rs

```rs
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
```

# crates/codev-core/src/llm/ollama.rs

```rs
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
pub struct OllamaResponse {
    response: String,
    done: bool,
}

pub struct OllamaClient {
    client: Client,
    endpoint: String,
    model: String,
}

impl OllamaClient {
    pub fn new(endpoint: String, model: String) -> Self {
        Self {
            client: Client::new(),
            endpoint,
            model,
        }
    }

    pub async fn stream_generate(&self, prompt: &str) -> Result<impl Stream<Item = Result<String, Box<dyn std::error::Error>>>> {
        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: true,
        };

        let response = self.client
            .post(&format!("{}/api/generate", self.endpoint))
            .json(&request)
            .send()
            .await?;

        Ok(response
            .bytes_stream()
            .map(|chunk| {
                chunk.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
                    .and_then(|bytes| {
                        let text = String::from_utf8(bytes.to_vec())?;
                        if let Ok(resp) = serde_json::from_str::<OllamaResponse>(&text) {
                            Ok(resp.response)
                        } else {
                            Ok(String::new())
                        }
                    })
            }))
    }
}
```

# crates/codev-core/src/llm/openai.rs

```rs
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
```

# docker-compose.yml

```yml
services:
  ollama:
    image: ollama/ollama
    container_name: codev-ollama
    ports:
      - "11434:11434"
    volumes:
      - ollama_data:/root/.ollama
    environment:
      - OLLAMA_KEEP_ALIVE=24h
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:11434/api/tags"]
      interval: 30s
      timeout: 10s
      retries: 5

  codev-agent:
    build: .
    container_name: codev-agent
    depends_on:
      ollama:
        condition: service_healthy
    volumes:
      - ./workspace:/app/workspace
      - /var/run/docker.sock:/var/run/docker.sock  # Pour exécution de commandes
    environment:
      - OLLAMA_ENDPOINT=http://ollama:11434
      - RUST_LOG=debug
    networks:
      - codev-network

networks:
  codev-network:
    driver: bridge

volumes:
  ollama_data:
```

# Dockerfile

```
FROM ubuntu:latest
LABEL authors="cedric"

ENTRYPOINT ["top", "-b"]
```

