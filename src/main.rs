mod formatter;
mod icmp;
mod ip;
mod stats;
mod tcp;

use crate::icmp::ping_icmp;
use crate::ip::parse_target;
use crate::tcp::ping_tcp;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "geoping")]
#[command(about = "Advanced ping tool able to ping from multiple sources over ICMP, TCP, and UDP", long_about = None)]
struct Cli {
    target: Option<String>,

    #[arg(default_value = "80")]
    port: Option<u16>,

    #[arg(short = 'p', long, default_value = "icmp")]
    protocol: String,

    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short = 'w', long, default_value = "0")]
    warmup: usize,
}

#[derive(Subcommand)]
enum Commands {
    #[command(visible_alias = "s")]
    Server {
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
    },
    #[command(visible_aliases = ["distribute", "config", "m"])]
    Multi {
        #[arg(short, long, default_value = "config.toml")]
        config: String,
    },
}

pub struct PingArgs {
    pub host: String,
    pub port: u16,
    pub protocol: String,
    pub warmup: usize,
}

async fn run_local_ping(cli: &Cli, target: &str) {
    let (host, parsed_port) = match parse_target(target) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };
    let port = if parsed_port != 0 {
        parsed_port
    } else {
        cli.port.unwrap_or(80)
    };

    let config = PingArgs {
        host,
        port,
        protocol: cli.protocol.clone(),
        warmup: cli.warmup,
    };

    if let Ok(ip) = config.host.parse::<std::net::IpAddr>() {
        let socket_addr = std::net::SocketAddr::new(ip, config.port);
        println!(
            "Locally Pinging {} using protocol {}",
            socket_addr,
            config.protocol.to_uppercase()
        );
    } else {
        let resolved = tokio::net::lookup_host(format!("{}:{}", config.host, config.port))
            .await
            .ok()
            .and_then(|mut addrs| addrs.next());

        if let Some(addr) = resolved {
            println!(
                "Locally Pinging {} ({}) using protocol {}",
                config.host,
                addr,
                config.protocol.to_uppercase()
            );
        } else {
            println!(
                "Locally Pinging {}:{} using protocol {}",
                config.host,
                config.port,
                config.protocol.to_uppercase()
            );
        }
    }

    let start_time = std::time::Instant::now();
    let result = match &config.protocol as &str {
        "tcp" => ping_tcp(&config).await,
        "icmp" => ping_icmp(&config).await,
        _ => {
            println!("Error: Unsupported protocol: {}", config.protocol);
            std::process::exit(1);
        }
    };
    let total_time = start_time.elapsed();

    match result {
        Ok(durations) => {
            stats::print_stats(&config, &durations, total_time);
        }
        Err(e) => {
            eprintln!("Error during ping: {}", e);
        }
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Server { port }) => {
            println!("Starting Relay Server mode on port {}", port);
        }
        Some(Commands::Multi { config }) => {
            println!("Starting Orchestrator mode with file {}", config);
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
