run platform:
    cargo run --bin todo-{{platform}}

build:
    cargo build

check:
    cargo clippy
