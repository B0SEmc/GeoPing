use surge_ping::{Client, Config, PingIdentifier, PingSequence};
use tokio::time::{Duration, timeout};

use crate::formatter::PingStatus;

pub async fn ping_icmp(addr: std::net::IpAddr) -> PingStatus {
    let client_config = match addr {
        std::net::IpAddr::V4(_) => Config::default(),
        std::net::IpAddr::V6(_) => Config::builder().kind(surge_ping::ICMP::V6).build(),
    };

    let client = match Client::new(&client_config) {
        Ok(c) => c,
        Err(e) => return PingStatus::Error(e.to_string()),
    };

    let mut pinger = client
        .pinger(addr, PingIdentifier(std::process::id() as u16))
        .await;
    pinger.timeout(Duration::from_secs(5));

    let payload = [0u8; 56];
    match timeout(
        Duration::from_secs(5),
        pinger.ping(PingSequence(0), &payload),
    )
    .await
    {
        Ok(Ok((_, elapsed))) => PingStatus::Success { elapsed },
        Ok(Err(e)) => match e {
            surge_ping::SurgeError::Timeout { .. } => PingStatus::Timeout,
            _ => PingStatus::Error(e.to_string()),
        },
        Err(_) => PingStatus::Timeout,
    }
}
