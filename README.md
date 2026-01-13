# FinCLI

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Go](https://img.shields.io/badge/Go-1.21%2B-00ADD8?logo=go)](https://go.dev/)

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
go build -o fincli ./cmd/fincli
```

## Usage

### Ticker

```bash
# Get current price
./fincli ticker AAPL

# Full information
./fincli ticker AAPL --info

# Specific attribute
./fincli ticker AAPL --attribute marketCap
```

### Portfolio

```bash
# Show portfolio with P&L
./fincli portfolio

# Show only total value
./fincli portfolio --total

# Custom portfolio file
./fincli portfolio --file ~/my_portfolio.json
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

```bash
# Windows
GOOS=windows GOARCH=amd64 go build -o fincli.exe ./cmd/fincli

# macOS (Intel)
GOOS=darwin GOARCH=amd64 go build -o fincli-mac ./cmd/fincli

# macOS (Apple Silicon)
GOOS=darwin GOARCH=arm64 go build -o fincli-mac-arm ./cmd/fincli

# Linux
GOOS=linux GOARCH=amd64 go build -o fincli ./cmd/fincli
```

## Development

```bash
# Run tests
go test ./...

# Build
go build -o fincli ./cmd/fincli
```

## License

MIT
