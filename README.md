# FinCLI

A command-line interface tool for managing and tracking investment portfolios and retrieving stock/fund information using Yahoo Finance data.

## Features

- **Ticker Information**: Retrieve detailed information about stocks, ETFs, and other financial instruments
- **Portfolio Tracking**: Monitor your investment portfolio with real-time pricing
- **Performance Metrics**: Track profit/loss (P&L) based on daily price changes
- **Real-time Data**: Direct Yahoo Finance API calls for up-to-date pricing
- **Rich Output**: Beautiful terminal output with formatted tables

## Requirements

- Python 3.x
- Dependencies:
  - `typer` - CLI framework
  - `yfinance` - Yahoo Finance API wrapper
  - `rich` - Terminal formatting

## Installation

1. Clone the repository:
```bash
git clone https://github.com/yourusername/fincli.git
cd fincli
```

2. Install dependencies:
```bash
pip install typer yfinance rich typing-extensions
```

3. Make the script executable (optional):
```bash
chmod +x fincli.py
```

## Usage

### Ticker Command

Retrieve information about a specific stock or fund.

```bash
# Basic usage - shows default price information
python fincli.py ticker AAPL

# Show all available information
python fincli.py ticker AAPL info

# Show a specific attribute
python fincli.py ticker AAPL --attribute marketCap
python fincli.py ticker AAPL -a previousClose
```

**Options:**
| Option | Short | Description |
|--------|-------|-------------|
| `--attribute` | `-a` | Display a specific ticker attribute |

### Portfolio Command

Display your portfolio status with current valuations and performance.

```bash
# Show full portfolio with all holdings
python fincli.py portfolio

# Show only portfolio totals
python fincli.py portfolio --total

# Use a custom portfolio file
python fincli.py portfolio --file /path/to/my_portfolio.json

```

**Options:**
| Option | Short | Description |
|--------|-------|-------------|
| `--total` | `-t` | Display only portfolio totals (aggregate value and P&L) |
| `--file` | `-f` | Path to portfolio JSON file (default: `portfolio.json`) |

## Portfolio File Format

Create a `portfolio.json` file in the project directory with your holdings:

```json
[
    {
        "ticker": "AAPL",
        "shares": 10
    },
    {
        "ticker": "MSFT",
        "shares": 5
    },
    {
        "ticker": "VWCE.MI",
        "shares": 68
    }
]
```

**Notes:**
- The file must be a JSON array (not an object)
- Each item must contain:
  - `ticker`: Stock/fund symbol (string)
  - `shares`: Number of shares owned (integer)
- Use the appropriate exchange suffix for non-US stocks (e.g., `.MI` for Milan Stock Exchange)

## Project Structure

```
fincli/
├── fincli.py              # Main CLI application and command definitions
├── ticker.py              # Ticker data model classes
├── portfolio.py           # Portfolio model with validation and aggregation
├── rich_functions.py      # Display formatting functions
├── portfolio.json         # Your portfolio holdings
├── LICENSE                # MIT License
└── README.md              # This file
```

## Examples

### Check Apple stock information
```bash
$ python fincli.py ticker AAPL
```

### View complete ticker details
```bash
$ python fincli.py ticker MSFT info
```

### Get specific attribute
```bash
$ python fincli.py ticker GOOGL -a marketCap
```

### View portfolio summary
```bash
$ python fincli.py portfolio -t
```

### Use custom portfolio file
```bash
$ python fincli.py portfolio -f ~/investments/my_portfolio.json
```

## License

MIT License - Copyright (c) 2024 Giorgio Cembran

See [LICENSE](LICENSE) for details.

## Author

Giorgio Cembran
