# Bencode

Serialization format for the BitTorrent protocol. Bencode supports four data types:

- strings
  - Strings are encoded as `<length>:<contents>`. For example, the string "hello" is encoded as "5:hello".
- integers
  - Integers are encoded as `i<integer>e`. For example, the integer 123 is encoded as `i123e`.
- arrays
  - Arrays are encoded as `l<items>e`, where `<items>` is a list of bencoded items. For example, the array `[1, 2, 3]` is encoded as `li1ei2ei3ee`.
- dictionaries
  - Dictionaries are encoded as `d<items>e`, where `<items>` is a list of key-value pairs. For example, the dictionary `{"name": "John", "age": 30}` is encoded as `d3:agei30e4:name5:Johnee`.
