use super::*;
use crate::quote_policy::{QuoteQuality, aggregate, evaluate_position};
use serde_json::json;

fn at(time: &str) -> DateTime<Local> {
    DateTime::parse_from_rfc3339(time)
        .unwrap()
        .with_timezone(&Local)
}
fn now() -> DateTime<Local> {
    at("2026-11-11T12:00:00Z")
}
fn chart() -> YahooResult {
    serde_json::from_value(json!({
        "meta": {"symbol":"PERIODIC", "instrumentType":"MUTUALFUND", "currency":"EUR",
            "dataGranularity":"1d", "exchangeTimezoneName":"Europe/Berlin",
            "regularMarketPrice":22.06, "previousClose":21.284,
            "regularMarketTime":at("2026-10-15T20:00:00Z").timestamp()},
        "timestamp":[at("2026-10-15T06:00:00Z").timestamp(), at("2026-10-30T07:00:00Z").timestamp()],
        "indicators":{"quote":[{"close":[21.28396,21.19809]}]}
    })).unwrap()
}
fn primary() -> QuoteData {
    quote_from_meta("PERIODIC", &chart().meta, now())
}
fn historical() -> QuoteData {
    let mut quote = primary();
    quote.last_price = Some(21.19809);
    quote.previous_close = Some(21.28396);
    quote.quote_time_ms = Some(at("2026-10-30T07:00:00Z").timestamp_millis());
    quote.daily_close = true;
    quote
}
fn cached_client(quote: &QuoteData, timestamp: i64) -> (tempfile::TempDir, Client) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cache.json");
    let entry = CacheEntry {
        data: QuoteCache::from(quote),
        timestamp,
    };
    std::fs::write(
        &path,
        serde_json::to_string(&HashMap::from([("PERIODIC", entry)])).unwrap(),
    )
    .unwrap();
    let mut cache = Cache::with_path(path);
    cache.load();
    (
        dir,
        Client {
            http_client: reqwest::Client::new(),
            cache: Some(Arc::new(Mutex::new(cache))),
        },
    )
}

#[test]
fn funds_never_request_history_even_with_dated_missing_or_invalid_quotes() {
    let mut quote = primary();
    assert!(!needs_daily_close(&quote, now().timestamp_millis()));
    quote.quote_time_ms = Some(now().timestamp_millis());
    assert!(!needs_daily_close(&quote, now().timestamp_millis()));
    quote.last_price = None;
    assert!(!needs_daily_close(&quote, now().timestamp_millis()));
    quote.last_price = Some(22.06);
    quote.quote_time_ms = None;
    assert!(!needs_daily_close(&quote, now().timestamp_millis()));
    quote.quote_time_ms = Some(now().timestamp_millis() + 600_000);
    assert!(!needs_daily_close(&quote, now().timestamp_millis()));
}

#[test]
fn newer_nav_history_cannot_replace_or_restore_primary_fund_quotes() {
    let mut missing = primary();
    missing.last_price = None;
    for original in [
        primary(),
        missing,
        quote_from_meta("PERIODIC", &YahooMeta::default(), now()),
    ] {
        let mut quote = original.clone();
        assert!(apply_daily_close("PERIODIC", &chart(), &mut quote, now()).is_none());
        assert_eq!(quote.last_price, original.last_price);
        assert_eq!(quote.previous_close, original.previous_close);
        assert_eq!(quote.quote_time_ms, original.quote_time_ms);
        assert_eq!(quote.provider_type, original.provider_type);
        assert!(!quote.daily_close);
    }
}

#[test]
fn primary_fund_quotes_value_positions_but_never_join_session_pnl() {
    let mut quote = primary();
    assert_eq!(
        evaluate_position(quote.position_quote(100.0), now().timestamp_millis()).quality,
        QuoteQuality::Dated
    );
    for time in [quote.quote_time_ms, Some(now().timestamp_millis())] {
        quote.quote_time_ms = time;
        let fund = evaluate_position(quote.position_quote(100.0), now().timestamp_millis());
        assert_eq!(fund.comparison_period, ComparisonPeriod::ValuationPeriod);
        assert!(!fund.session_eligible);
        let alone = aggregate(std::slice::from_ref(&fund), 0);
        assert_eq!(alone.total_value, 2206.0);
        assert_eq!(alone.session.amount, None);
        assert_eq!(alone.session.reference_date, None);
        let mut equity = primary();
        equity.provider_type = Some("ETF".to_string());
        equity.last_price = Some(100.0);
        equity.previous_close = Some(95.0);
        equity.quote_time_ms = Some(now().timestamp_millis());
        let mixed = aggregate(
            &[
                fund,
                evaluate_position(equity.position_quote(10.0), now().timestamp_millis()),
            ],
            0,
        );
        assert_eq!(mixed.total_value, 3206.0);
        assert_eq!(mixed.session.amount, Some(50.0));
        assert_eq!(mixed.session.included_positions, 1);
    }
}

