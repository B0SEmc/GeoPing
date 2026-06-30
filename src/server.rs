use crate::cli::PingArgs;
use crate::{icmp::ping_icmp, tcp::ping_tcp};
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

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    println!("Relay Server listening on port {}", port);
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

    let ip_addr = if config.protocol == "icmp" {
        if let Ok(ip) = config.host.parse::<std::net::IpAddr>() {
            Some(ip)
        } else {
            match tokio::net::lookup_host(format!("{}:0", config.host)).await {
                Ok(mut addrs) => addrs.next().map(|a| a.ip()),
                Err(_) => None,
            }
        }
    } else {
        None
    };

    let socket_addr = if config.protocol == "tcp" {
        if let Ok(ip) = config.host.parse::<std::net::IpAddr>() {
            Some(std::net::SocketAddr::new(ip, config.port.unwrap_or(80)))
        } else {
            match tokio::net::lookup_host(format!("{}:{}", config.host, config.port.unwrap_or(80)))
                .await
            {
                Ok(mut addrs) => addrs.next(),
                Err(_) => None,
            }
        }
    } else {
        None
    };

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
                ping_tcp(socket_addr.unwrap()).await
            } else {
                ping_icmp(ip_addr.unwrap()).await
            };

            yield Event::default()
                .json_data(&status)
                .unwrap_or_else(|_| Event::default().data("error"));
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    };

    Ok(Sse::new(stream.map(Ok::<_, Infallible>)))
}
