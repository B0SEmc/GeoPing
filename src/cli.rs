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

    #[arg(short = 'c', long)]
    pub count: Option<usize>,

    #[arg(short = 's', long)]
    pub silent: bool,

    #[arg(short = '4', long)]
    pub ipv4: bool,

    #[arg(short = '6', long)]
    pub ipv6: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(clap::Subcommand)]
pub enum Commands {
    #[command(visible_alias = "s")]
    Server {
        #[arg(short, long, default_value_t = 3000)]
        port: u16,

        #[arg(short = 't', long)]
        token: Option<String>,
    },
    #[command(visible_aliases = ["distribute", "config", "r"])]
    Remote {
        #[arg(short, long, default_value = "config.toml")]
        config: String,

        target: Option<String>,
    },
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct PingArgs {
    pub host: String,
    pub port: Option<u16>,
    pub protocol: String,
    pub warmup: Option<usize>,
    pub count: Option<usize>,
    #[serde(default)]
    pub silent: bool,
    #[serde(default)]
    pub ipv4: bool,
    #[serde(default)]
    pub ipv6: bool,
}
