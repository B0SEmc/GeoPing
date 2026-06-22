
# 🌍 Geoping

**Geoping** is an advanced, distributed command-line ping utility written in Rust. 

While the standard `ping` command is limited to ICMP and measures latency from a single machine, Geoping supports **ICMP, TCP, and UDP**. More importantly, it operates as a unified binary that can act as a local client, a remote listening relay, or an orchestrator. By firing simultaneous pings from multiple remote servers, it aggregates latency data to roughly estimate the geographical location of a target.

## ✨ Features

* **Multi-Protocol:** Ping targets using ICMP, TCP, or UDP.
* **Unified Binary:** A single executable that can run in three different modes (Standalone, Relay Server, or Orchestrator).
* **Distributed Pinging:** Coordinate multiple servers to ping a single target simultaneously.
* **Asynchronous & Fast:** Built on top of `tokio` for high-performance, non-blocking network requests.
* **Secure Relays:** Relay servers are protected by a shared secret token to prevent abuse.
* **Geo-Estimation:** Uses the speed of light in fiber optics to correlate latencies and estimate the target's physical area.

## 🚀 Installation

Ensure you have Rust and Cargo installed. Then, clone the repository and build the project:

```bash
git clone https://github.com/b0semc/geoping.git
cd geoping
cargo build --release

```

The executable will be available in `target/release/geoping`.

*(Note: Depending on your OS and how the ICMP protocol is implemented via `surge-ping`, you may need `sudo` or specific capabilities to run ICMP pings).*

## 🛠️ Usage

Geoping is designed to be highly versatile and operates in three distinct modes.

### 1. Local Mode (Standalone)

Use Geoping like a standard ping tool, but with your protocol of choice.

```bash
# Ping using TCP (default)
geoping 8.8.8.8 --protocol tcp

# Ping using UDP on a specific port
geoping google.com --protocol udp --port 443

```

### 2. Relay Mode (Server)

Deploy the binary on a remote VPS and start it as a background listening relay. It will wait for instructions from the orchestrator.

```bash
geoping -s
# Or
geoping --server

```

*Note: The relay will listen for HTTP/JSON requests on the port specified in your code/config.*

### 3. Orchestrator Mode (Distributed)

Run this from your local machine. Pass a configuration file containing your target and the list of your remote relay servers. Geoping will dispatch the requests asynchronously, gather the results, and print the geographical estimation.

```bash
geoping --config my_cluster.toml

```

## ⚙️ Configuration File (`config.toml`)

When running in Orchestrator mode, you need a TOML configuration file to define your setup.

```toml
[target]
host = "8.8.8.8"
protocol = "tcp"
port = 53

[auth]
# This token must match the one expected by your Relay servers
secret_token = "my_super_secret_password"

# Add as many relays as you want
[[server]]
host = "198.51.100.1"
port = 3000

[[server]]
host = "203.0.113.5"
port = 3000

```
