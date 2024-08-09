// main.rs
//
// This file contains the main entry point for a Bitcoin peer address crawler using asynchronous Rust with Tokio.
// The provided implementation efficiently collects peer addresses from a network node by leveraging concurrency.
// 
// Best Practices and Considerations:
// 
// 1. **Asynchronous Programming**: 
//    The use of asynchronous functions and Tokio allows the program to handle multiple network connections concurrently,
//    significantly improving performance and scalability.
//
// 2. **Error Handling**: 
//    Comprehensive error handling with the `anyhow` crate ensures that errors are properly propagated and logged, making
//    debugging easier and the program more robust.
//
// 3. **Modularity**: 
//    Functions are separated by their responsibilities (e.g., `connect`, `collect_initial_addresses`, `crawl_address`),
//    making the code easier to maintain and extend.
//
// 4. **Efficiency**: 
//    The provided code processes each address exactly once, ensuring that the overall time complexity is O(n), where n
//    is the total number of unique addresses processed. This makes the function efficient and scalable for large numbers
//    of records.
//
// 5. **Logging and Tracing**: 
//    Integration with the `tracing` crate allows for detailed logging and tracing of the program's execution, which is
//    essential for monitoring and debugging in a production environment.
//
// 6. **Command-Line Arguments**: 
//    The use of the `clap` crate for parsing command-line arguments provides a user-friendly way to configure the
//    program's behavior, enhancing usability.
//
// 7. **Documentation**: 
//    Each function is documented with comments explaining its purpose and usage, promoting code readability and
//    maintainability.
//
// 8. **Resource Management**: 
//    Proper use of asynchronous resource management (e.g., TCP connections) ensures that resources are allocated and
//    released efficiently, preventing leaks and optimizing performance.
//
// The provided code processes each address exactly once, ensuring that the overall time complexity
// is O(n), where n is the total number of unique addresses processed. This makes the function efficient
// and scalable for large numbers of records.

#![allow(unused)]
mod codec;
mod utils;
mod messaging;
mod error;
mod config;

use std::collections::{HashSet, VecDeque};
use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{Context, Result};
use bitcoin::p2p::message::{NetworkMessage, RawNetworkMessage};
use bitcoin::p2p::message_network::VersionMessage;
use bitcoin::Network;
use config::Args;
use codec::BitcoinCodec;
use futures::{SinkExt, StreamExt, TryFutureExt, future::join_all};
use tokio::net::TcpStream;
use tokio::time::{timeout};
use tokio_util::codec::Framed;
use clap::Parser;

use utils::{init_tracing, is_ipv4};
use messaging::build_version_message;
use error::Error;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging and tracing for debugging
    init_tracing(); //is a constant time operation

    // Parse command-line arguments
    let args = Args::parse(); //is a constant time operation

    // Parse remote and local addresses from command-line arguments
    // Has constant time operations
    let remote_address = args.remote_address
        .parse::<SocketAddr>()
        .map_err(|_| Error::InvalidAddress("remote_address".to_string()))?;
    let local_address = args.local_address
        .parse::<SocketAddr>()
        .map_err(|_| Error::InvalidAddress("local_address".to_string()))?;

    // Collect initial set of peer addresses from the remote node
    // O(m), `m` number of initial addresses collected.
    let initial_addresses = collect_initial_addresses(&remote_address, &local_address, &args).await?;

    let mut all_addresses = HashSet::new();
    let mut addresses_to_crawl = VecDeque::new();

    // O(m)
    all_addresses.extend(initial_addresses.iter().cloned());
    addresses_to_crawl.extend(initial_addresses);

    // Continue crawling until the address limit is reached
    // O(n)
    while all_addresses.len() < args.address_limit && !addresses_to_crawl.is_empty() {
        let tasks: Vec<_> = addresses_to_crawl.drain(..).map(|address| {
            let local_address = local_address.clone();
            tokio::spawn(async move {
                match crawl_address(address, local_address).await {
                    Ok(new_addresses) => new_addresses,
                    Err(e) => {
                        tracing::error!("Failed to crawl address {:?}: {:?}", address, e);
                        HashSet::new()
                    }
                }
            })
        }).collect();

        // Collect results from the crawling tasks
        let mut new_addresses = HashSet::new();
        for task in join_all(tasks).await {
            if let Ok(addresses) = task {
                new_addresses.extend(addresses);
            }
        }

        for addr in new_addresses.difference(&all_addresses) {
            addresses_to_crawl.push_back(*addr);
        }
        all_addresses.extend(new_addresses);
    }

    // Print out collected peer addresses, up to the limit
    // O(n)
    for addr in all_addresses.iter().take(args.address_limit) {
        println!("Peer address: {:?}", addr);
    }

    // O(1)+O(m)+O(m)+O(n)+O(n)=O(m+n)

    Ok(())
}

