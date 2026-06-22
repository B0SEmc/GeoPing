use std::net::IpAddr;
use std::time::Duration;

// ANSI escape codes for terminal coloring
pub const COLOR_RESET: &str = "\x1b[0m";
pub const COLOR_BOLD: &str = "\x1b[1m";

pub const COLOR_RED: &str = "\x1b[31m";
pub const COLOR_GREEN: &str = "\x1b[32m";
pub const COLOR_YELLOW: &str = "\x1b[33m";
pub const COLOR_BLUE: &str = "\x1b[34m";
pub const COLOR_MAGENTA: &str = "\x1b[35m";
pub const COLOR_CYAN: &str = "\x1b[36m";

pub const COLOR_BRIGHT_RED: &str = "\x1b[91m";
pub const COLOR_BRIGHT_GREEN: &str = "\x1b[92m";
pub const COLOR_BRIGHT_BLUE: &str = "\x1b[94m";
pub const COLOR_BRIGHT_MAGENTA: &str = "\x1b[95m";

#[derive(Debug, Clone)]
pub enum PingStatus {
    Success { elapsed: Duration },
    Timeout,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct PingResponse {
    pub ip: IpAddr,
    pub port: u16,
    pub status: PingStatus,
}

pub fn print_response(response: &PingResponse) {
    let dest = if response.port == 0 {
        format!("{}", response.ip)
    } else {
        format!("{}:{}", response.ip, response.port)
    };

    match &response.status {
        PingStatus::Success { elapsed } => {
            let ms = elapsed.as_secs_f64() * 1000.0;
            let latency_color = if ms < 50.0 {
                COLOR_GREEN
            } else if ms < 100.0 {
                COLOR_YELLOW
            } else if ms < 150.0 {
                COLOR_RED
            } else {
                COLOR_MAGENTA
            };
            println!(
                "Reply from {}{}{} time={}{:.1?}{}",
                COLOR_CYAN, dest, COLOR_RESET, latency_color, elapsed, COLOR_RESET
            );
        }
        PingStatus::Error(e) => {
            println!(
                "{}Error connecting{} to {}{}{}: {}",
                COLOR_RED, COLOR_RESET, COLOR_CYAN, dest, COLOR_RESET, e
            );
        }
        PingStatus::Timeout => {
            println!(
                "{}{}Request timeout{}{} for {}{}{}",
                COLOR_BOLD, COLOR_RED, COLOR_RESET, COLOR_RESET, COLOR_CYAN, dest, COLOR_RESET
            );
        }
    }
}
