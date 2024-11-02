//! Integration tests for the BitTorrent peer protocol implementation.
//!
//! # Test Coverage
//!
//! ## Protocol Messages
//! - Message serialization/deserialization for all message types
//! - Keep-alive message handling
//! - Bitfield exchange
//! - Choke/unchoke states
//!
//! ## Piece Transfer
//! - Single piece download
//! - Multiple piece requests
//! - Piece data verification
//!
//! ## Error Handling
//! - Malformed messages
//! - Connection timeouts
//! - Protocol violations
//!
//! # Test Structure
//!
//! Tests use a `MockPeer` to simulate a BitTorrent peer, which:
//! - Accepts incoming connections
//! - Handles handshake protocol
//! - Responds to piece requests
//! - Simulates protocol messages
//!
//! Each test focuses on a specific aspect of the protocol to ensure
//! proper implementation of the BitTorrent specification.

use super::*;
use peer::PeerConfig;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::debug;

/// Mock implementation of a BitTorrent peer for testing purposes.
struct MockPeer {
    listener: TcpListener,
}

impl MockPeer {
    /// Creates a new MockPeer listening on a random local port.
    async fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        debug!("MockPeer listening on {}", listener.local_addr().unwrap());
        Self { listener }
    }

    /// Returns the socket address the mock peer is listening on.
    fn addr(&self) -> std::net::SocketAddr {
        self.listener.local_addr().unwrap()
    }

    /// Accepts a single connection and handles it with the provided handler function.
    ///
    /// # Arguments
    /// * `handler` - Closure that processes the peer connection
    async fn handle_connection<F, Fut>(self, handler: F)
    where
        F: FnOnce(TcpStream) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        tokio::spawn(async move {
            debug!("Waiting for peer connection...");
            let (stream, addr) = self.listener.accept().await.unwrap();
            debug!("Accepted connection from {}", addr);
            handler(stream).await;
        });
    }
}

/// Tests serialization and deserialization of all message types.
#[test]
fn test_message_serialization() {
    debug!("Starting message serialization test");
    let messages = vec![
        (message::Message::KeepAlive, vec![]),
        (message::Message::Choke, vec![0, 0, 0, 1, 0]),
        (message::Message::Unchoke, vec![0, 0, 0, 1, 1]),
        (message::Message::Interested, vec![0, 0, 0, 1, 2]),
        (message::Message::NotInterested, vec![0, 0, 0, 1, 3]),
        (message::Message::Have(42), vec![0, 0, 0, 5, 4, 0, 0, 0, 42]),
        (
            message::Message::Bitfield(vec![1, 2, 3]),
            vec![0, 0, 0, 4, 5, 1, 2, 3],
        ),
        (
            message::Message::Request {
                index: 1,
                begin: 2,
                length: 16384,
            },
            vec![0, 0, 0, 13, 6, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 64, 0],
        ),
    ];

    for (message, expected_bytes) in messages {
        debug!("Testing message: {:?}", message);
        assert_eq!(message.to_bytes(), expected_bytes);
        if !expected_bytes.is_empty() {
            assert_eq!(
                message::Message::from_bytes(&expected_bytes[4..]).unwrap(),
                message
            );
        }
    }
    debug!("Message serialization test completed");
}

/// Tests downloading a single piece from a peer.
#[tokio::test]
async fn test_piece_download() {
    debug!("Starting single piece download test");
    let mock_peer = MockPeer::new().await;
    let peer_addr = mock_peer.addr();

    mock_peer
        .handle_connection(|mut stream| async move {
            debug!("Mock peer handling connection");
            let mut handshake = [0u8; 68];
            stream.read_exact(&mut handshake).await.unwrap();
            debug!("Received handshake from peer");
            assert_eq!(&handshake[1..20], b"BitTorrent protocol");

            stream.write_all(&handshake).await.unwrap();
            debug!("Sent handshake response");

            let bitfield = message::Message::Bitfield(vec![0xFF]).to_bytes();
            stream.write_all(&bitfield).await.unwrap();
            debug!("Sent bitfield");

            let mut msg_len = [0u8; 4];
            stream.read_exact(&mut msg_len).await.unwrap();
            let mut msg_type = [0u8];
            stream.read_exact(&mut msg_type).await.unwrap();
            debug!("Received message type: {}", msg_type[0]);
            assert_eq!(msg_type[0], 2); // Interested

            stream
                .write_all(&message::Message::Unchoke.to_bytes())
                .await
                .unwrap();
            debug!("Sent unchoke message");

            let piece_data = vec![42u8; 16384];
            loop {
                let mut header = [0u8; 4];
                if stream.read_exact(&mut header).await.is_err() {
                    debug!("Connection closed by peer");
                    break;
                }
                let mut msg_type = [0u8];
                stream.read_exact(&mut msg_type).await.unwrap();
                debug!("Received request message type: {}", msg_type[0]);

                if msg_type[0] == 6 {
                    let mut request = [0u8; 12];
                    stream.read_exact(&mut request).await.unwrap();
                    debug!("Received piece request");

                    let response = message::Message::Piece {
                        index: 0,
                        begin: 0,
                        block: piece_data.clone(),
                    }
                    .to_bytes();
                    stream.write_all(&response).await.unwrap();
                    debug!("Sent piece data");
                }
            }
        })
        .await;

    debug!("Connecting to mock peer at {}", peer_addr);
    let mut peer = peer::Peer::new(peer_addr, PeerConfig::default());
    peer.connect().await.unwrap();
    debug!("Starting piece download");
    let piece = peer.download_piece(0, 16384).await.unwrap();
    debug!("Piece download completed, length: {}", piece.len());
    assert_eq!(piece.len(), 16384);
    assert!(piece.iter().all(|&b| b == 42));
}

