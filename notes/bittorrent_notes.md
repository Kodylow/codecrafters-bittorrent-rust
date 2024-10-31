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
