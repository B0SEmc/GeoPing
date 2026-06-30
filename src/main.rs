mod cli;
mod formatter;
mod icmp;
mod ip;
mod local;
mod server;
mod stats;
mod tcp;

use clap::Parser;
use cli::{Cli, Commands};
use local::run_local_ping;
use server::run_server;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Server { port, token }) => {
            println!("Starting Relay Server mode on port {}", port);
            run_server(*port, token.clone()).await;
        }
        Some(Commands::Remote { config }) => {
            println!("Starting Remote mode with file {}", config);
        }
        None => {
            if let Some(target) = &cli.target {
                run_local_ping(&cli, target).await;
            } else {
                println!(
                    "Error: No target specified.\nUse 'geoping --help' to see available commands."
                );
            }
        }
    }
}
