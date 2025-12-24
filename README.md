<div align="center">

# FinCLI

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Go](https://img.shields.io/badge/Go-1.21%2B-00ADD8?logo=go)](https://go.dev/)

**A fast CLI tool for tracking investment portfolios with real-time market data**

[Features](#-features) • [Quick Start](#-quick-start) • [Usage](#-usage) • [Roadmap](#️-roadmap)

---

![Status](https://img.shields.io/badge/Status-Production%20Ready-brightgreen)

</div>

## ✨ Features

| Feature | Description |
|---------|-------------|
| **Real-time Data** | Direct Yahoo Finance API integration for live pricing |
| **Portfolio Tracking** | Monitor holdings with current values and daily P&L |
| **Concurrent Fetching** | Parallel requests via goroutines for fast updates |
| **Single Binary** | No runtime dependencies, cross-platform executable |

---

## 🛠️ Tech Stack

| Technology | Purpose |
|------------|---------|
| **Go 1.21+** | Runtime |
| **Cobra** | CLI framework |
| **Lipgloss** | Terminal styling |
| **Yahoo Finance API** | Market data |

---

## 🚀 Quick Start

### Prerequisites
- Go 1.21 or higher

### Installation

```bash
# Clone the repository
git clone https://github.com/gicrisf/fincli.git
cd fincli

# Build
go build -o fincli ./cmd/fincli

# Run
./fincli --help
```

---

## 📖 Usage

### Ticker Command

```bash
# Basic price info
./fincli ticker AAPL

# Full information
./fincli ticker AAPL --info

# Specific attribute
./fincli ticker AAPL --attribute marketCap
```

| Option | Short | Description |
|--------|-------|-------------|
| `--info` | `-i` | Display all available information |
| `--attribute` | `-a` | Display a specific attribute |

### Portfolio Command

```bash
# Show portfolio with P&L
./fincli portfolio

# Show only total value
./fincli portfolio --total

# Custom portfolio file
./fincli portfolio --file ~/my_portfolio.json
```

| Option | Short | Description |
|--------|-------|-------------|
| `--total` | `-t` | Display only total value |
| `--file` | `-f` | Path to portfolio JSON (default: `portfolio.json`) |

---

## ⚙️ Configuration

Create a `portfolio.json` file:

```json
[
    {"ticker": "AAPL", "shares": 10},
    {"ticker": "VWCE.MI", "shares": 68}
]
```

| Field | Description |
|-------|-------------|
| `ticker` | Stock/ETF symbol (use exchange suffix for non-US, e.g. `.MI`) |
| `shares` | Number of shares owned |

---

## 📦 Cross-Compilation

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

---

## 🗺️ Roadmap

### ✅ Completed

- [x] Core CLI with Cobra
- [x] Yahoo Finance integration
- [x] Portfolio tracking with P&L
- [x] Concurrent ticker fetching
- [x] Colored terminal output

### 🔄 In Progress

<!-- TODO -->

### 📋 Planned

- [ ] **Response Caching** — TTL-based cache for API responses to reduce latency, avoid rate limiting, and enable offline viewing of recent data

---

## 📁 Project Structure

```
fincli/
├── cmd/fincli/
│   └── main.go           # Entry point
├── internal/
│   ├── cli/              # CLI commands
│   ├── finance/          # Yahoo Finance client
│   ├── portfolio/        # Portfolio logic
│   └── display/          # Terminal output
├── portfolio.json        # Portfolio data
├── go.mod
└── README.md
```

---

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

---

## 📄 License

This project is licensed under the **MIT License** — see the [LICENSE](LICENSE) file for details.

---

<div align="center">

**Made with ❤️ by [Giorgio Cembran](https://github.com/gicrisf)**

</div>
