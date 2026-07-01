use crate::formatter::PingStatus;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::timeout;

pub async fn ping_udp(target: SocketAddr) -> PingStatus {
    // Bind to an ephemeral port (let OS pick)
    let socket = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(s) => s,
        Err(e) => return PingStatus::Error(e.to_string()),
    };

    let start = std::time::Instant::now();
    let payload = b"geoping";

    if let Err(e) = socket.send_to(payload, target).await {
        return PingStatus::Error(e.to_string());
    }

    let mut buf = [0; 1024];
    match timeout(Duration::from_secs(2), socket.recv_from(&mut buf)).await {
        Ok(Ok((_len, _addr))) => PingStatus::Success {
            elapsed: start.elapsed(),
        },
        Ok(Err(e)) => PingStatus::Error(e.to_string()),
        Err(_) => PingStatus::Timeout,
    }
}
