# FinCLI

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.95%2B-orange?logo=rust)](https://www.rust-lang.org/)

A CLI tool for tracking investment portfolios with real-time Yahoo Finance data.

## Features

- Real-time quotes from Yahoo Finance API
- Portfolio tracking with daily P&L calculations
- Multi-currency support with grouped subtotals
- Concurrent data fetching for fast updates
- File-based caching with 2-minute TTL
- Single binary, no runtime dependencies

## Installation

Build from source:

```bash
git clone https://github.com/gicrisf/fincli.git
cd fincli
cargo build --release
```

The binary will be available at `target/release/fincli`.

To install or update `fincli` in your `PATH`:

```bash
cargo install --path . --force
```

Make sure Cargo binaries are available in your shell:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Alternatively, install the release binary system-wide:

```bash
sudo install -m 755 target/release/fincli /usr/local/bin/fincli
```

If `/usr/local/bin/fincli` is a symlink to a local `./fincli` binary, update that binary with:

```bash
cargo build --release
cp target/release/fincli ./fincli
```

## Usage

### Ticker

```bash
# Get current price
fincli ticker AAPL

# Full information
fincli ticker AAPL --info

# Specific attribute
fincli ticker AAPL --attribute marketCap
```

### Portfolio

```bash
# Show portfolio with P&L
fincli portfolio

# Show only total value
fincli portfolio --total

# Custom portfolio file
fincli portfolio --file ~/my_portfolio.json
```

## Configuration

Create `~/.fincli/portfolio.json`:

```json
[
  {"ticker": "AAPL", "shares": 10},
  {"ticker": "VWCE.MI", "shares": 68}
]
```

Use exchange suffixes for non-US markets (e.g., `.MI` for Milan, `.L` for London).

## Cross-Compilation

Install the desired Rust target first, then build with `cargo`:

```bash
# Windows
rustup target add x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc

# macOS (Intel)
rustup target add x86_64-apple-darwin
cargo build --release --target x86_64-apple-darwin

# macOS (Apple Silicon)
rustup target add aarch64-apple-darwin
cargo build --release --target aarch64-apple-darwin

# Linux
rustup target add x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-gnu
```

## Development

```bash
# Run without installing
cargo run -- ticker AAPL
cargo run -- portfolio

# Run tests
cargo test

# Build
cargo build --release
```

## License

MIT
