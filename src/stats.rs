use crate::PingArgs;
use std::time::Duration;

use crate::formatter::{
    COLOR_RESET, COLOR_BOLD, COLOR_BLUE, COLOR_GREEN, COLOR_YELLOW, COLOR_RED, COLOR_MAGENTA,
    COLOR_BRIGHT_RED, COLOR_BRIGHT_GREEN, COLOR_BRIGHT_BLUE, COLOR_BRIGHT_MAGENTA,
};


struct RttStats {
    min: f64,
    max: f64,
    avg: f64,
    mdev: f64,
}

struct JitterStats {
    p5: f64,
    p95: f64,
    jitter: f64,
}

pub fn print_stats(config: &PingArgs, durations: &[Option<Duration>], total_time: Duration) {
    let transmitted = durations.len();
    if transmitted == 0 {
        println!("\n--- {} ping statistics ---", config.host);
        println!("0 packets transmitted, 0 received");
        return;
    }

    let (received, latencies) = get_latencies(durations);
    let loss = calc_loss(transmitted, received);

    print_summary(
        config.host.as_str(),
        transmitted,
        received,
        loss,
        total_time,
    );

    if received > 0 {
        let mut sorted_latencies = latencies;
        sorted_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let rtt = calc_rtt_stats(&sorted_latencies);
        let jitter = calc_jitter_stats(&sorted_latencies);

        print_rtt_stats(&rtt);
        print_jitter_stats(&jitter);
    }
}

fn get_latencies(durations: &[Option<Duration>]) -> (usize, Vec<f64>) {
    let mut received = 0;
    let mut latencies = Vec::new();
    for dur in durations.iter().flatten() {
        received += 1;
        latencies.push(dur.as_secs_f64() * 1000.0);
    }
    (received, latencies)
}

fn calc_loss(transmitted: usize, received: usize) -> f64 {
    if transmitted > 0 {
        ((transmitted - received) as f64 / transmitted as f64) * 100.0
    } else {
        0.0
    }
}

fn calc_rtt_stats(latencies: &[f64]) -> RttStats {
    let min = latencies[0];
    let max = latencies[latencies.len() - 1];
    let sum: f64 = latencies.iter().sum();
    let avg = sum / latencies.len() as f64;

    let variance: f64 =
        latencies.iter().map(|&x| (x - avg).powi(2)).sum::<f64>() / latencies.len() as f64;
    let mdev = variance.sqrt();

    RttStats {
        min,
        max,
        avg,
        mdev,
    }
}

fn calc_percentile(sorted_data: &[f64], p: f64) -> f64 {
    let n = sorted_data.len();
    if n == 1 {
        return sorted_data[0];
    }
    let i = p * (n - 1) as f64;
    let lower = i.floor() as usize;
    let upper = i.ceil() as usize;
    let frac = i - i.floor();
    sorted_data[lower] + frac * (sorted_data[upper] - sorted_data[lower])
}

fn calc_jitter_stats(latencies: &[f64]) -> JitterStats {
    let p5 = calc_percentile(latencies, 0.05);
    let p95 = calc_percentile(latencies, 0.95);
    let jitter = p95 - p5;
    JitterStats { p5, p95, jitter }
}

fn print_summary(host: &str, transmitted: usize, received: usize, loss: f64, total_time: Duration) {
    println!("\n--- {} ping statistics ---", host);

    let loss_color = if loss == 0.0 {
        COLOR_GREEN
    } else if loss < 20.0 {
        COLOR_YELLOW
    } else {
        COLOR_RED
    };

    println!(
        "{}{}{} packets transmitted, {}{}{} received, {}{:.1}% packet loss{}, time {}{}{}ms",
        COLOR_BRIGHT_BLUE,
        transmitted,
        COLOR_RESET,
        COLOR_BRIGHT_GREEN,
        received,
        COLOR_RESET,
        loss_color,
        loss,
        COLOR_RESET,
        COLOR_BRIGHT_MAGENTA,
        total_time.as_millis(),
        COLOR_RESET
    );
}

fn print_rtt_stats(rtt: &RttStats) {
    println!(
        "rtt {}{}min/avg/max/mdev{}{} = {}{:.3}{}/{}{:.3}{}/{}{:.3}{}/{}{:.3}{} ms",
        COLOR_BOLD,
        COLOR_RESET,
        COLOR_BOLD,
        COLOR_RESET,
        COLOR_GREEN,
        rtt.min,
        COLOR_RESET,
        COLOR_BLUE,
        rtt.avg,
        COLOR_RESET,
        COLOR_RED,
        rtt.max,
        COLOR_RESET,
        COLOR_YELLOW,
        rtt.mdev,
        COLOR_RESET
    );
}

fn print_jitter_stats(jitter: &JitterStats) {
    let jitter_color = if jitter.jitter <= 5.0 {
        COLOR_BRIGHT_GREEN
    } else if jitter.jitter <= 10.0 {
        COLOR_YELLOW
    } else if jitter.jitter <= 20.0 {
        COLOR_RED
    } else {
        COLOR_MAGENTA
    };

    println!(
        "jitter {}{}(p95-p5){}{} = {}{:.3}{} ms (p5: {}{:.3}{} ms, p95: {}{:.3}{} ms)",
        COLOR_BOLD,
        COLOR_RESET,
        COLOR_BOLD,
        COLOR_RESET,
        jitter_color,
        jitter.jitter,
        COLOR_RESET,
        COLOR_BRIGHT_RED,
        jitter.p5,
        COLOR_RESET,
        COLOR_GREEN,
        jitter.p95,
        COLOR_RESET
    );
}
