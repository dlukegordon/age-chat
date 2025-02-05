use anyhow::Result;
use clap::{Parser, Subcommand};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.run()?;
    Ok(())
}

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Subcommands,
}

#[derive(Subcommand)]
enum Subcommands {
    /// Run the chat server
    Serve,
}

impl Cli {
    fn run(self) -> Result<()> {
        match self.command {
            Subcommands::Serve => serve()?,
        }
        Ok(())
    }
}

fn serve() -> Result<()> {
    println!("Hello world!");
    Ok(())
}
