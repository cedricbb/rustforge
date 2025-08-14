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