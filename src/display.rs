use std::time::Duration;

use chrono::NaiveDate;

use crate::portfolio::{CurrencyGroup, EnrichedItem, FetchInfo};
use crate::quote_policy::PortfolioSummary;

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
    if !value.is_finite() {
        return "N/A".to_string();
    }
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

pub fn format_quantity(value: f64) -> String {
    format_with_thousands(value, 8)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

pub fn print_portfolio_table(
    items: &[EnrichedItem],
    summary: &PortfolioSummary,
    currency: &str,
    details: bool,
) {
    print_header(details);
    let separator = separator();
    println!("{}", dim(&separator));

    for item in items {
        print_item(item, summary.session.reference_date, details);
    }

    println!("{}", dim(&separator));
    print_total(
        if summary.unvalued_positions > 0 {
            "Subtotal"
        } else {
            "Total"
        },
        summary,
        currency,
    );
    if details {
        print_summary_lines(summary, currency, "");
        print_valuation_comparisons(items, summary.session.reference_date);
    }
}

pub fn print_multi_currency_portfolio(groups: &[CurrencyGroup], details: bool) {
    print_header(details);
    let separator = separator();
    println!("{}", dim(&separator));

    for (index, group) in groups.iter().enumerate() {
        for item in &group.items {
            print_item(item, group.summary.session.reference_date, details);
        }

        println!("{}", dim(&separator));
        print_total("Subtotal", &group.summary, &group.currency);
        if details {
            print_summary_lines(
                &group.summary,
                &group.currency,
                &format!("{} ", group.currency),
            );
            print_valuation_comparisons(&group.items, group.summary.session.reference_date);
        }

        if index < groups.len() - 1 {
            println!();
            print_header(details);
            println!("{}", dim(&separator));
        }
    }
}

pub fn print_total_only(
    summary: &PortfolioSummary,
    currency: &str,
    items: &[EnrichedItem],
    details: bool,
) {
    let label = if summary.unvalued_positions > 0 {
        "Priced subtotal"
    } else {
        "Total Value"
    };
    let header = format!("{label:<16}  {:<12}", pnl_heading(details));
    println!("{}", header_style(&header));

    let separator = format!("{:<16}  {:<12}", "────────────────", "────────────");
    println!("{}", dim(&separator));

    let formatted_value = valuation_label(summary);
    let value = format!("{:>12} {currency}", formatted_value);
    println!("{}  {}", bold(&value), format_pnl(summary.session.percent));

    if details {
        print_summary_lines(summary, currency, "");
        print_valuation_comparisons(items, summary.session.reference_date);
    }
}

pub fn print_multi_currency_total_only(groups: &[CurrencyGroup], details: bool) {
    let label = if groups
        .iter()
        .any(|group| group.summary.unvalued_positions > 0)
    {
        "Priced subtotal"
    } else {
        "Total Value"
    };
    let header = format!("{label:<16}  {:<12}", pnl_heading(details));
    println!("{}", header_style(&header));

    let separator = format!("{:<16}  {:<12}", "────────────────", "────────────");
    println!("{}", dim(&separator));

    for group in groups {
        let formatted_value = valuation_label(&group.summary);
        let value = format!("{:>12} {}", formatted_value, group.currency);
        println!(
            "{}  {}",
            bold(&value),
            format_pnl(group.summary.session.percent)
        );
    }

    if details {
        for group in groups {
            print_summary_lines(
                &group.summary,
                &group.currency,
                &format!("{} ", group.currency),
            );
        }
        for group in groups {
            print_valuation_comparisons(&group.items, group.summary.session.reference_date);
        }
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

pub fn print_cache_footer(info: &FetchInfo, details: bool) {
    let Some(oldest) = info.oldest_fetched_at else {
        if details {
            println!("\n{}", dim("Fetched: N/A (original timestamp unavailable)"));
        }
        return;
    };
    let newest = info.newest_fetched_at.unwrap_or(oldest);
    let now = chrono::Local::now();
    let fetched_label = if details { "Fetched" } else { "Last updated" };

    let message = if info.all_from_cache {
        let age = now
            .signed_duration_since(oldest)
            .to_std()
            .unwrap_or_default();
        let fetched = if details {
            format!(" · Fetched: {}", oldest.format("%Y-%m-%d %H:%M:%S %:z"))
        } else {
            String::new()
        };
        format!("Data from cache{fetched} ({})", format_duration(age))
    } else if info.any_from_cache {
        let oldest_age = now
            .signed_duration_since(oldest)
            .to_std()
            .unwrap_or_default();
        let oldest_label = if details {
            "oldest fetch"
        } else {
            "oldest data"
        };
        format!(
            "{fetched_label}: {} ({oldest_label}: {})",
            newest.format("%H:%M:%S"),
            format_duration(oldest_age)
        )
    } else {
        format!("{fetched_label}: {}", newest.format("%H:%M:%S"))
    };

    println!("\n{}", dim(&message));
}

pub fn render_warning(message: &str) -> String {
    paint(WARNING, message)
}

fn pnl_heading(details: bool) -> &'static str {
    if details { "Session P&L" } else { "P&L" }
}

fn print_header(details: bool) {
    let header = format!(
        "{:<COL_TICKER$}  {:>COL_QTY$}  {:<COL_VALUE$}  {:<COL_PNL$}",
        "Ticker",
        "Qty",
        "Value",
        pnl_heading(details)
    );
    println!("{}", header_style(&header));
}

fn separator() -> String {
    format!(
        "{:<COL_TICKER$}  {:>COL_QTY$}  {:<COL_VALUE$}  {:<COL_PNL$}",
        "────────────", "────────", "──────────────────", "────────────"
    )
}

fn included_percent(item: &EnrichedItem, reference: Option<NaiveDate>) -> Option<f64> {
    item.evaluation
        .session_percent
        .filter(|_| reference.is_some() && item.evaluation.session_date == reference)
}

fn quote_details(item: &EnrichedItem) -> String {
    let date = item
        .evaluation
        .comparison_date
        .map(|date| date.to_string())
        .unwrap_or_else(|| "N/A".to_string());
    format!(
        "{} · quoted {date} UTC · {}{}",
        item.evaluation.comparison_period.label(),
        item.evaluation.quality.label(),
        if item.from_cache { " · cached" } else { "" }
    )
}

fn print_item(item: &EnrichedItem, reference: Option<NaiveDate>, details: bool) {
    let ticker = format!("{:<COL_TICKER$}", item.ticker);
    let quantity = format!("{:>COL_QTY$}", format_quantity(item.shares));
    let value = match item.evaluation.value {
        Some(value) => format!(
            "{:>width$} {}",
            format_with_thousands(value, 2),
            item.currency,
            width = COL_VALUE - 4
        ),
        None => dim(&format!("{:>COL_VALUE$}", "N/A")),
    };
    let pnl = format_pnl(included_percent(item, reference));

    println!("{}  {}  {}  {}", blue(&ticker), quantity, value, pnl);
    if details {
        println!("  {}", dim(&quote_details(item)));
    }
}

fn valuation_label(summary: &PortfolioSummary) -> String {
    if summary.session.total_positions > 0
        && summary.unvalued_positions == summary.session.total_positions
    {
        "N/A".to_string()
    } else {
        format_with_thousands(summary.total_value, 2)
    }
}

fn print_total(label: &str, summary: &PortfolioSummary, currency: &str) {
    let total_label = format!("{:<COL_TICKER$}", label);
    let quantity = format!("{:>COL_QTY$}", "");
    let formatted_value = valuation_label(summary);
    let total_value = format!(
        "{:>width$} {currency}",
        formatted_value,
        width = COL_VALUE - 4
    );
    let total_pnl = format_pnl(summary.session.percent);

    println!(
        "{}  {}  {}  {}",
        bold(&total_label),
        quantity,
        bold(&total_value),
        total_pnl
    );
}

/// Session aggregate and coverage, labeled with the reference UTC session
/// date so it can never be mistaken for "today" or a 24h change.
fn print_summary_lines(summary: &PortfolioSummary, currency: &str, prefix: &str) {
    let session = &summary.session;
    if summary.unvalued_positions > 0 {
        println!(
            "{}",
            dim(&format!(
                "{prefix}Priced subtotal only: {} requested positions have unknown valuation/currency.",
                summary.unvalued_positions
            ))
        );
    }
    let reference = session
        .reference_date
        .map(|date| format!(" · ref {} UTC", date.format("%Y-%m-%d")))
        .unwrap_or_default();
    let label = format!(
        "{prefix}Session P&L ({}{reference})",
        session.status.as_str()
    );

    match session.amount {
        Some(amount) => {
            let percent = session
                .percent
                .map(|value| format!(" {}", format_percent_signed(value)))
                .unwrap_or_default();
            println!(
                "{}: {}{}",
                dim(&label),
                format_amount_signed(amount, currency),
                percent
            );
        }
        None => println!(
            "{}: {}",
            dim(&label),
            dim("N/A (no comparable session data)")
        ),
    }

    let coverage = summary
        .coverage_percent
        .map(|value| format!("{value:.1}%"))
        .unwrap_or_else(|| "N/A".to_string());
    println!(
        "{}",
        dim(&format!(
            "{prefix}Coverage: {coverage} of priced value · {}/{} positions",
            session.included_positions, session.total_positions
        ))
    );
}

/// Raw price/baseline moves that are not session P&L (funds, stale or
/// unclassified quotes), shown with their own source date and period.
fn comparison_line(item: &EnrichedItem) -> String {
    let change = match (
        item.evaluation.comparison_amount,
        item.evaluation.comparison_percent,
    ) {
        (Some(amount), Some(percent)) => format!(
            "{} {}",
            format_amount_signed(amount, &item.currency),
            format_percent_signed(percent)
        ),
        _ => dim("N/A comparison"),
    };
    format!(
        "  {} {change} · {}",
        blue(&item.ticker),
        dim(&quote_details(item))
    )
}

fn print_valuation_comparisons(items: &[EnrichedItem], reference: Option<NaiveDate>) {
    let comparisons: Vec<_> = items
        .iter()
        .filter(|item| included_percent(item, reference).is_none())
        .collect();
    if comparisons.is_empty() {
        return;
    }
    println!(
        "{}",
        dim("Other quote comparisons (not aggregate session P&L):")
    );
    for item in comparisons {
        println!("{}", comparison_line(item));
    }
}

fn format_pnl(pnl: Option<f64>) -> String {
    let Some(pnl) = pnl else {
        return paint(DIM, &format!("{:>COL_PNL$}", "N/A"));
    };
    let (arrow, color) = pnl_style(pnl);
    let raw = format!("{arrow} {pnl:.2}%");
    paint(color, &format!("{:>COL_PNL$}", raw))
}

fn pnl_style(value: f64) -> (&'static str, &'static str) {
    if value >= 0.0 {
        ("▲", GREEN)
    } else {
        ("▼", RED)
    }
}

fn format_signed(value: f64) -> String {
    let sign = if value < 0.0 { "-" } else { "+" };
    format!("{sign}{}", format_with_thousands(value.abs(), 2))
}

fn format_amount_signed(value: f64, currency: &str) -> String {
    let (arrow, color) = pnl_style(value);
    paint(
        color,
        &format!("{arrow} {} {currency}", format_signed(value)),
    )
}

fn format_percent_signed(value: f64) -> String {
    let (_, color) = pnl_style(value);
    paint(color, &format!("({:+.2}%)", value))
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
    fn format_fractional_quantities() {
        let tests = [
            (218.0, "218"),
            (756.344, "756.344"),
            (1234.5, "1,234.5"),
            (0.00000001, "0.00000001"),
            (1.23000000, "1.23"),
        ];

        for (value, expected) in tests {
            assert_eq!(format_quantity(value), expected);
        }
    }

    #[test]
    fn duration_formatting_matches_go_version() {
        assert_eq!(format_duration(Duration::from_secs(2)), "just now");
        assert_eq!(format_duration(Duration::from_secs(12)), "12 sec ago");
        assert_eq!(format_duration(Duration::from_secs(120)), "2 min ago");
    }

    #[test]
    fn unknown_pnl_is_neutral_na() {
        let rendered = format_pnl(None);
        assert!(rendered.contains("N/A"));
        assert!(!rendered.contains(GREEN));
        assert!(!rendered.contains(RED));
    }

    #[test]
    fn real_zero_pnl_is_rendered_as_a_number() {
        let rendered = format_pnl(Some(0.0));
        assert!(rendered.contains("0.00%"));
    }

    #[test]
    fn missing_fund_baseline_still_shows_quote_context() {
        let now = 1_789_387_200_000;
        let item = EnrichedItem {
            ticker: "PERIODIC".to_string(),
            shares: 100.0,
            currency: "EUR".to_string(),
            from_cache: true,
            evaluation: crate::quote_policy::evaluate_position(
                crate::quote_policy::PositionQuote {
                    shares: 100.0,
                    price: Some(21.0),
                    previous_close: None,
                    provider_type: Some("MUTUALFUND"),
                    quote_time: Some(now),
                    fetched_at: Some(now),
                },
                now,
            ),
        };
        let line = comparison_line(&item);
        for text in [
            "PERIODIC",
            "N/A comparison",
            "valuation period",
            "quoted 2026-09-14 UTC",
            "incomplete",
            "cached",
        ] {
            assert!(line.contains(text), "{text}: {line}");
        }
    }

    #[test]
    fn an_older_session_row_is_not_part_of_the_current_pnl() {
        let now = 1_789_387_200_000;
        let item = EnrichedItem {
            ticker: "OLDER".to_string(),
            shares: 10.0,
            currency: "EUR".to_string(),
            from_cache: false,
            evaluation: crate::quote_policy::evaluate_position(
                crate::quote_policy::PositionQuote {
                    shares: 10.0,
                    price: Some(100.0),
                    previous_close: Some(95.0),
                    provider_type: Some("ETF"),
                    quote_time: Some(now - 3 * 86_400_000),
                    fetched_at: Some(now),
                },
                now,
            ),
        };
        assert_eq!(
            included_percent(&item, NaiveDate::from_ymd_opt(2026, 9, 14)),
            None
        );
        assert!(comparison_line(&item).contains("quoted 2026-09-11 UTC"));
        assert_eq!(format_with_thousands(f64::INFINITY, 2), "N/A");
    }

    #[test]
    fn signed_amounts_keep_thousands_separator() {
        assert_eq!(format_signed(1234.5), "+1,234.50");
        assert_eq!(format_signed(-9_876_543.21), "-9,876,543.21");
        assert_eq!(format_signed(0.0), "+0.00");
    }
}
