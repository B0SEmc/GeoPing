use std::time::Instant;
use tokio::net::TcpStream;
use tokio::time::{Duration, timeout};

use crate::formatter::PingStatus;

pub async fn ping_tcp(addr: std::net::SocketAddr) -> PingStatus {
    let start = Instant::now();
    match timeout(Duration::from_secs(2), TcpStream::connect(addr)).await {
        Ok(Ok(_)) => {
            let elapsed = start.elapsed();
            PingStatus::Success { elapsed }
        }
        Ok(Err(e)) => PingStatus::Error(e.to_string()),
        Err(_) => PingStatus::Timeout,
    }
}
