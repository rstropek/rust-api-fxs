run platform:
    cargo run --bin todo-{{platform}}

build:
    cargo build

run-spin: (build-spin)
    spin up

build-spin:
    spin build

check:
    cargo clippy
