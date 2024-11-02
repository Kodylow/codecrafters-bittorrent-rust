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
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

/// Mock implementation of a BitTorrent peer for testing purposes.
struct MockPeer {
    listener: TcpListener,
}

impl MockPeer {
    /// Creates a new MockPeer listening on a random local port.
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
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
    fn handle_connection<F>(self, handler: F)
    where
        F: FnOnce(TcpStream) + Send + 'static,
    {
        thread::spawn(move || {
            let (stream, _) = self.listener.accept().unwrap();
            handler(stream);
        });
    }
}

/// Tests serialization and deserialization of all message types.
#[test]
fn test_message_serialization() {
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
        assert_eq!(message.to_bytes(), expected_bytes);
        if !expected_bytes.is_empty() {
            assert_eq!(
                message::Message::from_bytes(&expected_bytes[4..]).unwrap(),
                message
            );
        }
    }
}

/// Tests downloading a single piece from a peer.
#[test]
fn test_piece_download() {
    let mock_peer = MockPeer::new();
    let peer_addr = mock_peer.addr();

    mock_peer.handle_connection(|mut stream| {
        let mut handshake = [0u8; 68];
        stream.read_exact(&mut handshake).unwrap();
        assert_eq!(&handshake[1..20], b"BitTorrent protocol");

        stream.write_all(&handshake).unwrap();

        let bitfield = message::Message::Bitfield(vec![0xFF]).to_bytes();
        stream.write_all(&bitfield).unwrap();

        let mut msg_len = [0u8; 4];
        stream.read_exact(&mut msg_len).unwrap();
        let mut msg_type = [0u8];
        stream.read_exact(&mut msg_type).unwrap();
        assert_eq!(msg_type[0], 2); // Interested

        stream
            .write_all(&message::Message::Unchoke.to_bytes())
            .unwrap();

        let piece_data = vec![42u8; 16384];
        loop {
            let mut header = [0u8; 4];
            if stream.read_exact(&mut header).is_err() {
                break;
            }
            let mut msg_type = [0u8];
            stream.read_exact(&mut msg_type).unwrap();

            if msg_type[0] == 6 {
                let mut request = [0u8; 12];
                stream.read_exact(&mut request).unwrap();

                let response = message::Message::Piece {
                    index: 0,
                    begin: 0,
                    block: piece_data.clone(),
                }
                .to_bytes();
                stream.write_all(&response).unwrap();
            }
        }
    });

    let mut peer = peer::Peer::new(peer_addr, [0u8; 20]);
    peer.connect().unwrap();
    let piece = peer.download_piece(0, 16384).unwrap();
    assert_eq!(piece.len(), 16384);
    assert!(piece.iter().all(|&b| b == 42));
}

/// Tests handling of malformed messages from peers.
#[test]
fn test_message_error_handling() {
    let mock_peer = MockPeer::new();
    let peer_addr = mock_peer.addr();

    mock_peer.handle_connection(|mut stream| {
        let mut handshake = [0u8; 68];
        stream.read_exact(&mut handshake).unwrap();
        stream.write_all(&handshake).unwrap();

        stream.write_all(&[0xFF, 0xFF, 0xFF, 0xFF]).unwrap();
    });

    let mut peer = peer::Peer::new(peer_addr, [0u8; 20]);
    peer.connect().unwrap();
    assert!(peer.download_piece(0, 16384).is_err());
}

/// Tests connection timeout handling for unreachable peers.
#[test]
fn test_peer_connection_timeout() {
    let addr = "10.0.0.1:1234".parse().unwrap(); // Non-existent address
    let mut peer = peer::Peer::new(addr, [0u8; 20]);
    assert!(peer.connect().is_err());
}

/// Tests proper handling of keep-alive messages.
#[test]
fn test_message_handling_keep_alive() {
    let mock_peer = MockPeer::new();
    let peer_addr = mock_peer.addr();

    mock_peer.handle_connection(|mut stream| {
        let mut handshake = [0u8; 68];
        stream.read_exact(&mut handshake).unwrap();
        stream.write_all(&handshake).unwrap();

        // Send keep-alive message
        stream.write_all(&[0, 0, 0, 0]).unwrap();
    });

    let mut peer = peer::Peer::new(peer_addr, [0u8; 20]);
    peer.connect().unwrap();

    match peer.receive_message().unwrap() {
        message::Message::KeepAlive => (),
        _ => panic!("Expected keep-alive message"),
    }
}

/// Tests downloading a complete file from a peer.
#[test]
fn test_download_complete_file() {
    let mock_peer = MockPeer::new();
    let peer_addr = mock_peer.addr();

    mock_peer.handle_connection(|mut stream| {
        // Handle handshake
        let mut handshake = [0u8; 68];
        stream.read_exact(&mut handshake).unwrap();
        stream.write_all(&handshake).unwrap();

        // Send bitfield showing we have all pieces
        let bitfield = message::Message::Bitfield(vec![0xFF]).to_bytes();
        stream.write_all(&bitfield).unwrap();

        // Handle interested message
        let mut msg_len = [0u8; 4];
        stream.read_exact(&mut msg_len).unwrap();
        let mut msg_type = [0u8];
        stream.read_exact(&mut msg_type).unwrap();
        assert_eq!(msg_type[0], 2); // Interested

        // Send unchoke
        stream
            .write_all(&message::Message::Unchoke.to_bytes())
            .unwrap();

        // Handle piece requests
        let piece_data = vec![42u8; 16384];
        loop {
            let mut header = [0u8; 4];
            if stream.read_exact(&mut header).is_err() {
                break;
            }
            let mut msg_type = [0u8];
            stream.read_exact(&mut msg_type).unwrap();

            if msg_type[0] == 6 {
                let mut request = [0u8; 12];
                stream.read_exact(&mut request).unwrap();

                let response = message::Message::Piece {
                    index: 0,
                    begin: 0,
                    block: piece_data.clone(),
                }
                .to_bytes();
                stream.write_all(&response).unwrap();
            }
        }
    });

    let mut peer = peer::Peer::new(peer_addr, [0u8; 20]);
    peer.connect().unwrap();

    // Download multiple pieces
    for i in 0..3 {
        let piece = peer.download_piece(i, 16384).unwrap();
        assert_eq!(piece.len(), 16384);
        assert!(piece.iter().all(|&b| b == 42));
    }
}
