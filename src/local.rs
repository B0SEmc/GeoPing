use crate::cli::{Cli, PingArgs};
use crate::formatter::{PingResponse, PingStatus, print_response};
use crate::ip::parse_target;
use crate::stats;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

pub async fn run_local_ping(cli: &Cli, target: &str) {
    let (host, parsed_port) = match parse_target(target) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let mut protocol = cli.protocol.clone();
    let port = if parsed_port != 0 {
        if protocol == "icmp" {
            protocol = "tcp".to_string();
        }
        Some(parsed_port)
    } else {
        Some(cli.port.unwrap_or(80))
    };

    let config = PingArgs {
        host,
        port,
        protocol,
        warmup: cli.warmup,
        count: cli.count,
        timeout: cli.timeout,
        silent: cli.silent,
        ipv4: cli.ipv4,
        ipv6: cli.ipv6,
    };

    let (ip_addr, socket_addr) = resolve_target(&config).await;

    let start_time = std::time::Instant::now();
    let durations = execute_ping_loop(&config, ip_addr, socket_addr).await;
    let total_time = start_time.elapsed();

    stats::print_stats(&config.host, &durations, total_time);
}

async fn resolve_target(config: &PingArgs) -> (Option<IpAddr>, Option<SocketAddr>) {
    let mut ip_addr = None;
    let mut socket_addr = None;

    match config.protocol.as_str() {
        "icmp" => {
            let sa = crate::ip::resolve_host(&config.host, 0, config.ipv4, config.ipv6)
                .await
                .unwrap_or_else(|| {
                    eprintln!("Error: Could not resolve host {}", config.host);
                    std::process::exit(1);
                });
            ip_addr = Some(sa.ip());
            println!(
                "Locally Pinging {} ({}) using protocol ICMP",
                config.host,
                sa.ip()
            );
        }
        "tcp" | "udp" => {
            let port = config.port.unwrap_or(80);
            let sa = crate::ip::resolve_host(&config.host, port, config.ipv4, config.ipv6)
                .await
                .unwrap_or_else(|| {
                    eprintln!("Error: Could not resolve host {}:{}", config.host, port);
                    std::process::exit(1);
                });
            socket_addr = Some(sa);
            println!(
                "Locally Pinging {} ({}) using protocol {}",
                config.host,
                sa,
                config.protocol.to_uppercase()
            );
        }
        p => {
            eprintln!("Error: Unsupported protocol: {}", p);
            std::process::exit(1);
        }
    }

    (ip_addr, socket_addr)
}

async fn execute_ping_loop(
    config: &PingArgs,
    ip_addr: Option<IpAddr>,
    socket_addr: Option<SocketAddr>,
) -> Vec<Option<Duration>> {
    let mut durations = Vec::new();
    let mut count = 0;
    let mut warmup_count = 0;

    loop {
        let is_warmup = if let Some(w) = config.warmup {
            warmup_count < w
        } else {
            false
        };

        if !is_warmup && config.count.is_some_and(|max| count >= max) {
            break;
        }

        tokio::select! {
            _ = async {
                let status = if config.protocol == "tcp" {
                    crate::tcp::ping_tcp(socket_addr.unwrap()).await
                } else if config.protocol == "udp" {
                    crate::udp::ping_udp(socket_addr.unwrap()).await
                } else {
                    crate::icmp::ping_icmp(ip_addr.unwrap()).await
                };

                if is_warmup {
                    warmup_count += 1;
                } else {
                    count += 1;
                    let dur_opt = match &status {
                        PingStatus::Success { elapsed } => Some(*elapsed),
                        _ => None,
                    };
                    durations.push(dur_opt);

                    print_response(&PingResponse {
                        ip: ip_addr.unwrap_or_else(|| socket_addr.unwrap().ip()),
                        port: if config.protocol == "tcp" || config.protocol == "udp" { config.port.unwrap_or(80) } else { 0 },
                        status,
                    });
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(config.timeout.unwrap_or(1000))).await;
            } => {}
            _ = tokio::signal::ctrl_c() => {
                break;
            }
        }
    }

    durations
}
