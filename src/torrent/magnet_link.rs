use anyhow::Result;

pub struct MagnetLink {
    pub url: String,
    pub info_hash: [u8; 20],
    pub name: Option<String>,
    pub tracker: Option<String>,
}

impl MagnetLink {
    pub fn parse(magnet_link: &str) -> Result<Self> {
        let url = url::Url::parse(magnet_link)?;

        if url.scheme() != "magnet" {
            return Err(anyhow::anyhow!("Not a magnet link"));
        }

        let mut info_hash = None;
        let mut name = None;
        let mut tracker = None;

        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "xt" => {
                    if let Some(hash) = value.strip_prefix("urn:btih:") {
                        let hash_bytes = hex::decode(hash)?;
                        if hash_bytes.len() != 20 {
                            return Err(anyhow::anyhow!("Invalid info hash length"));
                        }
                        let mut arr = [0u8; 20];
                        arr.copy_from_slice(&hash_bytes);
                        info_hash = Some(arr);
                    }
                }
                "dn" => name = Some(value.into_owned()),
                "tr" => tracker = Some(value.into_owned()),
                _ => {}
            }
        }

        let info_hash = info_hash.ok_or_else(|| anyhow::anyhow!("Missing info hash"))?;

        Ok(Self {
            url: magnet_link.to_string(),
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
        write!(f, "Info Hash: {}", hex::encode(self.info_hash))
    }
}
