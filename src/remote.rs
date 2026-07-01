use crate::cli::{Cli, PingArgs};
use crate::formatter::{PingResponse, PingStatus, print_response};
use crate::ip::parse_target;
use reqwest::Client;
use reqwest_eventsource::Event;
use std::collections::HashMap;
use std::fs;
use tokio_stream::StreamExt;

use crate::geo::{GeoLocation, fetch_location, estimate_location, calculate_distance};

#[derive(serde::Deserialize, Debug)]
pub struct RemoteServer {
    pub name: String,
    pub url: String,
    pub token: Option<String>,
    #[serde(skip)]
    pub location: Option<GeoLocation>,
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
    if protocol == "udp" {
        eprintln!("Error: UDP protocol is only supported for local pings directly to a geoping server.");
        std::process::exit(1);
    }

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
        timeout: cli.timeout,
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

fn process_sse_event(
    result: Option<Result<Event, reqwest_eventsource::Error>>,
    remote: &RemoteServer,
    ping_args: &PingArgs,
    is_single: bool,
) -> (Option<String>, Option<Option<std::time::Duration>>, bool) {
    let mut row_output = None;
    let mut dur_to_push = None;
    let mut is_closed = false;

    match result {
        Some(Ok(Event::Message(msg))) => {
            if let Ok(status) = serde_json::from_str::<PingStatus>(&msg.data) {
                let dur_opt = match &status {
                    PingStatus::Success { elapsed } => Some(*elapsed),
                    _ => None,
                };
                dur_to_push = Some(dur_opt);

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
                    row_output = Some(format!("{:<15} | ", text));
                }
            } else {
                if !is_single {
                    row_output = Some(format!("{:<15} | ", "ParseErr"));
                }
            }
        }
        Some(Ok(Event::Open)) => {
            if !is_single {
                row_output = Some(format!("{:<15} | ", "Connected"));
            }
        }
        Some(Err(reqwest_eventsource::Error::StreamEnded)) => {
            is_closed = true;
            if !is_single {
                row_output = Some(format!("{:<15} | ", "Ended"));
            }
        }
        Some(Err(_)) => {
            is_closed = true;
            dur_to_push = Some(None);
            if !is_single {
                row_output = Some(format!("{:<15} | ", "ConnErr"));
            } else {
                eprintln!("Connection error with remote {}", remote.name);
            }
        }
        None => {
            is_closed = true;
            if !is_single {
                row_output = Some(format!("{:<15} | ", "Closed"));
            }
        }
    }

    (row_output, dur_to_push, is_closed)
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
                    let (text_opt, dur_opt, is_closed) = process_sse_event(result, &remotes[i], ping_args, is_single);

                    if !is_closed {
                        all_closed = false;
                    }

                    if let Some(text) = text_opt {
                        row_output.push_str(&text);
                    }

                    if let Some(dur) = dur_opt {
                        all_durations[i].push(dur);
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
    let mut estimation_data = Vec::new();

    for (i, remote) in remotes.iter().enumerate() {
        let title = if is_single {
            ping_args.host.clone()
        } else {
            let loc_str = if let Some(loc) = &remote.location {
                format!("{} ({}, {})", remote.name, loc.city, loc.country_code)
            } else {
                remote.name.clone()
            };
            format!("{} from {}", ping_args.host, loc_str)
        };
        crate::stats::print_stats(&title, &all_durations[i], total_time);

        if let Some(loc) = &remote.location {
            let mut sum = std::time::Duration::from_secs(0);
            let mut valid_count = 0;
            for d in all_durations[i].iter().flatten() {
                sum += *d;
                valid_count += 1;
            }
            if valid_count > 0 {
                let avg_rtt = sum / valid_count as u32;
                estimation_data.push((loc.clone(), avg_rtt));
                let dist = calculate_distance(avg_rtt);
                println!("Relay: {} -> Avg: {:.2}ms -> Est. Distance: {:.0} km", title, avg_rtt.as_secs_f64() * 1000.0, dist);
            }
        }
    }

    if !estimation_data.is_empty() {
        println!("\nGEO-ESTIMATION RESULTS:");
        println!("{}", "-".repeat(50));
        if let Some((est_lat, est_lon)) = estimate_location(&estimation_data) {
            println!("=> Target Estimated Location: {:.4}, {:.4}", est_lat, est_lon);
            println!("=> View on Map: https://maps.google.com/?q={:.4},{:.4}", est_lat, est_lon);
        } else {
            println!("=> Could not estimate location (insufficient data).");
        }
    }
}

pub async fn run_remote_ping(cli: &Cli, config_path: &str, target: &str) {
    let mut remotes = load_remotes(config_path);
    let ping_args = build_ping_args(cli, target);

    let mut location_futures = Vec::new();
    for remote in &remotes {
        location_futures.push(fetch_location(&remote.url));
    }
    
    let locations = futures_util::future::join_all(location_futures).await;
    for (i, loc) in locations.into_iter().enumerate() {
        remotes[i].location = loc;
    }


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
