# Set the local rustup toolchain to 1.77.0 for this project
rust: 
  rustup install 1.77 && rustup default 1.77 && rustc --version && cargo --version

# Run locally
decode bencoded_value: 
  cargo run -- decode {{bencoded_value}}
