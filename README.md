# reso-dns

> **Work in progress:** expect breaking changes and missing features.

A fast, self hosted DNS resolver with a web UI. Supports forwarding over UDP and TCP, with query caching and domain blocking.

---

![Dashboard](docs/screenshots/dashboard.png)

## Features

- **Web UI** built in dashboard for monitoring and configuration
- **Query Cache** in-memory caching to reduce upstream lookups
- **Blocklist** domain blocking

## Screenshots

![Dashboard](docs/screenshots/dashboard.png)
![Logs](docs/screenshots/logs.png)
![Configuration](docs/screenshots/configuration.png)
![Blocklist](docs/screenshots/blocklist.png)

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) 1.93+
- [pnpm](https://pnpm.io/installation)

### Build

```sh
cargo build
```

### Run

```sh
cargo run
```

## Docker

```sh
docker pull ghcr.io/thohui/reso-dns:latest
```

## Configuration

Copy the example env file and fill in the values:

```sh
cp reso/.env.example reso/.env
```

| Variable                   | Default        | Description                               |
| -------------------------- | -------------- | ----------------------------------------- |
| `RESO_DATABASE_PATH`       | `reso.db`      | Path to the SQLite database file          |
| `RESO_DNS_SERVER_ADDRESS`  | `0.0.0.0:5300` | Address the DNS server listens on         |
| `RESO_HTTP_SERVER_ADDRESS` | `0.0.0.0:80`   | Address the web UI/API listens on         |
| `RESO_COOKIE_SECRET`       | —              | Secret key for signing cookies (required) |

Generate a secret with:

```sh
openssl rand -base64 32
```
