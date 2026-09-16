use std::{fs, process::Command};

use chrono::Utc;
use serde_json::{Value, json};
use tempfile::TempDir;

fn sandbox(include_usd: bool, missing_price: bool) -> TempDir {
    let dir = tempfile::tempdir().expect("temporary HOME");
    let now = Utc::now().timestamp();
    let mut cache = serde_json::Map::new();
    let mut portfolio = Vec::new();
    for (symbol, currency, price, previous, kind, age, shares) in [
        ("REGULAR", "EUR", Some(100.0), Some(95.0), "ETF", 0, 10.0),
        (
            "OLDER",
            "EUR",
            Some(100.0),
            Some(95.0),
            "ETF",
            3 * 86400,
            10.0,
        ),
        (
            "PERIODIC",
            "EUR",
            Some(21.0),
            None,
            "MUTUALFUND",
            14 * 86400,
            100.0,
        ),
        ("DOLLAR", "USD", Some(20.0), Some(19.0), "EQUITY", 0, 10.0),
    ] {
        if symbol == "DOLLAR" && !include_usd {
            continue;
        }
        portfolio.push(json!({ "ticker": symbol, "shares": shares }));
        cache.insert(symbol.to_string(), json!({
            "timestamp": now,
            "data": { "symbol": symbol, "currency": currency,
                "lastPrice": if missing_price { None } else { price }, "previousClose": previous,
                "quoteTimeMs": (now - age) * 1000, "providerType": kind,
                "open": 0, "dayHigh": 0, "dayLow": 0, "volume": 0, "marketCap": 0,
                "fiftyTwoWeekHigh": 0, "fiftyTwoWeekLow": 0 }
        }));
    }
    if include_usd {
        portfolio.push(json!({ "ticker": "MISSING", "shares": 1 }));
    }
    fs::create_dir(dir.path().join(".fincli")).unwrap();
    fs::write(
        dir.path().join(".fincli/cache.json"),
        Value::Object(cache).to_string(),
    )
    .unwrap();
    fs::write(
        dir.path().join("portfolio.json"),
        json!(portfolio).to_string(),
    )
    .unwrap();
    dir
}

fn run(dir: &TempDir, total: bool, details: bool) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fincli"));
    command
        .env("HOME", dir.path())
        .env("USERPROFILE", dir.path())
        .env("NO_COLOR", "1");
    // Cache-only inputs; an intentionally missing ticker must fail without
    // contacting Yahoo or inheriting a corporate/system proxy.
    for key in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
    ] {
        command.env(key, "http://127.0.0.1:0");
    }
    command.env("NO_PROXY", "").env("no_proxy", "");
    command
        .arg("portfolio")
        .arg("--file")
        .arg(dir.path().join("portfolio.json"));
    if total {
        command.arg("--total");
    }
    if details {
        command.arg("--details");
    }
    let output = command.output().expect("run test binary");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut escape = false;
    String::from_utf8(output.stdout)
        .unwrap()
        .chars()
        .filter(|ch| {
            if *ch == '\u{1b}' {
                escape = true;
                return false;
            }
            if escape {
                if *ch == 'm' {
                    escape = false;
                }
                return false;
            }
            true
        })
        .collect()
}

fn compact_lines(stdout: &str) -> Vec<String> {
    for text in [
        "Session P&L",
        "Coverage:",
        "quoted",
        "period",
        "comparison",
        "Fetched:",
        "ref ",
    ] {
        assert!(!stdout.contains(text), "unexpected {text}: {stdout}");
    }
    stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

#[test]
fn real_cli_defaults_to_the_original_compact_table() {
    let home = sandbox(false, false);
    let stdout = run(&home, false, false);
    let lines = compact_lines(&stdout);
    assert_eq!(lines.len(), 8, "{stdout}");
    assert_eq!(
        &lines[..7],
        [
            "Ticker Qty Value P&L",
            "──────────── ──────── ────────────────── ────────────",
            "REGULAR 10 1,000.00 EUR ▲ 5.26%",
            "OLDER 10 1,000.00 EUR N/A",
            "PERIODIC 100 2,100.00 EUR N/A",
            "──────────── ──────── ────────────────── ────────────",
            "Total 4,100.00 EUR ▲ 5.26%",
        ]
    );
    assert!(lines[7].starts_with("Data from cache ("), "{stdout}");
}

#[test]
fn real_cli_total_defaults_to_only_the_summary_and_cache_footer() {
    let home = sandbox(false, false);
    let stdout = run(&home, true, false);
    let lines = compact_lines(&stdout);
    assert_eq!(lines.len(), 4, "{stdout}");
    assert_eq!(
        &lines[..3],
        [
            "Total Value P&L",
            "──────────────── ────────────",
            "4,100.00 EUR ▲ 5.26%",
        ]
    );
    assert!(lines[3].starts_with("Data from cache ("), "{stdout}");
}

#[test]
fn real_multicurrency_cli_defaults_to_compact_priced_subtotals() {
    let home = sandbox(true, false);
    for total in [false, true] {
        let stdout = run(&home, total, false);
        let lines = compact_lines(&stdout);
        if total {
            assert_eq!(lines.len(), 5, "{stdout}");
            assert_eq!(
                &lines[..4],
                [
                    "Priced subtotal P&L",
                    "──────────────── ────────────",
                    "4,100.00 EUR ▲ 5.26%",
                    "200.00 USD ▲ 5.26%",
                ]
            );
        } else {
            assert_eq!(lines.len(), 13, "{stdout}");
            let subtotals: Vec<_> = lines
                .iter()
                .filter(|line| line.starts_with("Subtotal"))
                .map(String::as_str)
                .collect();
            assert_eq!(
                subtotals,
                [
                    "Subtotal 4,100.00 EUR ▲ 5.26%",
                    "Subtotal 200.00 USD ▲ 5.26%"
                ]
            );
        }
    }
}

#[test]
fn real_cli_details_keeps_missing_baseline_context_and_dates_in_both_modes() {
    let home = sandbox(false, false);
    for total in [false, true] {
        let stdout = run(&home, total, true);
        for text in [
            "PERIODIC",
            "N/A comparison",
            "valuation period",
            "incomplete",
            "quoted",
            "cached",
            "Fetched:",
            "+50.00 EUR",
            "partial",
        ] {
            assert!(stdout.contains(text), "missing {text}: {stdout}");
        }
        assert!(stdout.contains("4,100.00"), "{stdout}");
        if !total {
            let older_row = stdout
                .lines()
                .find(|line| line.starts_with("OLDER"))
                .unwrap();
            assert!(
                older_row.contains("N/A"),
                "off-reference session row: {older_row}"
            );
        }
    }
}

#[test]
fn real_multicurrency_cli_details_does_not_lose_failed_quote_coverage() {
    let home = sandbox(true, false);
    for total in [false, true] {
        let stdout = run(&home, total, true);
        assert!(stdout.contains("EUR Coverage: N/A"), "{stdout}");
        assert!(stdout.contains("USD Coverage: N/A"), "{stdout}");
        assert!(!stdout.contains("100.0%"), "{stdout}");
        assert!(stdout.contains("N/A comparison"), "{stdout}");
        assert!(stdout.contains("Priced subtotal"), "{stdout}");
        if total {
            assert!(stdout.starts_with("Priced subtotal"), "{stdout}");
        }
    }
}

#[test]
fn real_cli_does_not_display_an_all_unpriced_portfolio_as_zero() {
    let home = sandbox(false, true);
    for total in [false, true] {
        for details in [false, true] {
            let stdout = run(&home, total, details);
            assert!(stdout.contains("N/A"), "{stdout}");
            assert!(!stdout.contains("0.00 EUR"), "{stdout}");
            if details {
                assert!(stdout.contains("unavailable"), "{stdout}");
            } else {
                let lines = compact_lines(&stdout);
                assert_eq!(lines.len(), if total { 4 } else { 8 }, "{stdout}");
            }
        }
    }
}
