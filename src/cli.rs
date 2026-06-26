use clap::Parser;

#[derive(Parser)]
#[command(name = "geoping")]
#[command(about = "Advanced ping tool able to ping from multiple sources over ICMP, TCP, and UDP", long_about = None)]
pub struct Cli {
    pub target: Option<String>,

    #[arg(default_value = "80")]
    pub port: Option<u16>,

    #[arg(short = 'p', long, default_value = "icmp")]
    pub protocol: String,

    #[arg(short = 'w', long, default_value = "0")]
    pub warmup: Option<usize>,
}

pub struct PingArgs {
    pub host: String,
    pub port: u16,
    pub protocol: String,
    pub warmup: Option<usize>,
    pub count: Option<usize>,
    pub silent: bool,
}
