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
