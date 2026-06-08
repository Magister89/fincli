use std::time::Duration;

use crate::portfolio::{CurrencyGroup, EnrichedItem, FetchInfo};

const GREEN: &str = "\x1b[38;2;166;209;137m";
const RED: &str = "\x1b[38;2;231;130;132m";
const BLUE: &str = "\x1b[38;2;140;170;238m";
const TEXT_BOLD: &str = "\x1b[1;38;2;198;208;245m";
const HEADER: &str = "\x1b[1;38;2;186;187;241m";
const DIM: &str = "\x1b[38;2;115;121;148m";
const WARNING: &str = "\x1b[38;2;229;200;144m";
const RESET: &str = "\x1b[0m";

const COL_TICKER: usize = 12;
const COL_QTY: usize = 8;
const COL_VALUE: usize = 18;
const COL_PNL: usize = 12;
const COL_ATTR: usize = 18;
const COL_VAL: usize = 14;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickerInfoRow {
    pub attribute: String,
    pub value: String,
}

pub fn format_with_thousands(value: f64, decimals: usize) -> String {
    let formatted = format!("{value:.decimals$}");
    let (integer, decimal) = formatted
        .split_once('.')
        .map_or((formatted.as_str(), None), |(integer, decimal)| {
            (integer, Some(decimal))
        });

    let (negative, digits) = integer
        .strip_prefix('-')
        .map_or((false, integer), |digits| (true, digits));

    let mut grouped_reversed = String::new();
    for (index, ch) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            grouped_reversed.push(',');
        }
        grouped_reversed.push(ch);
    }

    let grouped = grouped_reversed.chars().rev().collect::<String>();
    let sign = if negative { "-" } else { "" };

    match decimal {
        Some(decimal) => format!("{sign}{grouped}.{decimal}"),
        None => format!("{sign}{grouped}"),
    }
}

pub fn format_int_with_thousands(value: i64) -> String {
    let string = value.to_string();
    let (negative, digits) = string
        .strip_prefix('-')
        .map_or((false, string.as_str()), |digits| (true, digits));

    let mut grouped_reversed = String::new();
    for (index, ch) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            grouped_reversed.push(',');
        }
        grouped_reversed.push(ch);
    }

    let grouped = grouped_reversed.chars().rev().collect::<String>();
    if negative {
        format!("-{grouped}")
    } else {
        grouped
    }
}

pub fn print_portfolio_table(
    items: &[EnrichedItem],
    show_total: bool,
    total_value: f64,
    total_pnl: f64,
    currency: &str,
) {
    print_header();
    let separator = separator();
    println!("{}", dim(&separator));

    for item in items {
        print_item(item);
    }

    if show_total {
        println!("{}", dim(&separator));
        print_total("Total", total_value, total_pnl, currency);
    }
}

pub fn print_multi_currency_portfolio(groups: &[CurrencyGroup]) {
    print_header();
    let separator = separator();
    println!("{}", dim(&separator));

    for (index, group) in groups.iter().enumerate() {
        for item in &group.items {
            print_item(item);
        }

        println!("{}", dim(&separator));
        print_total(
            "Subtotal",
            group.total_value,
            group.total_pnl,
            &group.currency,
        );

        if index < groups.len() - 1 {
            println!();
            print_header();
            println!("{}", dim(&separator));
        }
    }
}

pub fn print_total_only(total_value: f64, total_pnl: f64, currency: &str) {
    let header = format!("{:<16}  {:<12}", "Total Value", "P&L");
    println!("{}", header_style(&header));

    let separator = format!("{:<16}  {:<12}", "────────────────", "────────────");
    println!("{}", dim(&separator));

    let formatted_value = format_with_thousands(total_value, 2);
    let value = format!("{:>12} {currency}", formatted_value);
    println!("{}  {}", bold(&value), format_pnl(total_pnl));
}

pub fn print_multi_currency_total_only(groups: &[CurrencyGroup]) {
    let header = format!("{:<16}  {:<12}", "Total Value", "P&L");
    println!("{}", header_style(&header));

    let separator = format!("{:<16}  {:<12}", "────────────────", "────────────");
    println!("{}", dim(&separator));

    for group in groups {
        let formatted_value = format_with_thousands(group.total_value, 2);
        let value = format!("{:>12} {}", formatted_value, group.currency);
        println!("{}  {}", bold(&value), format_pnl(group.total_pnl));
    }
}

pub fn print_ticker_info(symbol: &str, rows: &[TickerInfoRow]) {
    println!("{}\n", bold(symbol));

    let header = format!("{:<COL_ATTR$}  {:<COL_VAL$}", "Attribute", "Value");
    println!("{}", header_style(&header));

    let separator = format!(
        "{:<COL_ATTR$}  {:<COL_VAL$}",
        "──────────────────", "──────────────"
    );
    println!("{}", dim(&separator));

    for row in rows {
        let attribute = format!("{:<COL_ATTR$}", row.attribute);
        let value = format!("{:>COL_VAL$}", row.value);
        println!("{}  {}", blue(&attribute), value);
    }
}

