use surge_ping::{Client, Config, PingIdentifier, PingSequence};
use tokio::signal;
use tokio::time::{Duration, sleep, timeout};

use crate::PingArgs;
use crate::formatter::{PingResponse, PingStatus, print_response};

pub async fn ping_icmp(config: &PingArgs) -> Result<Vec<Option<Duration>>, std::io::Error> {
    let mut durations: Vec<Option<Duration>> = Vec::new();
    let addr = tokio::net::lookup_host(format!("{}:0", config.host))
        .await?
        .next()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Could not resolve host"))?
        .ip();

    let client_config = match addr {
        std::net::IpAddr::V4(_) => Config::default(),
        std::net::IpAddr::V6(_) => Config::builder().kind(surge_ping::ICMP::V6).build(),
    };

    let client = Client::new(&client_config)?;
    let mut pinger = client
        .pinger(addr, PingIdentifier(std::process::id() as u16))
        .await;
    pinger.timeout(Duration::from_secs(5));
    let mut sequence = 0u16;

    loop {
        tokio::select! {
            _ = async {
                let payload = [0u8; 56];

                match timeout(Duration::from_secs(5), pinger.ping(PingSequence(sequence), &payload)).await {
                    Ok(Ok((_, elapsed))) => {
                        durations.push(Some(elapsed));
                        print_response(&PingResponse {
                            ip: addr,
                            port: 0,
                            status: PingStatus::Success { elapsed },
                        });
                    }
                    Ok(Err(e)) => {
                        durations.push(None);
                        let status = match e {
                            surge_ping::SurgeError::Timeout { .. } => PingStatus::Timeout,
                            _ => PingStatus::Error(e.to_string()),
                        };
                        print_response(&PingResponse {
                            ip: addr,
                            port: 0,
                            status,
                        });
                    }
                    Err(_) => {
                        durations.push(None);
                        print_response(&PingResponse {
                            ip: addr,
                            port: 0,
                            status: PingStatus::Timeout,
                        });
                    }
                }

                sequence = sequence.wrapping_add(1);
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