/// Tests handling of malformed messages from peers.
#[tokio::test]
async fn test_message_error_handling() {
    debug!("Starting malformed message test");
    let mock_peer = MockPeer::new().await;
    let peer_addr = mock_peer.addr();

    mock_peer
        .handle_connection(|mut stream| async move {
            debug!("Mock peer handling connection");
            let mut handshake = [0u8; 68];
            stream.read_exact(&mut handshake).await.unwrap();
            stream.write_all(&handshake).await.unwrap();
            debug!("Handshake completed");

            debug!("Sending malformed message");
            stream.write_all(&[0xFF, 0xFF, 0xFF, 0xFF]).await.unwrap();
        })
        .await;

    let mut peer = peer::Peer::new(peer_addr, PeerConfig::default());
    peer.connect().await.unwrap();
    debug!("Testing piece download with malformed message");
    assert!(peer.download_piece(0, 16384).await.is_err());
}

/// Tests connection timeout handling for unreachable peers.
#[tokio::test]
async fn test_peer_connection_timeout() {
    debug!("Starting connection timeout test");
    let addr = "10.0.0.1:1234".parse().unwrap();
    let mut peer = peer::Peer::new(addr, PeerConfig::default());
    debug!("Attempting to connect to unreachable peer: {}", addr);
    assert!(peer.connect().await.is_err());
}

/// Tests proper handling of keep-alive messages.
#[tokio::test]
async fn test_message_handling_keep_alive() {
    debug!("Starting keep-alive message test");
    let mock_peer = MockPeer::new().await;
    let peer_addr = mock_peer.addr();

    mock_peer
        .handle_connection(|mut stream| async move {
            debug!("Mock peer handling connection");
            let mut handshake = [0u8; 68];
            stream.read_exact(&mut handshake).await.unwrap();
            stream.write_all(&handshake).await.unwrap();
            debug!("Handshake completed");

            debug!("Sending keep-alive message");
            stream.write_all(&[0, 0, 0, 0]).await.unwrap();
        })
        .await;

    let mut peer = peer::Peer::new(peer_addr, PeerConfig::default());
    peer.connect().await.unwrap();
    debug!("Waiting for keep-alive message");

    match peer.receive_message().await.unwrap() {
        message::Message::KeepAlive => debug!("Received keep-alive message"),
        _ => panic!("Expected keep-alive message"),
    }
}

/// Tests downloading a complete file from a peer.
#[tokio::test]
async fn test_download_complete_file() {
    debug!("Starting complete file download test");
    let mock_peer = MockPeer::new().await;
    let peer_addr = mock_peer.addr();

    mock_peer
        .handle_connection(|mut stream| async move {
            debug!("Mock peer handling connection");
            // Handle handshake
            let mut handshake = [0u8; 68];
            stream.read_exact(&mut handshake).await.unwrap();
            stream.write_all(&handshake).await.unwrap();
            debug!("Handshake completed");

            // Send bitfield showing we have all pieces
            let bitfield = message::Message::Bitfield(vec![0xFF]).to_bytes();
            stream.write_all(&bitfield).await.unwrap();
            debug!("Sent bitfield");

            // Handle interested message
            let mut msg_len = [0u8; 4];
            stream.read_exact(&mut msg_len).await.unwrap();
            let mut msg_type = [0u8];
            stream.read_exact(&mut msg_type).await.unwrap();
            debug!("Received message type: {}", msg_type[0]);
            assert_eq!(msg_type[0], 2); // Interested

            // Send unchoke
            stream
                .write_all(&message::Message::Unchoke.to_bytes())
                .await
                .unwrap();
            debug!("Sent unchoke message");

            // Handle piece requests
            let piece_data = vec![42u8; 16384];
            loop {
                let mut header = [0u8; 4];
                if stream.read_exact(&mut header).await.is_err() {
                    debug!("Connection closed by peer");
                    break;
                }
                let mut msg_type = [0u8];
                stream.read_exact(&mut msg_type).await.unwrap();
                debug!("Received request message type: {}", msg_type[0]);

                if msg_type[0] == 6 {
                    let mut request = [0u8; 12];
                    stream.read_exact(&mut request).await.unwrap();
                    debug!("Received piece request");

                    let response = message::Message::Piece {
                        index: 0,
                        begin: 0,
                        block: piece_data.clone(),
                    }
                    .to_bytes();
                    stream.write_all(&response).await.unwrap();
                    debug!("Sent piece data");
                }
            }
        })
        .await;

    debug!("Connecting to mock peer at {}", peer_addr);
    let mut peer = peer::Peer::new(peer_addr, PeerConfig::default());
    peer.connect().await.unwrap();

    // Download multiple pieces
    debug!("Starting multi-piece download");
    for i in 0..3 {
        debug!("Downloading piece {}", i);
        let piece = peer.download_piece(i, 16384).await.unwrap();
        debug!("Piece {} downloaded, length: {}", i, piece.len());
        assert_eq!(piece.len(), 16384);
        assert!(piece.iter().all(|&b| b == 42));
    }
    debug!("Multi-piece download completed");
}
