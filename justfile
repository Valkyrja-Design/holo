default: build
build:
    cargo build
build-rel:
    cargo build --release
run *ARGS:
    cargo run -- {{ARGS}} 
run-rel:
    cargo run --release
test:
    cargo test --all-targets
clippy:
    cargo clippy --all-targets --all-features -- -D warnings
fmt:
    cargo fmt --all
check:
    cargo check --all-targets
fix:
    cargo fix --all-targets --allow-dirty --allow-staged
