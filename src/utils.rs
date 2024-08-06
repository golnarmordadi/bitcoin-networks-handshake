// utils.rs
use tracing::level_filters::LevelFilter;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use std::net::{SocketAddr, IpAddr};
use bitcoin::p2p::{Address, ServiceFlags};

/// Initialize logging and tracing for debugging.
pub fn init_tracing() {
    let env = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .with_env_var("RUST_LOG")
        .from_env_lossy();

    let fmt_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(false)
        .with_target(false);
    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(env)
        .init();
}

/// Check if the address is IPv4.
pub fn is_ipv4(address: &Address) -> bool {
    matches!(address.socket_addr(), Ok(SocketAddr::V4(_)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use std::str::FromStr;
    use bitcoin::p2p::Address;

    #[test]
    fn test_init_tracing() {
        // Ensure that init_tracing does not panic
        init_tracing();
    }

    #[test]
    fn test_is_ipv4() {
        let ipv4_addr = Ipv4Addr::new(192, 168, 1, 1);
        let socket_addr_v4 = SocketAddr::new(IpAddr::V4(ipv4_addr), 8333);
        let ipv4_address = Address::new(&socket_addr_v4, ServiceFlags::NONE);

        assert!(is_ipv4(&ipv4_address));
    }
}
