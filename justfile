fmt:
    cargo fmt && cargo fmt --manifest-path ./derive/Cargo.toml
clippy:
    cargo clippy --all-features && cargo clippy --all-features --manifest-path ./derive/Cargo.toml
test:
    cargo test && cargo test --all-features
