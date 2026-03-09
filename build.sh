rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl #static binary
cp target/x86_64-unknown-linux-musl/release/backend build