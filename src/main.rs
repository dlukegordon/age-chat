mod client;
mod common;
mod server;

use anyhow::Result;
use clap::{Parser, Subcommand};

const DEFAULT_ADDRESS: &str = "127.0.0.1:42069";

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
    /// Username to authenticate as
    #[clap(long, short = 'u', default_value_t = get_current_user())]
    username: String,

    /// Recipient to chat with
    #[clap(long, short = 'r', default_value_t = get_current_user())]
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

fn get_current_user() -> String {
    std::env::var("USER").unwrap()
}
