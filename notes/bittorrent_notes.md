# Bencode

Serialization format for the BitTorrent protocol. Bencode supports four data types:

- strings
  - Strings are encoded as `<length>:<contents>`. For example, the string "hello" is encoded as "5:hello".
  - Printing a string requires inner quotes.
- integers
  - Integers are encoded as `i<integer>e`. For example, the integer 123 is encoded as `i123e`.
  - Negative integers are supported as `i-123e`.
- lists
  - Lists are encoded as `l<bencoded_elements>e`.
  - For example, `["hello", 52]` would be encoded as `l5:helloi52ee`. Note that there are no separators between the elements.
- dictionaries
  - A dictionary is encoded as `d<key1><value1>...<keyN><valueN>e`. `<key1>`, `<value1>` etc. correspond to the bencoded keys & values. The keys are sorted in lexicographical order and must be strings.
  - For example, `{"hello": 52, "foo":"bar"}` would be encoded as: `d3:foo3:bar5:helloi52ee` (note that the keys were reordered).

# Torrent metainfo

A torrent file (also known as a metainfo file) contains a bencoded dictionary with the following keys and values:

- `announce`: URL to a "tracker", which is a central server that keeps track of peers participating in the sharing of a torrent.
- `info`:
  A dictionary with keys:
  length: size of the file in bytes, for single-file torrents
  - `name`: suggested name to save the file / directory as
  - `piece length`: number of bytes in each piece
  - `pieces`: concatenated SHA-1 hashes of each piece
  - Note: .torrent files contain bytes that aren’t valid UTF-8 characters. You'll run into problems if you try to read the contents of this file as a String. Use &[u8] or Vec<u8> instead.

Note: The info dictionary looks slightly different for multi-file torrents. For this challenge, we'll only implement support for single-file torrents.

## Info hash

Info hash is a unique identifier for a torrent file. It's used when talking to trackers or peers.

- To calculate the info hash:
  - Extract the info dictionary from the torrent file after parsing
  - Bencode the contents of the info dictionary
  - Calculate the SHA-1 hash of this bencoded dictionary

# Discover Peers (Tracker Request)

Trackers are central servers that maintain information about peers participating in the sharing and downloading of a torrent. To discover peers, make a request to the tracker URL extracted as `announce` from the torrent file.

- `info_hash`: the info hash of the torrent
  - 20 bytes long, will need to be URL encoded
  - Note: this is NOT the hexadecimal representation, which is 40 bytes long
- `peer_id`: a unique identifier for your client
  - A string of length 20 that you get to pick.
- `port`: the port your client is listening on
  - You can set this to 6881, you will not have to support this functionality during this challenge.
- `uploaded`: the total amount uploaded so far
  - Since your client hasn't uploaded anything yet, you can set this to 0.
- `downloaded`: the total amount downloaded so far
  - Since your client hasn't downloaded anything yet, you can set this to 0.
- `left`: the number of bytes left to download
  - Since you client hasn't downloaded anything yet, this'll be the total length of the file (you've extracted this value from the torrent file in previous stages)
- `compact`: whether the peer list should use the compact representation
  - For the purposes of this challenge, set this to 1.
  - The compact representation is more commonly used in the wild, the non-compact representation is mostly supported for backward-compatibility.

Tracker response:

- `interval`: the number of seconds you should wait before requesting an update from the tracker
- `peers`: a list of peers
  - A string, which contains list of peers that your client can connect to.
    - Each peer is represented using 6 bytes. The first 4 bytes are the peer's IP address and the last 2 bytes are the peer's port number.
  - Each peer is represented as a dictionary with the following keys:
    - `ip`: the IP address of the peer
    - `port`: the port the peer is listening on

# Downloading Pieces

To download a piece:

1. Connect to peer and handshake (done in previous stages)

2. Initial peer message exchange:

   - Receive bitfield message (id=5) showing peer's available pieces
   - Send interested message (id=2, empty payload)
   - Wait for unchoke message (id=1, empty payload)

3. Download piece in 16KiB blocks:

   - Break piece into 16KiB (16384 byte) blocks
   - For each block:
     - Send request message (id=6):
       - index: piece index
       - begin: block offset (0, 16384, 32768, etc)
       - length: block size (16384 or less for last block)
     - Receive piece message (id=7):
       - index: piece index
       - begin: block offset
       - block: actual data

4. Verify piece integrity:
   - Combine blocks into complete piece
   - Calculate SHA-1 hash
   - Compare against piece hash from torrent file

Message Format:

- 4 bytes: message length prefix
- 1 byte: message id
- Variable length payload

Message IDs:

- 5: bitfield
- 2: interested
- 1: unchoke
- 6: request
- 7: piece

Optional optimization:

- Pipeline multiple requests (5 pending recommended)
- Improves download speed by reducing block transfer delays

# Magnet Links

Magnet links contain minimal info to discover peers without a .torrent file:

- Info hash (40-char hex)
- File name (optional)
- Tracker URL (optional)

Format:
magnet:?xt=urn:btih:<info-hash>&dn=<name>&tr=<tracker-url>
magnet:?xt=urn:btih:ad42ce8109f54c99613ce38f9b4d87e70f24a165&dn=magnet1.gif&tr=http%3A%2F%2Fbittorrent-test-tracker.codecrafters.io%2Fannounce

# Extension Protocol

Extension protocol allows adding functionality without breaking compatibility.

Handshake reserved bytes (8 bytes total):
00 00 00 00 00 10 00 00

Setting bit 20 (from right, 0-based) indicates extension support:

- Binary: .... 00010000 00000000 00000000
- Hex: 00 00 00 00 00 10 00 00
- Bit 20 = 1, all others = 0

Extension handshake process:

1. Set bit 20 in handshake reserved bytes
2. Send handshake with modified reserved bytes
3. Peer response indicates if they support extensions

Extension messages:

- ID 20: Extension protocol message
- First byte: Extension message ID
- Rest: bencoded payload

Common extensions:

- Metadata exchange (BEP 9)
- PEX peer exchange (BEP 11)
- DHT (BEP 5)
