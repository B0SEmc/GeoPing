use crate::cli::Cli;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Deserialize, Debug)]
pub struct RemoteServer {
    pub name: String,
    pub url: String,
    pub token: Option<String>,
}

pub async fn run_remote_ping(cli: &Cli, config_path: &str) {
    let content = match fs::read_to_string(config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading config file '{}': {}", config_path, e);
            std::process::exit(1);
        }
    };

    let parsed: HashMap<String, Vec<RemoteServer>> = match toml::from_str(&content) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error parsing config file: {}", e);
            std::process::exit(1);
        }
    };

    let remotes = parsed.into_values().next().unwrap_or_default();
    if remotes.is_empty() {
        eprintln!("No remotes configured in '{}'. Please add [[remotes]] blocks.", config_path);
        std::process::exit(1);
    }

    println!("Loaded {} remote servers from config.", remotes.len());
    // ... we will implement the pinging logic in the next phase
}
