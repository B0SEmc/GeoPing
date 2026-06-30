use crate::cli::{Cli, PingArgs};
use crate::formatter::{PingResponse, PingStatus, print_response};
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

fn build_ping_args(cli: &Cli, target: &str) -> PingArgs {
    let (host, parsed_port) = parse_target(target).unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    });

    let mut protocol = cli.protocol.clone();
    let port = if parsed_port != 0 {
        if protocol == "icmp" {
            protocol = "tcp".to_string();
        }
        parsed_port
    } else {
        cli.port.unwrap_or(80)
    };

    PingArgs {
        host,
        port: Some(port),
        protocol,
        warmup: cli.warmup,
        count: cli.count,
        silent: cli.silent,
        ipv4: cli.ipv4,
        ipv6: cli.ipv6,
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
    let mut all_durations: Vec<Vec<Option<std::time::Duration>>> = vec![Vec::new(); remotes.len()];
    let start_time = std::time::Instant::now();

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
                                let dur_opt = match &status {
                                    PingStatus::Success { elapsed } => Some(*elapsed),
                                    _ => None,
                                };
                                all_durations[i].push(dur_opt);

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
                            all_durations[i].push(None);
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

    let total_time = start_time.elapsed();
    for (i, remote) in remotes.iter().enumerate() {
        let title = if is_single {
            ping_args.host.clone()
        } else {
            format!("{} from {}", ping_args.host, remote.name)
        };
        crate::stats::print_stats(&title, &all_durations[i], total_time);
    }
}

pub async fn run_remote_ping(cli: &Cli, config_path: &str, target: &str) {
    let remotes = load_remotes(config_path);
    let ping_args = build_ping_args(cli, target);


    let client = Client::new();
    let streams = build_streams(&client, &remotes, &ping_args);

    if remotes.len() > 1 {
        println!("Remotely Pinging {} using protocol {} with {} servers", ping_args.host, ping_args.protocol.to_uppercase(), remotes.len());
        print_headers(&remotes);
    } else {
        println!("Remotely Pinging {} using protocol {} via {}", ping_args.host, ping_args.protocol.to_uppercase(), remotes[0].name);
    }

    run_event_loop(&remotes, streams, &ping_args).await;
}
