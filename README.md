# Building Web APIs with Rust

This repository compares Rust web frameworks by implementing the same small API in each of them. A larger sample then shows what a more realistic API looks like, including a database, validation, error handling, and tests.

The samples started out as the demo code for Rainer Stropek's talk *Building Web APIs with Rust - State of the Union* at Eurorust 2022 and BASTA 2023 ([slides](https://slides.com/rainerstropek/rust-api-fxs/fullscreen)). The code has since been updated to current Rust and framework versions, so details may differ from the slides.

## Samples

| Sample | Framework | What to look for |
| --- | --- | --- |
| [`todo-axum`](todo-axum/src/main.rs) | [Axum](https://github.com/tokio-rs/axum) | Macro-free routing, extractors, and Tower middleware |
| [`todo-actix-web`](todo-actix-web/src/main.rs) | [Actix Web](https://actix.rs/) | Routing with and without macros, responders, and `ResponseError` |
| [`todo-rocket`](todo-rocket/src/main.rs) | [Rocket](https://rocket.rs/) | Attribute macros, request guards, and derived responders |
| [`todo-warp`](todo-warp/src/main.rs) | [Warp](https://github.com/seanmonstar/warp) | Composable filters and rejections |
| [`hero-manager-axum`](hero-manager-axum/) | Axum and [SQLx](https://github.com/launchbadge/sqlx) | Postgres access, validation, problem details, graceful shutdown, and handler tests with mocks |

The four todo samples expose the same API and share their business logic through [`todo-logic`](todo-logic/src/lib.rs), so the code in each sample is only what is specific to its framework.

## Prerequisites

- Rust 1.94 or later
- [just](https://github.com/casey/just) (optional, for the commands below)
- Docker and [sqlx-cli](https://crates.io/crates/sqlx-cli) for `hero-manager-axum`

## Run a todo sample

Pick `axum`, `actix-web`, `rocket`, or `warp`:

```bash
just run axum
```

Each todo sample listens on port 3000 and keeps its data in memory. [`requests.http`](requests.http) contains example requests for the [REST Client](https://marketplace.visualstudio.com/items?itemName=humao.rest-client) extension for VS Code.

All samples log with [tracing](https://github.com/tokio-rs/tracing). Set `RUST_LOG` to change the log filter:

```bash
RUST_LOG=debug just run warp
```

## Run the hero manager

Run these commands in the `hero-manager-axum` folder:

```bash
just start-pg
```

```bash
just create-db
```

```bash
just apply-migrations
```

```bash
just run
```

The API listens on port 4000. The connection string comes from [`.env`](hero-manager-axum/.env), and [`requests.http`](hero-manager-axum/requests.http) contains example requests. The credentials in `.env` are for the local demo database only.

## Check the code

```bash
just check
```

```bash
just test
```

The tests do not need a database because the handlers are tested against a mocked repository.
