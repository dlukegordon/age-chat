mod client;
mod common;
mod server;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

const DEFAULT_ADDRESS: &str = "0.0.0.0:42069";
const DEFAULT_KEY_FILE: &str = "key.txt";

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Subcommands,
}

#[derive(Subcommand)]
enum Subcommands {
    /// Run the chat server
    Serve(ServerArgs),
    /// Run the chat server
    Connect(ClientArgs),
}

#[derive(Parser)]
struct CommonArgs {
    /// Address to connect to formatted as <host>:<port>
    #[clap(default_value = DEFAULT_ADDRESS)]
    address: String,
}

#[derive(Parser)]
struct ServerArgs {
    #[command(flatten)]
    common: CommonArgs,
}

#[derive(Parser)]
struct ClientArgs {
    /// Key file to authenticate with
    #[clap(long, short = 'u', default_value = DEFAULT_KEY_FILE)]
    key_file: PathBuf,

    /// Recipient pubkey to chat with
    #[clap(long, short = 'r')]
    recipient: String,

    #[command(flatten)]
    common: CommonArgs,
}

impl Cli {
    async fn run(self) -> Result<()> {
        match self.command {
            Subcommands::Serve(args) => server::run(args).await?,
            Subcommands::Connect(args) => client::run(args).await?,
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.run().await?;
    Ok(())
}
