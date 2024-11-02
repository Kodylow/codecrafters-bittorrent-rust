use std::net::SocketAddr;

use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::info;

use crate::{PEER_ID, PROTOCOL};

use super::message::Message;

pub type PeerId = [u8; 20];
pub type InfoHash = [u8; 20];

#[derive(Debug, Clone)]
pub struct PeerConfig {
    pub peer_id: PeerId,
    pub info_hash: InfoHash,
    pub port: u16,
}

impl Default for PeerConfig {
    fn default() -> Self {
        Self {
            peer_id: *PEER_ID,
            info_hash: [0u8; 20],
            port: 6881,
        }
    }
}

#[derive(Debug)]
pub struct Peer {
    addr: SocketAddr,
    stream: Option<TcpStream>,
    pub peer_id: Option<PeerId>,
    config: PeerConfig,
}

impl Peer {
    pub fn new(addr: SocketAddr, config: PeerConfig) -> Self {
        Self {
            addr,
            stream: None,
            peer_id: None,
            config,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        info!("Connecting to peer: {}", self.addr);
        let stream = TcpStream::connect(self.addr).await?;
        self.stream = Some(stream);
        self.handshake().await?;
        Ok(())
    }

    async fn handshake(&mut self) -> Result<()> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;

        // Construct handshake message
        let mut message = Vec::with_capacity(68);
        message.push(19);
        message.extend_from_slice(PROTOCOL.as_bytes());

        // Set reserved bytes with bit 20 set to 1 (extension protocol support)
        let mut reserved = [0u8; 8];
        reserved[5] = 0x10; // Set bit 20 (00010000)
        message.extend_from_slice(&reserved);

        message.extend_from_slice(&self.config.info_hash);
        message.extend_from_slice(&*PEER_ID);

        // Send handshake
        stream.write_all(&message).await?;
        info!("Sent handshake message with extension protocol support");

        // Read response
        let mut response = [0u8; 68];
        stream.read_exact(&mut response).await?;
        info!("Received handshake response");

        // Verify protocol
        if response[1..20] != *PROTOCOL.as_bytes() {
            return Err(anyhow::anyhow!("Invalid protocol in handshake response"));
        }

        // Verify info hash
        if response[28..48] != self.config.info_hash {
            return Err(anyhow::anyhow!("Info hash mismatch in handshake"));
        }

        // Store peer ID
        let mut peer_id = [0u8; 20];
        peer_id.copy_from_slice(&response[48..68]);
        self.peer_id = Some(peer_id);

        Ok(())
    }

    pub async fn send_message(&mut self, message: Message) -> Result<()> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        let bytes = message.to_bytes();
        stream.write_all(&bytes).await?;
        Ok(())
    }

    pub async fn receive_message(&mut self) -> Result<Message> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;

        // Read message length (4 bytes)
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes).await?;
        let len = u32::from_be_bytes(len_bytes);

        if len == 0 {
            return Ok(Message::KeepAlive);
        }

        // Read message body
        let mut message_bytes = vec![0u8; len as usize];
        stream.read_exact(&mut message_bytes).await?;

        Message::from_bytes(&message_bytes)
    }

    pub async fn download_piece(
        &mut self,
        piece_index: usize,
        piece_length: usize,
    ) -> Result<Vec<u8>> {
        const BLOCK_SIZE: u32 = 16 * 1024; // 16 KiB
        let mut piece_data = Vec::new();
        let mut remaining = piece_length;
        let mut offset = 0;

        while remaining > 0 {
            let block_size = std::cmp::min(remaining, BLOCK_SIZE as usize);

            // Request block
            self.send_message(Message::Request {
                index: piece_index as u32,
                begin: offset,
                length: block_size as u32,
            })
            .await?;

            // Receive block
            match self.receive_message().await? {
                Message::Piece {
                    index,
                    begin,
                    block,
                } => {
                    if index as usize != piece_index || begin != offset {
                        return Err(anyhow::anyhow!("Received unexpected piece/offset"));
                    }
                    piece_data.extend_from_slice(&block);
                }
                _ => return Err(anyhow::anyhow!("Expected piece message")),
            }

            offset += block_size as u32;
            remaining -= block_size;
        }

        Ok(piece_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;
    async fn setup_mock_peer() -> (Peer, TcpListener) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let peer = Peer::new(addr, PeerConfig::default());
        (peer, listener)
    }

    #[test]
    fn test_peer_creation() {
        let addr = "127.0.0.1:8080".parse().unwrap();
        let peer = Peer::new(addr, PeerConfig::default());

        assert_eq!(peer.addr, addr);
        assert!(peer.stream.is_none());
        assert!(peer.peer_id.is_none());
    }

    #[tokio::test]
    async fn test_handshake_protocol_mismatch() {
        let (mut peer, listener) = setup_mock_peer().await;

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 68];
            stream.read_exact(&mut buf).await.unwrap();

            // Send back invalid protocol
            let mut response = [0u8; 68];
            response[1..20].copy_from_slice(b"Invalid protocol!!!!");
            stream.write_all(&response).await.unwrap();
        });

        peer.connect()
            .await
            .expect_err("Should fail with protocol mismatch");
    }

    #[tokio::test]
    async fn test_handshake_info_hash_mismatch() {
        let (mut peer, listener) = setup_mock_peer().await;

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 68];
            stream.read_exact(&mut buf).await.unwrap();

            // Send back valid protocol but wrong info_hash
            let mut response = [0u8; 68];
            response[1..20].copy_from_slice(PROTOCOL.as_bytes());
            response[28..48].copy_from_slice(&[2u8; 20]); // Wrong info_hash
            stream.write_all(&response).await.unwrap();
        });

        peer.connect()
            .await
            .expect_err("Should fail with info hash mismatch");
    }
}
