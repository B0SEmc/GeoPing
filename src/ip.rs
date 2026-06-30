use std::net::SocketAddr;

pub fn parse_target(target: &str) -> Result<(String, u16), clap::Error> {
    let parts: Vec<&str> = target.split(':').collect();
    if parts.len() > 2 {
        return Err(clap::Error::new(clap::error::ErrorKind::InvalidValue));
    }
    if parts.len() == 1 {
        Ok((parts[0].to_string(), 0))
    } else {
        Ok((parts[0].to_string(), parts[1].parse().unwrap_or(0)))
    }
}

pub async fn resolve_host(host: &str, port: u16, ipv4: bool, ipv6: bool) -> Option<SocketAddr> {
    let addrs = tokio::net::lookup_host(format!("{}:{}", host, port)).await.ok()?;
    for addr in addrs {
        if ipv4 && addr.is_ipv6() {
            continue;
        }
        if ipv6 && addr.is_ipv4() {
            continue;
        }
        return Some(addr);
    }
    None
}
