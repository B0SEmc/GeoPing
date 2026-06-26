mod cli;
mod formatter;
mod icmp;
mod ip;
mod local;
mod stats;
mod tcp;

use clap::Parser;
use cli::Cli;
use local::run_local_ping;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Some(target) = &cli.target {
        run_local_ping(&cli, target).await;
    } else {
        println!(
            "Error: No target specified.\nUse 'geoping --help' to see available commands."
        );
    }
}
