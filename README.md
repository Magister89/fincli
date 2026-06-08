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

```bash
git clone https://github.com/gicrisf/fincli.git
cd fincli
cargo build --release
```

The binary will be available at `target/release/fincli`.

## Usage

### Ticker

```bash
# Get current price
./target/release/fincli ticker AAPL

# Full information
./target/release/fincli ticker AAPL --info

# Specific attribute
./target/release/fincli ticker AAPL --attribute marketCap
```

### Portfolio

```bash
# Show portfolio with P&L
./target/release/fincli portfolio

# Show only total value
./target/release/fincli portfolio --total

# Custom portfolio file
./target/release/fincli portfolio --file ~/my_portfolio.json
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
# Run tests
cargo test

# Build
cargo build --release
```

## License

MIT
