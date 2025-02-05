use anyhow::Result;
use clap::{Parser, Subcommand};

mod server;

const DEFAULT_ADDRESS: &str = "127.0.0.1:42069";

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Subcommands,
}

#[derive(Subcommand)]
enum Subcommands {
    /// Run the chat server
    Serve(ServeArgs),
}

#[derive(Parser)]
struct CommonArgs {
    /// Address to connect to formatted as <host>:<port>
    #[clap(default_value = DEFAULT_ADDRESS)]
    address: String,
}

#[derive(Parser)]
struct ServeArgs {
    #[command(flatten)]
    common: CommonArgs,
}

impl Cli {
    async fn run(self) -> Result<()> {
        match self.command {
            Subcommands::Serve(args) => server::run(args).await?,
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();

    let cli = Cli::parse();
    cli.run().await?;

    Ok(())
}
