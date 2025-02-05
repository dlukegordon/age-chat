use anyhow::Result;
use clap::{Parser, Subcommand};

mod server;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: &str = "42069";

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
    /// Host to connect to
    #[clap(short = 'H', long = "host", default_value = DEFAULT_HOST)]
    host: String,

    /// Port to connect to
    #[clap(short = 'p', long = "port", default_value = DEFAULT_PORT)]
    port: String,
}

#[derive(Parser)]
struct ServeArgs {
    #[command(flatten)]
    common: CommonArgs,
}

impl CommonArgs {
    fn address(self) -> String {
        format!("{}:{}", self.host, self.port)
    }
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
