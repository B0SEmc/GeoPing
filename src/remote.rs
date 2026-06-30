use crate::cli::{Cli, PingArgs};
use crate::formatter::{print_response, PingResponse, PingStatus};
use crate::ip::parse_target;
use reqwest::Client;
use reqwest_eventsource::Event;
use std::collections::HashMap;
use std::fs;
use tokio_stream::StreamExt;

#[derive(serde::Deserialize, Debug)]
pub struct RemoteServer {
    pub name: String,
    pub url: String,
    pub token: Option<String>,
}

fn load_remotes(config_path: &str) -> Vec<RemoteServer> {
    let content = fs::read_to_string(config_path).unwrap_or_else(|e| {
        eprintln!("Error reading config file '{}': {}", config_path, e);
        std::process::exit(1);
    });

    let parsed: HashMap<String, Vec<RemoteServer>> = toml::from_str(&content).unwrap_or_else(|e| {
        eprintln!("Error parsing config file: {}", e);
        std::process::exit(1);
    });

    let remotes = parsed.into_values().next().unwrap_or_default();
    if remotes.is_empty() {
        eprintln!(
            "No remotes configured in '{}'. Please add [[remotes]] blocks.",
            config_path
        );
        std::process::exit(1);
    }

    remotes
}

fn build_ping_args(cli: &Cli) -> PingArgs {
    let target = cli.target.as_ref().expect("Target is required");
    let (host, parsed_port) = parse_target(target).unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    });
    
    let port = if parsed_port != 0 {
        parsed_port
    } else {
        cli.port.unwrap_or(80)
    };

    PingArgs {
        host,
        port: Some(port),
        protocol: cli.protocol.clone(),
        warmup: cli.warmup,
        count: cli.count,
        silent: cli.silent,
    }
}

fn build_streams(
    client: &Client,
    remotes: &[RemoteServer],
    ping_args: &PingArgs,
) -> Vec<reqwest_eventsource::EventSource> {
    let mut streams = Vec::new();

    for remote in remotes {
        let mut req = client.post(&remote.url).json(ping_args);
        if let Some(token) = &remote.token {
            req = req.bearer_auth(token);
        }

        let source = reqwest_eventsource::EventSource::new(req).unwrap();
        streams.push(source);
    }

    streams
}

fn print_headers(remotes: &[RemoteServer]) {
    let mut header = String::new();
    for remote in remotes {
        header.push_str(&format!("{:<15} | ", remote.name));
    }
    println!("{}", header);
    println!("{}", "-".repeat(header.len()));
}

async fn run_event_loop(
    remotes: &[RemoteServer],
    mut streams: Vec<reqwest_eventsource::EventSource>,
    ping_args: &PingArgs,
) {
    let is_single = remotes.len() == 1;

    loop {
        let mut futures = Vec::new();
        for stream in &mut streams {
            futures.push(stream.next());
        }

        tokio::select! {
            results = futures_util::future::join_all(futures) => {
                let mut all_closed = true;
                let mut row_output = String::new();

                for (i, result) in results.into_iter().enumerate() {
                    let remote = &remotes[i];
                    match result {
                        Some(Ok(Event::Message(msg))) => {
                            all_closed = false;
                            if let Ok(status) = serde_json::from_str::<PingStatus>(&msg.data) {
                                if is_single {
                                    print_response(&PingResponse {
                                        ip: "127.0.0.1".parse().unwrap(),
                                        port: ping_args.port.unwrap_or(80),
                                        status,
                                    });
                                } else {
                                    let text = match status {
                                        PingStatus::Success { elapsed } => format!("{:.2}ms", elapsed.as_secs_f64() * 1000.0),
                                        PingStatus::Timeout => "Timeout".to_string(),
                                        PingStatus::Error(_) => "Error".to_string(),
                                    };
                                    row_output.push_str(&format!("{:<15} | ", text));
                                }
                            } else {
                                if !is_single {
                                    row_output.push_str(&format!("{:<15} | ", "ParseErr"));
                                }
                            }
                        }
                        Some(Ok(Event::Open)) => {
                            all_closed = false;
                            if !is_single {
                                row_output.push_str(&format!("{:<15} | ", "Connected"));
                            }
                        }
                        Some(Err(reqwest_eventsource::Error::StreamEnded)) => {
                            if !is_single {
                                row_output.push_str(&format!("{:<15} | ", "Ended"));
                            }
                        }
                        Some(Err(_)) => {
                            if !is_single {
                                row_output.push_str(&format!("{:<15} | ", "ConnErr"));
                            } else {
                                eprintln!("Connection error with remote {}", remote.name);
                            }
                        }
                        None => {
                            if !is_single {
                                row_output.push_str(&format!("{:<15} | ", "Closed"));
                            }
                        }
                    }
                }

                if all_closed {
                    break;
                }

                if !is_single {
                    println!("{}", row_output);
                }
            }
            _ = tokio::signal::ctrl_c() => {
                break;
            }
        }
    }
}

pub async fn run_remote_ping(cli: &Cli, config_path: &str) {
    let remotes = load_remotes(config_path);
    let ping_args = build_ping_args(cli);
    
    let client = Client::new();
    let streams = build_streams(&client, &remotes, &ping_args);

    if remotes.len() > 1 {
        print_headers(&remotes);
    }

    run_event_loop(&remotes, streams, &ping_args).await;
}
