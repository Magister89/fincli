//! Exercises the production pure normalizer and aggregation against the
//! canonical cross-repo quote policy fixture (copied byte-for-byte from
//! /tmp/portfolio-quote-policy-v1.json into tests/fixtures/quote-policy.json).

use std::fs;

use fincli::quote_policy::{self, PositionQuote};
use serde::Deserialize;

const FIXTURE_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/quote-policy.json"
);

#[derive(Deserialize)]
struct PolicyFixture {
    version: u32,
    now: i64,
    cases: Vec<FixtureCase>,
}

#[derive(Deserialize)]
struct FixtureCase {
    name: String,
    #[serde(default)]
    now: Option<i64>,
    quotes: Vec<FixtureQuote>,
    expected: FixtureExpected,
}

#[derive(Deserialize)]
struct FixtureQuote {
    #[allow(dead_code)]
    symbol: String,
    amount: f64,
    price: Option<f64>,
    #[serde(rename = "previousClose")]
    previous_close: Option<f64>,
    #[serde(rename = "quoteType")]
    quote_type: Option<String>,
    #[serde(rename = "quoteTime")]
    quote_time: Option<i64>,
    #[serde(rename = "fetchedAt")]
    fetched_at: Option<i64>,
    // `fromCache` is deliberately not mapped: cache state is not policy input,
    // freshness is carried by the quote/fetch timestamps.
}

#[derive(Deserialize)]
struct FixtureExpected {
    value: f64,
    pnl: Option<f64>,
    #[serde(rename = "pnlPercent")]
    pnl_percent: Option<f64>,
    #[serde(rename = "coveragePercent")]
    coverage_percent: Option<f64>,
    status: String,
}

fn assert_close(case: &str, field: &str, actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "{case}: {field} expected {expected}, got {actual}"
    );
}

fn assert_option_close(case: &str, field: &str, actual: Option<f64>, expected: Option<f64>) {
    match (actual, expected) {
        (Some(actual), Some(expected)) => assert_close(case, field, actual, expected),
        (None, None) => {}
        (actual, expected) => panic!("{case}: {field} expected {expected:?}, got {actual:?}"),
    }
}

#[test]
fn canonical_quote_policy_fixture_cases() {
    let raw = fs::read_to_string(FIXTURE_PATH).expect("read quote policy fixture");
    let fixture: PolicyFixture = serde_json::from_str(&raw).expect("parse quote policy fixture");

    assert_eq!(fixture.version, 1);
    assert_eq!(fixture.cases.len(), 14);

    for case in fixture.cases {
        let now = case.now.unwrap_or(fixture.now);

        let evaluations: Vec<_> = case
            .quotes
            .iter()
            .map(|quote| {
                quote_policy::evaluate_position(
                    PositionQuote {
                        shares: quote.amount,
                        price: quote.price,
                        previous_close: quote.previous_close,
                        provider_type: quote.quote_type.as_deref(),
                        quote_time: quote.quote_time,
                        fetched_at: quote.fetched_at,
                    },
                    now,
                )
            })
            .collect();

        let summary = quote_policy::aggregate(&evaluations, 0);

        assert_close(
            &case.name,
            "value",
            summary.total_value,
            case.expected.value,
        );
        assert_option_close(&case.name, "pnl", summary.session.amount, case.expected.pnl);
        assert_option_close(
            &case.name,
            "pnlPercent",
            summary.session.percent,
            case.expected.pnl_percent,
        );
        assert_option_close(
            &case.name,
            "coveragePercent",
            summary.coverage_percent,
            case.expected.coverage_percent,
        );
        assert_eq!(
            summary.session.status.as_str(),
            case.expected.status,
            "{}: status",
            case.name
        );
    }
}
