use std::time::Instant;
use tokio::net::TcpStream;
use tokio::signal;
use tokio::time::{Duration, sleep, timeout};

use crate::PingArgs;
use crate::formatter::{PingResponse, PingStatus, print_response};

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
                        print_response(&PingResponse {
                            ip: addr.ip(),
                            port: config.port,
                            status: PingStatus::Success { elapsed },
                        });
                    }
                    Ok(Err(e)) => {
                        durations.push(None);
                        print_response(&PingResponse {
                            ip: addr.ip(),
                            port: config.port,
                            status: PingStatus::Error(e.to_string()),
                        });
                    }
                    Err(_) => {
                        durations.push(None);
                        print_response(&PingResponse {
                            ip: addr.ip(),
                            port: config.port,
                            status: PingStatus::Timeout,
                        });
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
