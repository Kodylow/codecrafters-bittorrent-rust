use anyhow::Result;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use tracing::info;

use super::message::Message;

const PROTOCOL: &str = "BitTorrent protocol";
const PEER_ID: [u8; 20] = *b"00112233445566778899";

#[derive(Debug)]
pub struct Peer {
    addr: SocketAddr,
    stream: Option<TcpStream>,
    pub peer_id: Option<[u8; 20]>,
    info_hash: [u8; 20],
    // am_choking: bool,
    // am_interested: bool,
    // peer_choking: bool,
    // peer_interested: bool,
}

impl Peer {
    pub fn new(addr: SocketAddr, info_hash: [u8; 20]) -> Self {
        Self {
            addr,
            stream: None,
            peer_id: None,
            info_hash,
            // am_choking: true,
            // am_interested: false,
            // peer_choking: true,
            // peer_interested: false,
        }
    }

    pub fn connect(&mut self) -> Result<()> {
        info!("Connecting to peer: {}", self.addr);
        let stream = TcpStream::connect(self.addr)?;
        self.stream = Some(stream);
        self.handshake()?;
        Ok(())
    }

    fn handshake(&mut self) -> Result<()> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;

        // Construct handshake message
        let mut message = Vec::with_capacity(68);
        message.push(19);
        message.extend_from_slice(PROTOCOL.as_bytes());
        message.extend_from_slice(&[0u8; 8]);
        message.extend_from_slice(&self.info_hash);
        message.extend_from_slice(&PEER_ID);

        // Send handshake
        stream.write_all(&message)?;
        info!("Sent handshake message");

        // Read response
        let mut response = [0u8; 68];
        stream.read_exact(&mut response)?;
        info!("Received handshake response");

        // Verify protocol
        if response[1..20] != *PROTOCOL.as_bytes() {
            return Err(anyhow::anyhow!("Invalid protocol in handshake response"));
        }

        // Verify info hash
        if response[28..48] != self.info_hash {
            return Err(anyhow::anyhow!("Info hash mismatch in handshake"));
        }

        // Store peer ID
        let mut peer_id = [0u8; 20];
        peer_id.copy_from_slice(&response[48..68]);
        self.peer_id = Some(peer_id);

        Ok(())
    }

    pub fn send_message(&mut self, message: Message) -> Result<()> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        let bytes = message.to_bytes();
        stream.write_all(&bytes)?;
        Ok(())
    }

    pub fn receive_message(&mut self) -> Result<Message> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;

        // Read message length (4 bytes)
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes)?;
        let len = u32::from_be_bytes(len_bytes);

        if len == 0 {
            return Ok(Message::KeepAlive);
        }

        // Read message body
        let mut message_bytes = vec![0u8; len as usize];
        stream.read_exact(&mut message_bytes)?;

        Message::from_bytes(&message_bytes)
    }

    pub fn download_piece(&mut self, piece_index: usize, piece_length: usize) -> Result<Vec<u8>> {
        // Wait for bitfield
        let _bitfield = match self.receive_message()? {
            Message::Bitfield(b) => b,
            _ => return Err(anyhow::anyhow!("Expected bitfield message")),
        };

        // Send interested
        self.send_message(Message::Interested)?;

        // Wait for unchoke
        match self.receive_message()? {
            Message::Unchoke => (),
            _ => return Err(anyhow::anyhow!("Expected unchoke message")),
        }

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
            })?;

            // Receive block
            match self.receive_message()? {
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
