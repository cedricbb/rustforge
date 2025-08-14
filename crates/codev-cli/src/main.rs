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