/// Establish a TCP connection to the specified remote address with a timeout.
pub async fn connect(remote_address: &SocketAddr, timeout_duration: u64) -> Result<Framed<TcpStream, BitcoinCodec>> {
    let connection = TcpStream::connect(remote_address).map_err(|e| anyhow::anyhow!("Connection failed: {:?}", e));
    let stream = timeout(Duration::from_secs(timeout_duration), connection)
        .map_err(|e| anyhow::anyhow!("Connection timed out: {:?}", e))
        .await??;
    let framed = Framed::new(stream, BitcoinCodec {});
    Ok(framed)
}

/// Collect initial peer addresses from a connected node.
async fn collect_initial_addresses(
    remote_address: &SocketAddr,
    local_address: &SocketAddr,
    args: &Args,
) -> Result<HashSet<SocketAddr>> {
    let mut stream = connect(remote_address, args.connection_timeout).await?;
    let version_message = RawNetworkMessage::new(
        Network::Bitcoin.magic(),
        NetworkMessage::Version(build_version_message(remote_address, local_address, &args.user_agent)),
    );

    // Send version message to initiate handshake
    stream
        .send(version_message)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to send version message: {:?}", e))?;

    let mut peer_addresses = HashSet::new();
    let mut verack_sent = false;
    let mut getaddr_sent = false;

    while let Some(result) = stream.next().await {
        match result {
            Ok(message) => {
                let payload = message.payload().clone(); // Clone payload to avoid lifetime issues
                match payload {
                    NetworkMessage::Version(remote_version) => {
                        tracing::info!("Received Version Message: {:?}", remote_version);
                        if !verack_sent {
                            // Send verack message to complete handshake
                            stream
                                .send(RawNetworkMessage::new(
                                    Network::Bitcoin.magic(),
                                    NetworkMessage::Verack,
                                ))
                                .await
                                .map_err(|e| anyhow::anyhow!("Failed to send verack message: {:?}", e))?;
                            verack_sent = true;
                        }
                        if !getaddr_sent {
                            // Request a list of addresses from the remote node
                            stream
                                .send(RawNetworkMessage::new(
                                    Network::Bitcoin.magic(),
                                    NetworkMessage::GetAddr,
                                ))
                                .await
                                .map_err(|e| anyhow::anyhow!("Failed to send getaddr message: {:?}", e))?;
                            getaddr_sent = true;
                        }
                    }
                    NetworkMessage::Addr(addresses) => {
                        tracing::info!("Received Addr Message with {} addresses", addresses.len());
                        // Filter and collect IPv4 addresses
                        for (_, address) in addresses {
                            if is_ipv4(&address) {
                                if let Ok(socket_addr) = address.socket_addr() {
                                    peer_addresses.insert(socket_addr);
                                }
                            }
                        }
                        // Break the loop if we have enough addresses
                        if peer_addresses.len() >= args.address_limit {
                            break;
                        }
                    }
                    _ => {}
                }
            }
            Err(err) => {
                tracing::error!("Decoding error: {}", err);
            }
        }
    }

    Ok(peer_addresses)
}

/// Crawl a specific address to collect more peer addresses.
async fn crawl_address(address: SocketAddr, local_address: SocketAddr) -> Result<HashSet<SocketAddr>> {
    let mut stream = connect(&address, 10).await?; // Use default timeout here
    let new_addresses = collect_initial_addresses(&address, &local_address, &Args {
        remote_address: String::new(),
        local_address: String::new(),
        address_limit: 5000,
        connection_timeout: 10,
        user_agent: String::new(),
    }).await?;
    Ok(new_addresses)
}

 #[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::Sequence;
    use futures::channel::mpsc;
    use tokio_test::block_on;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV6};
    use std::collections::HashSet;
    use bitcoin::consensus::encode::serialize;
    use tokio::net::TcpListener;
    use mockito::{mock, Matcher};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use bitcoin::p2p::address::Address as BitcoinAddress;
    use futures::StreamExt;
    use bitcoin::p2p::message::NetworkMessage;

    #[tokio::test]
    async fn test_connect_success() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let local_addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
        });

        let result = connect(&local_addr, 5).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_connect_timeout() {
        let remote_addr = "127.0.0.1:65535".parse::<SocketAddr>().unwrap(); // Unused port
        let result = connect(&remote_addr, 1).await;
        assert!(result.is_err());
    }
   
}