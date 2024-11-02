//! BitTorrent magnet link parsing and handling.
//!
//! Provides functionality for parsing magnet URIs that contain torrent metadata like:
//! - Info hash (xt parameter)
//! - Display name (dn parameter)
//! - Tracker URLs (tr parameter)
//!
//! Magnet links allow sharing torrent metadata without a .torrent file.
//! Format: magnet:?xt=urn:btih:<info-hash>&dn=<name>&tr=<tracker-url>

use anyhow::Result;

/// Represents a parsed BitTorrent magnet link
pub struct MagnetLink {
    /// 20-byte SHA-1 hash of the info dictionary
    pub info_hash: [u8; 20],
    /// Optional file name of the torrent content
    pub name: Option<String>,
    /// Optional tracker URL for peer discovery
    pub tracker: Option<String>,
}

impl MagnetLink {
    /// Parse a magnet URI string into a MagnetLink struct
    ///
    /// # Arguments
    /// * `magnet_link` - The magnet URI string to parse
    ///
    /// # Returns
    /// * `Result<MagnetLink>` - Parsed magnet link on success, error on invalid format
    pub fn parse(magnet_link: &str) -> Result<Self> {
        if !magnet_link.starts_with("magnet:?") {
            return Err(anyhow::anyhow!("Not a magnet link"));
        }

        let mut info_hash = None;
        let mut tracker = None;
        let mut name = None;
        let query = &magnet_link["magnet:?".len()..];
        for param in query.split('&') {
            let mut parts = param.split('=');
            let key = parts.next().unwrap_or("");
            let value = parts.next().unwrap_or("");

            match key {
                "xt" => {
                    if let Some(hash) = value.strip_prefix("urn:btih:") {
                        let mut arr = [0u8; 20];
                        for i in 0..20 {
                            let byte = u8::from_str_radix(&hash[i * 2..i * 2 + 2], 16)?;
                            arr[i] = byte;
                        }
                        info_hash = Some(arr);
                    }
                }
                "tr" => tracker = Some(url_decode(value)?),
                "dn" => name = Some(url_decode(value)?),
                _ => {}
            }
        }

        let info_hash = info_hash.ok_or_else(|| anyhow::anyhow!("Missing info hash"))?;

        Ok(Self {
            info_hash,
            name,
            tracker,
        })
    }
}

impl std::fmt::Display for MagnetLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(tracker) = &self.tracker {
            writeln!(f, "Tracker URL: {}", tracker)?;
        }
        write!(
            f,
            "Info Hash: {}",
            self.info_hash
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>()
        )
    }
}

/// URL decode a percent-encoded string
///
/// # Arguments
/// * `input` - The percent-encoded string to decode
///
/// # Returns
/// * `Result<String>` - Decoded string on success, error on invalid encoding
fn url_decode(input: &str) -> Result<String> {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars();

    while let Some(c) = chars.next() {
        if c == '%' {
            let hex = chars
                .next()
                .and_then(|c1| chars.next().map(|c2| (c1, c2)))
                .ok_or_else(|| anyhow::anyhow!("Invalid percent encoding"))?;

            let byte = u8::from_str_radix(&format!("{}{}", hex.0, hex.1), 16)?;
            output.push(byte as char);
        } else {
            output.push(c);
        }
    }

    Ok(output)
}
