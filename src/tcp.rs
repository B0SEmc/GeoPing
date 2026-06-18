use std::time::Instant;
use tokio::net::TcpStream;
use tokio::signal;
use tokio::time::{Duration, sleep, timeout};

use crate::PingArgs;

const COLOR_RESET: &str = "\x1b[0m";
const COLOR_BOLD: &str = "\x1b[1m";
const COLOR_GREEN: &str = "\x1b[32m";
const COLOR_YELLOW: &str = "\x1b[33m";
const COLOR_RED: &str = "\x1b[31m";
const COLOR_MAGENTA: &str = "\x1b[35m";
const COLOR_CYAN: &str = "\x1b[36m";

pub async fn ping_tcp(config: &PingArgs) -> Result<Vec<Option<Duration>>, std::io::Error> {
    let mut durations: Vec<Option<Duration>> = Vec::new();
    let addr = tokio::net::lookup_host(format!("{}:{}", config.host, config.port))
        .await?
        .next()
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Could not resolve host")
        })?;

    loop {
        tokio::select! {
            _ = async {
                let start = Instant::now();
                match timeout(Duration::from_secs(2), TcpStream::connect(addr)).await {
                    Ok(Ok(_)) => {
                        let elapsed = start.elapsed();
                        durations.push(Some(elapsed));
                        let ms = elapsed.as_secs_f64() * 1000.0;
                        let latency_color = if ms < 50.0 {
                            COLOR_GREEN
                        } else if ms < 100.0 {
                            COLOR_YELLOW
                        } else if ms < 150.0 {
                            COLOR_RED
                        } else {
                            COLOR_MAGENTA
                        };
                        println!(
                            "Reply from {}{}:{}{} time={}{:.1?}{}",
                            COLOR_CYAN,
                            addr.ip(),
                            config.port,
                            COLOR_RESET,
                            latency_color,
                            elapsed,
                            COLOR_RESET
                        );
                    }
                    Ok(Err(e)) => {
                        durations.push(None);
                        println!(
                            "{}Error connecting{} to {}{}:{}{}: {}",
                            COLOR_RED,
                            COLOR_RESET,
                            COLOR_CYAN,
                            addr.ip(),
                            config.port,
                            COLOR_RESET,
                            e
                        );
                    }
                    Err(_) => {
                        durations.push(None);
                        println!(
                            "{}{}Request timeout{}{} for {}{}:{}{}",
                            COLOR_BOLD,
                            COLOR_RED,
                            COLOR_RESET,
                            COLOR_RESET,
                            COLOR_CYAN,
                            addr.ip(),
                            config.port,
                            COLOR_RESET
                        );
                    }
                }
                sleep(Duration::from_secs(1)).await;
            } => {}
            result = signal::ctrl_c() => {
                match result {
                    Ok(()) => {
                        break;
                    }
                    Err(e) => {
                        eprintln!("Error listening for Ctrl-C: {}", e);
                        break;
                    }
                }
            }
        }
    }

    Ok(durations)
}