pub fn print_single_attribute(symbol: &str, attribute: &str, value: &str) {
    println!("{}\n", bold(symbol));

    let header = format!("{:<COL_ATTR$}  {:<COL_VAL$}", "Attribute", "Value");
    println!("{}", header_style(&header));

    let separator = format!(
        "{:<COL_ATTR$}  {:<COL_VAL$}",
        "──────────────────", "──────────────"
    );
    println!("{}", dim(&separator));

    let attribute = format!("{:<COL_ATTR$}", attribute);
    let value = format!("{:>COL_VAL$}", value);
    println!("{}  {}", blue(&attribute), value);
}

pub fn print_cache_footer(info: &FetchInfo) {
    let Some(oldest) = info.oldest_fetched_at else {
        return;
    };
    let newest = info.newest_fetched_at.unwrap_or(oldest);
    let now = chrono::Local::now();

    let message = if info.all_from_cache {
        let age = now
            .signed_duration_since(oldest)
            .to_std()
            .unwrap_or_default();
        format!("Data from cache ({})", format_duration(age))
    } else if info.any_from_cache {
        let oldest_age = now
            .signed_duration_since(oldest)
            .to_std()
            .unwrap_or_default();
        format!(
            "Last updated: {} (oldest data: {})",
            newest.format("%H:%M:%S"),
            format_duration(oldest_age)
        )
    } else {
        format!("Last updated: {}", newest.format("%H:%M:%S"))
    };

    println!("\n{}", dim(&message));
}

pub fn render_warning(message: &str) -> String {
    paint(WARNING, message)
}

fn print_header() {
    let header = format!(
        "{:<COL_TICKER$}  {:>COL_QTY$}  {:<COL_VALUE$}  {:<COL_PNL$}",
        "Ticker", "Qty", "Value", "P&L"
    );
    println!("{}", header_style(&header));
}

fn separator() -> String {
    format!(
        "{:<COL_TICKER$}  {:>COL_QTY$}  {:<COL_VALUE$}  {:<COL_PNL$}",
        "────────────", "────────", "──────────────────", "────────────"
    )
}

fn print_item(item: &EnrichedItem) {
    let ticker = format!("{:<COL_TICKER$}", item.ticker);
    let quantity = format!("{:>COL_QTY$}", format_int_with_thousands(item.shares));
    let formatted_value = format_with_thousands(item.price, 2);
    let value = format!(
        "{:>width$} {}",
        formatted_value,
        item.currency,
        width = COL_VALUE - 4
    );
    let pnl = format_pnl(item.pnl);

    println!("{}  {}  {}  {}", blue(&ticker), quantity, value, pnl);
}

fn print_total(label: &str, value: f64, pnl: f64, currency: &str) {
    let total_label = format!("{:<COL_TICKER$}", label);
    let quantity = format!("{:>COL_QTY$}", "");
    let formatted_value = format_with_thousands(value, 2);
    let total_value = format!(
        "{:>width$} {currency}",
        formatted_value,
        width = COL_VALUE - 4
    );
    let total_pnl = format_pnl(pnl);

    println!(
        "{}  {}  {}  {}",
        bold(&total_label),
        quantity,
        bold(&total_value),
        total_pnl
    );
}

fn format_pnl(pnl: f64) -> String {
    let (arrow, color) = if pnl >= 0.0 {
        ("▲", GREEN)
    } else {
        ("▼", RED)
    };
    let raw = format!("{arrow} {pnl:.2}%");
    paint(color, &format!("{:>COL_PNL$}", raw))
}

fn format_duration(duration: Duration) -> String {
    if duration < Duration::from_secs(5) {
        "just now".to_string()
    } else if duration < Duration::from_secs(60) {
        format!("{} sec ago", duration.as_secs())
    } else {
        format!("{} min ago", duration.as_secs() / 60)
    }
}

fn blue(value: &str) -> String {
    paint(BLUE, value)
}

fn bold(value: &str) -> String {
    paint(TEXT_BOLD, value)
}

fn header_style(value: &str) -> String {
    paint(HEADER, value)
}

fn dim(value: &str) -> String {
    paint(DIM, value)
}

fn paint(style: &str, value: &str) -> String {
    format!("{style}{value}{RESET}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_float_with_thousands() {
        let tests = [
            (0.0, 2, "0.00"),
            (123.45, 2, "123.45"),
            (1234.56, 2, "1,234.56"),
            (1234567.89, 2, "1,234,567.89"),
            (-1234.56, 2, "-1,234.56"),
            (1234567.0, 0, "1,234,567"),
            (-9876543.21, 2, "-9,876,543.21"),
            (1234.5, 1, "1,234.5"),
        ];

        for (value, decimals, expected) in tests {
            assert_eq!(format_with_thousands(value, decimals), expected);
        }
    }

    #[test]
    fn format_ints_with_thousands() {
        let tests = [
            (0, "0"),
            (123, "123"),
            (1234, "1,234"),
            (1234567, "1,234,567"),
            (1234567890, "1,234,567,890"),
            (-1234, "-1,234"),
            (-1234567, "-1,234,567"),
        ];

        for (value, expected) in tests {
            assert_eq!(format_int_with_thousands(value), expected);
        }
    }

    #[test]
    fn duration_formatting_matches_go_version() {
        assert_eq!(format_duration(Duration::from_secs(2)), "just now");
        assert_eq!(format_duration(Duration::from_secs(12)), "12 sec ago");
        assert_eq!(format_duration(Duration::from_secs(120)), "2 min ago");
    }
}