#[test]
fn cached_historical_navs_are_rejected_when_fresh_expired_or_missing_provider_metadata() {
    for age in [0, 600] {
        for kind in [Some("MUTUALFUND"), Some(" mutualfund "), None] {
            let mut quote = historical();
            quote.provider_type = kind.map(str::to_string);
            let (dir, client) = cached_client(&quote, Local::now().timestamp() - age);
            let path = dir.path().join("cache.json");
            let original = std::fs::read_to_string(&path).unwrap();
            assert_eq!(
                client
                    .cache
                    .as_ref()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .get_entry("PERIODIC")
                    .is_some(),
                age == 0
            );
            assert!(client.cached_quote("PERIODIC").is_none());
            assert!(client.last_valid_cached_quote("PERIODIC").is_none());
            assert!(
                client
                    .resolve_fetched_quote(
                        "PERIODIC",
                        Err(anyhow!("offline")),
                        now().timestamp_millis()
                    )
                    .is_err()
            );
            let mut missing = primary();
            missing.last_price = None;
            let result = client
                .resolve_fetched_quote("PERIODIC", Ok(missing), now().timestamp_millis())
                .unwrap();
            assert_eq!(result.last_price, None);
            assert!(!result.daily_close);
            assert_eq!(std::fs::read_to_string(path).unwrap(), original);
        }
    }
}

#[test]
fn older_primary_replaces_a_newer_historical_nav_cache_without_changing_source_date() {
    let (_dir, client) = cached_client(&historical(), Local::now().timestamp());
    let quote = client
        .resolve_fetched_quote("PERIODIC", Ok(primary()), now().timestamp_millis())
        .unwrap();
    assert_eq!(quote.last_price, Some(22.06));
    assert_eq!(quote.quote_time_ms, primary().quote_time_ms);
    assert_eq!(quote.fetched_at, primary().fetched_at);
    assert!(!quote.daily_close && !quote.from_cache);
    let cached = client.cached_quote("PERIODIC").unwrap();
    assert_eq!(cached.last_price, Some(22.06));
    assert_eq!(cached.quote_time_ms, primary().quote_time_ms);
    assert!(cached.from_cache && !cached.daily_close);
}

#[test]
fn primary_nav_cache_survives_failures_without_freshening_then_new_quotes_take_over() {
    let fetched_at = Local::now().timestamp() - 600;
    let (dir, client) = cached_client(&primary(), fetched_at);
    let path = dir.path().join("cache.json");
    let original = std::fs::read_to_string(&path).unwrap();
    let mut missing = primary();
    missing.last_price = None;
    for result in [Ok(missing), Err(anyhow!("offline"))] {
        let restored = client
            .resolve_fetched_quote("PERIODIC", result, now().timestamp_millis())
            .unwrap();
        assert_eq!(restored.last_price, Some(22.06));
        assert_eq!(restored.quote_time_ms, primary().quote_time_ms);
        assert_eq!(restored.fetched_at.unwrap().timestamp(), fetched_at);
        assert!(restored.from_cache && !restored.daily_close);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }
    let mut recovered = primary();
    recovered.last_price = Some(23.0);
    recovered.quote_time_ms = Some(now().timestamp_millis());
    let result = client
        .resolve_fetched_quote("PERIODIC", Ok(recovered), now().timestamp_millis())
        .unwrap();
    assert_eq!(result.last_price, Some(23.0));
    assert_eq!(result.quote_time_ms, Some(now().timestamp_millis()));
    assert!(!result.from_cache && !result.daily_close);
}

#[test]
fn missing_primary_baseline_is_not_filled_from_nav_history() {
    let mut quote = primary();
    quote.previous_close = None;
    assert!(apply_daily_close("PERIODIC", &chart(), &mut quote, now()).is_none());
    let evaluation = evaluate_position(quote.position_quote(100.0), now().timestamp_millis());
    assert_eq!(evaluation.comparison_percent, None);
    assert_eq!(evaluation.quality, QuoteQuality::Incomplete);
    assert_eq!(evaluation.value, Some(2206.0));
}

#[test]
fn primary_nav_ticker_exposes_quote_provenance_and_original_date() {
    let ticker = Ticker { data: primary() };
    assert!(
        matches!(ticker.attribute("priceSource"), Some(AttributeValue::Text(text)) if text == "quote")
    );
    assert!(
        matches!(ticker.attribute("quoteTime"), Some(AttributeValue::Text(text)) if text.starts_with("2026-10-15"))
    );
}
