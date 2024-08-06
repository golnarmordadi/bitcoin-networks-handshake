// message.rs
use bitcoin::p2p::{Address, ServiceFlags};
use bitcoin::p2p::message_network::VersionMessage;
use std::net::SocketAddr;
use rand::Rng;

/// Build a version message for the Bitcoin protocol.
pub fn build_version_message(
    receiver_address: &SocketAddr,
    sender_address: &SocketAddr,
    user_agent: &str,
) -> VersionMessage {
    const START_HEIGHT: i32 = 0;
    const SERVICES: ServiceFlags = ServiceFlags::NONE;

    let sender = Address::new(sender_address, SERVICES);
    let timestamp = chrono::Utc::now().timestamp();
    let receiver = Address::new(receiver_address, SERVICES);
    let nonce = rand::thread_rng().gen();
    let user_agent = user_agent.to_string();

    VersionMessage::new(
        SERVICES,
        timestamp,
        receiver,
        sender,
        nonce,
        user_agent,
        START_HEIGHT,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_build_version_message() {
        let receiver_ip = Ipv4Addr::new(192, 168, 1, 1);
        let sender_ip = Ipv4Addr::new(192, 168, 1, 2);
        let receiver_address = SocketAddr::new(IpAddr::V4(receiver_ip), 8333);
        let sender_address = SocketAddr::new(IpAddr::V4(sender_ip), 8333);
        let user_agent = "/Satoshi:0.21.0/";

        let version_message = build_version_message(&receiver_address, &sender_address, user_agent);

        // Assuming 70001 is the actual default Bitcoin protocol version in this context
        const EXPECTED_VERSION: i32 = 70001;
        
        assert_eq!(version_message.services, ServiceFlags::NONE);
        assert_eq!(version_message.start_height, 0);
        assert_eq!(version_message.user_agent, user_agent);
        assert_eq!(version_message.receiver.services, ServiceFlags::NONE);
        assert_eq!(version_message.sender.services, ServiceFlags::NONE);
        assert!(version_message.timestamp > 0);  // Ensure timestamp is a positive value
        assert!(version_message.nonce > 0);  // Ensure nonce is a positive value
    }
}
