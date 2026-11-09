run platform:
    cargo run --bin todo-{{platform}}

build:
    cargo build --workspace

check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace
