use crate::cli::PingArgs;

use axum::http::HeaderMap;
use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use std::convert::Infallible;

use axum::response::IntoResponse;
use axum::response::sse::{Event, Sse};
use tokio_stream::StreamExt;
pub async fn run_server(port: u16, token: Option<String>) {
    let app = Router::new()
        .route("/ping", post(handle_ping))
        .with_state(token);

    let udp_port = port;
    tokio::spawn(async move {
        if let Ok(listener) = tokio::net::UdpSocket::bind(format!("0.0.0.0:{}", udp_port)).await {
            let mut buf = [0; 1024];
            loop {
                if let Ok((len, addr)) = listener.recv_from(&mut buf).await {
                    let _ = listener.send_to(&buf[..len], addr).await;
                }
            }
        }
    });

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    println!("Relay Server listening on port {} (TCP/UDP)", port);
    axum::serve(listener, app).await.unwrap();
}

async fn handle_ping(
    State(token): State<Option<String>>,
    headers: HeaderMap,
    Json(config): Json<PingArgs>,
) -> Result<impl IntoResponse, StatusCode> {
    if let Some(expected_token) = token {
        let auth_header = headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        if auth_header != format!("Bearer {}", expected_token) {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    if config.protocol != "tcp" && config.protocol != "icmp" {
        return Err(StatusCode::BAD_REQUEST);
    }

    let sa = crate::ip::resolve_host(
        &config.host,
        config.port.unwrap_or(0),
        config.ipv4,
        config.ipv6,
    ).await;

    let ip_addr = sa.map(|s| s.ip());
    let socket_addr = sa;

    if config.protocol == "icmp" && ip_addr.is_none() {
        return Err(StatusCode::BAD_REQUEST);
    }
    if config.protocol == "tcp" && socket_addr.is_none() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let stream = async_stream::stream! {
        let mut count = 0;
        loop {
            if let Some(max) = config.count && count >= max {
                break;
            }
            count += 1;

            let status = if config.protocol == "tcp" {
                crate::tcp::ping_tcp(socket_addr.unwrap()).await
            } else {
                crate::icmp::ping_icmp(ip_addr.unwrap()).await
            };

            yield Event::default()
                .json_data(&status)
                .unwrap_or_else(|_| Event::default().data("error"));
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    };

    Ok(Sse::new(stream.map(Ok::<_, Infallible>)))
}
