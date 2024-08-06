// config.rs
use clap::{Parser, Arg};

/// Command-line arguments for the program.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Address of the node to connect to.
    #[arg(short, long, default_value = "79.56.220.96:8333")]
    pub remote_address: String,

    /// Local address of this node.
    #[arg(short, long, default_value = "0.0.0.0:8333")]
    pub local_address: String,

    /// Maximum number of peer addresses to collect.
    #[arg(long, default_value_t = 50)]
    pub address_limit: usize,

    /// Connection timeout duration in seconds.
    #[arg(long, default_value_t = 10)]
    pub connection_timeout: u64,

    /// User agent string to use in the version message.
    #[arg(long, default_value = "/Satoshi:25.0.0/")]
    pub user_agent: String,
}
