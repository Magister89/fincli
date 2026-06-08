use std::{env, path::PathBuf};

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};

use crate::{
    display::{self, TickerInfoRow},
    finance::{AttributeValue, Ticker},
    portfolio::Portfolio,
};

#[derive(Debug, Parser)]
#[command(name = "fincli")]
#[command(about = "A CLI tool for financial data")]
#[command(
    long_about = "FinCLI is a command-line tool to fetch stock/ETF data and manage portfolios."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(about = "Get information about a stock or ETF")]
    #[command(long_about = "Fetch and display financial data for a given ticker symbol.")]
    Ticker {
        symbol: String,
        #[arg(
            short = 'i',
            long = "info",
            conflicts_with = "attribute",
            help = "Show full ticker information"
        )]
        info: bool,
        #[arg(short = 'a', long = "attribute", help = "Show specific attribute")]
        attribute: Option<String>,
    },
    #[command(about = "Display portfolio with real-time values")]
    #[command(long_about = "Load a portfolio from JSON and display current values with P&L.")]
    Portfolio {
        #[arg(short = 't', long = "total", help = "Show only total portfolio value")]
        total: bool,
        #[arg(
            short = 'f',
            long = "file",
            value_name = "PATH",
            help = "Path to portfolio JSON file"
        )]
        file: Option<PathBuf>,
    },
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ticker {
            symbol,
            info,
            attribute,
        } => run_ticker(&symbol, info, attribute.as_deref()).await,
        Commands::Portfolio { total, file } => run_portfolio(total, file).await,
    }
}

async fn run_ticker(symbol: &str, show_info: bool, attribute: Option<&str>) -> Result<()> {
    let ticker = Ticker::new(symbol)
        .await
        .map_err(|err| anyhow!("fetching ticker data: {err}"))?;

    if let Some(attribute) = attribute {
        return print_attribute(symbol, &ticker, attribute);
    }

    if show_info {
        print_full_info(symbol, &ticker);
    } else {
        print_fast_info(symbol, &ticker);
    }

    Ok(())
}

async fn run_portfolio(show_total_only: bool, file: Option<PathBuf>) -> Result<()> {
    let file = file.unwrap_or_else(default_portfolio_path);
    let portfolio = Portfolio::new(file).await?;

    if !portfolio.skipped().is_empty() {
        let message = format!(
            "Warning: failed to fetch data for: {}",
            portfolio.skipped().join(", ")
        );
        eprintln!("{}", display::render_warning(&message));
    }

    if portfolio.is_single_currency() {
        let currency = portfolio.currency().unwrap_or("");
        if show_total_only {
            display::print_total_only(portfolio.total_value(), portfolio.total_pnl(), currency);
        } else {
            display::print_portfolio_table(
                portfolio.items(),
                true,
                portfolio.total_value(),
                portfolio.total_pnl(),
                currency,
            );
        }
    } else {
        let groups = portfolio.currency_groups();
        if show_total_only {
            display::print_multi_currency_total_only(&groups);
        } else {
            display::print_multi_currency_portfolio(&groups);
        }
    }

    display::print_cache_footer(portfolio.fetch_info());
    Ok(())
}

fn print_fast_info(symbol: &str, ticker: &Ticker) {
    let data = ticker.data();
    let rows = [
        row("lastPrice", AttributeValue::Float(data.last_price)),
        row("previousClose", AttributeValue::Float(data.previous_close)),
        row("open", AttributeValue::Float(data.open)),
        row("dayHigh", AttributeValue::Float(data.day_high)),
        row("dayLow", AttributeValue::Float(data.day_low)),
        row("volume", AttributeValue::Int(data.volume)),
        row("currency", AttributeValue::Text(data.currency.clone())),
    ];
    display::print_ticker_info(symbol, &rows);
}

fn print_full_info(symbol: &str, ticker: &Ticker) {
    let data = ticker.data();
    let rows = [
        row("lastPrice", AttributeValue::Float(data.last_price)),
        row("previousClose", AttributeValue::Float(data.previous_close)),
        row("open", AttributeValue::Float(data.open)),
        row("dayHigh", AttributeValue::Float(data.day_high)),
        row("dayLow", AttributeValue::Float(data.day_low)),
        row("volume", AttributeValue::Int(data.volume)),
        row("marketCap", AttributeValue::Int(data.market_cap)),
        row(
            "fiftyTwoWeekHigh",
            AttributeValue::Float(data.fifty_two_week_high),
        ),
        row(
            "fiftyTwoWeekLow",
            AttributeValue::Float(data.fifty_two_week_low),
        ),
        row("currency", AttributeValue::Text(data.currency.clone())),
    ];
    display::print_ticker_info(symbol, &rows);
}

fn print_attribute(symbol: &str, ticker: &Ticker, attribute: &str) -> Result<()> {
    let value = ticker
        .attribute(attribute)
        .ok_or_else(|| anyhow!("unknown attribute: {attribute}"))?;
    display::print_single_attribute(symbol, attribute, &format_value(value));
    Ok(())
}

fn row(attribute: &str, value: AttributeValue) -> TickerInfoRow {
    TickerInfoRow {
        attribute: attribute.to_string(),
        value: format_value(value),
    }
}

fn format_value(value: AttributeValue) -> String {
    match value {
        AttributeValue::Float(value) => display::format_with_thousands(value, 2),
        AttributeValue::Int(value) => display::format_int_with_thousands(value),
        AttributeValue::Text(value) => value,
    }
}

fn default_portfolio_path() -> PathBuf {
    home_dir()
        .map(|home| home.join(".fincli").join("portfolio.json"))
        .unwrap_or_else(|| PathBuf::from("portfolio.json"))
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
}
