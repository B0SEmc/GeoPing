use crate::cli::{Cli, PingArgs};
use crate::formatter::{PingResponse, PingStatus, print_response};
use crate::icmp::ping_icmp;
use crate::ip::parse_target;
use crate::stats;
use crate::tcp::ping_tcp;

pub async fn run_local_ping(cli: &Cli, target: &str) {
    let (host, parsed_port) = match parse_target(target) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };
    let port = if parsed_port != 0 {
        Some(parsed_port)
    } else {
        Some(cli.port.unwrap_or(80))
    };

    let config = PingArgs {
        host,
        port,
        protocol: cli.protocol.clone(),
        warmup: cli.warmup,
        count: cli.count,
        silent: cli.silent,
    };

    let mut ip_addr = None;
    let mut socket_addr = None;

    match config.protocol.as_str() {
        "icmp" => {
            let addrs = tokio::net::lookup_host(format!("{}:0", config.host)).await;
            let ip = addrs
                .ok()
                .and_then(|mut a| a.next())
                .map(|a| a.ip())
                .unwrap_or_else(|| {
                    eprintln!("Error: Could not resolve host {}", config.host);
                    std::process::exit(1);
                });
            ip_addr = Some(ip);
            println!(
                "Locally Pinging {} ({}) using protocol ICMP",
                config.host, ip
            );
        }
        "tcp" => {
            let addrs =
                tokio::net::lookup_host(format!("{}:{}", config.host, config.port.unwrap_or(80)))
                    .await;
            let sa = addrs.ok().and_then(|mut a| a.next()).unwrap_or_else(|| {
                eprintln!(
                    "Error: Could not resolve host {}:{}",
                    config.host,
                    config.port.unwrap()
                );
                std::process::exit(1);
            });
            socket_addr = Some(sa);
            println!(
                "Locally Pinging {} ({}) using protocol TCP",
                config.host, sa
            );
        }
        p => {
            eprintln!("Error: Unsupported protocol: {}", p);
            std::process::exit(1);
        }
    }

    let start_time = std::time::Instant::now();
    let mut durations: Vec<Option<std::time::Duration>> = Vec::new();
    let mut count = 0;

    loop {
        if let Some(max) = config.count
            && count >= max
        {
            break;
        }
        count += 1;

        tokio::select! {
            _ = async {
                let status = if config.protocol == "tcp" {
                    ping_tcp(socket_addr.unwrap()).await
                } else {
                    ping_icmp(ip_addr.unwrap()).await
                };

                let dur_opt = match &status {
                    PingStatus::Success { elapsed } => Some(*elapsed),
                    _ => None,
                };
                durations.push(dur_opt);
                print_response(&PingResponse {
                    ip: ip_addr.unwrap_or_else(|| socket_addr.unwrap().ip()),
                    port: if config.protocol == "tcp" { config.port.unwrap_or(80) } else { 0 },
                    status,
                });
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            } => {}
            _ = tokio::signal::ctrl_c() => {
                break;
            }
        }
    }

    let total_time = start_time.elapsed();
    stats::print_stats(&config, &durations, total_time);
}
