use super::*;
use crate::quote_policy::{QuoteQuality, aggregate, evaluate_position};
use serde_json::{Value, json};

fn at(time: &str) -> DateTime<Local> {
    DateTime::parse_from_rfc3339(time)
        .unwrap()
        .with_timezone(&Local)
}
fn now() -> DateTime<Local> {
    at("2026-11-11T12:00:00Z")
}
fn nav_time() -> i64 {
    at("2026-10-30T07:00:00Z").timestamp_millis()
}
fn raw_chart() -> Value {
    json!({
        "meta": {"symbol":"PERIODIC", "instrumentType":"MUTUALFUND", "currency":"EUR",
            "dataGranularity":"1d", "exchangeTimezoneName":"Europe/Berlin",
            "regularMarketPrice":22.06, "previousClose":21.284, "chartPreviousClose":9999,
            "regularMarketTime":at("2026-10-15T20:00:00Z").timestamp()},
        "timestamp":[at("2026-10-15T06:00:00Z").timestamp(), nav_time() / 1000,
            at("2026-11-11T07:00:00Z").timestamp()],
        "indicators":{"quote":[{"close":[21.28396,21.19809,999]}], "adjclose":[{"adjclose":[1,2,3]}]}
    })
}
fn chart(raw: Value) -> YahooResult {
    serde_json::from_value(raw).unwrap()
}
fn primary() -> QuoteData {
    quote_from_meta("PERIODIC", &chart(raw_chart()).meta, now())
}
fn apply(raw: Value, mut quote: QuoteData, time: DateTime<Local>) -> QuoteData {
    apply_daily_close("PERIODIC", &chart(raw), &mut quote, time);
    quote
}

#[test]
fn dated_missing_and_invalid_navs_request_history_but_healthy_funds_do_not() {
    let mut quote = primary();
    assert!(needs_daily_close(&quote, now().timestamp_millis()));
    quote.quote_time_ms = Some(now().timestamp_millis());
    assert!(!needs_daily_close(&quote, now().timestamp_millis()));
    quote.last_price = None;
    assert!(needs_daily_close(&quote, now().timestamp_millis()));
    quote.last_price = Some(22.06);
    quote.quote_time_ms = None;
    assert!(needs_daily_close(&quote, now().timestamp_millis()));
    quote.quote_time_ms = Some(now().timestamp_millis() + 600_000);
    assert!(needs_daily_close(&quote, now().timestamp_millis()));
}

#[test]
fn latest_completed_raw_nav_wins_without_meta_adjusted_or_current_day_prices() {
    let quote = apply(raw_chart(), primary(), now());
    assert_eq!(quote.last_price, Some(21.19809));
    assert_eq!(quote.previous_close, Some(21.28396));
    assert_eq!(quote.quote_time_ms, Some(nav_time()));
    assert_eq!(quote.provider_type.as_deref(), Some("MUTUALFUND"));
    assert_eq!(quote.currency, "EUR");
    assert_eq!(quote.fetched_at, Some(now()));
    assert!(quote.is_historical_nav());
    assert!(!quote.from_cache);
    let evaluation = evaluate_position(quote.position_quote(100.0), now().timestamp_millis());
    assert_eq!(evaluation.quality, QuoteQuality::Dated);
    assert_eq!(
        evaluation.comparison_period,
        ComparisonPeriod::ValuationPeriod
    );
    assert!(
        (evaluation.comparison_percent.unwrap() - (21.19809 / 21.28396 - 1.0) * 100.0).abs() < 1e-9
    );
}

#[test]
fn recent_nav_never_joins_session_pnl_even_with_equity_on_the_same_date() {
    let time = at("2026-11-01T12:00:00Z");
    let quote = apply(raw_chart(), primary(), time);
    let fund = evaluate_position(quote.position_quote(100.0), time.timestamp_millis());
    assert_eq!(fund.quality, QuoteQuality::Available);
    assert!(!fund.session_eligible);
    let own = aggregate(std::slice::from_ref(&fund), 0);
    assert!((own.total_value - 2119.809).abs() < 1e-9);
    assert_eq!(own.session.reference_date, None);
    assert_eq!(own.session.amount, None);
    assert_eq!(own.session.percent, None);
    assert_eq!(own.session.included_positions, 0);
    let mut equity = primary();
    equity.provider_type = Some("ETF".to_string());
    equity.last_price = Some(100.0);
    equity.previous_close = Some(95.0);
    equity.quote_time_ms = Some(nav_time());
    equity.fetched_at = Some(time);
    let mixed = aggregate(
        &[
            fund,
            evaluate_position(equity.position_quote(10.0), time.timestamp_millis()),
        ],
        0,
    );
    assert!((mixed.total_value - 3119.809).abs() < 1e-9);
    assert_eq!(
        mixed.session.reference_date.unwrap().to_string(),
        "2026-10-30"
    );
    assert_eq!(mixed.session.amount, Some(50.0));
    assert_eq!(mixed.session.included_positions, 1);
}

#[test]
fn missing_adjacent_nav_baseline_is_neither_invented_nor_bridged() {
    let mut raw = raw_chart();
    raw["timestamp"]
        .as_array_mut()
        .unwrap()
        .insert(0, json!(at("2026-09-30T06:00:00Z").timestamp()));
    raw["indicators"]["quote"][0]["close"] = json!([21.18748, null, 21.19809, 999]);
    let quote = apply(raw, primary(), now());
    assert_eq!(quote.last_price, Some(21.19809));
    assert_eq!(quote.previous_close, None);
    let evaluation = evaluate_position(quote.position_quote(100.0), now().timestamp_millis());
    assert_eq!(
        evaluation.comparison_period,
        ComparisonPeriod::ValuationPeriod
    );
    assert_eq!(evaluation.comparison_percent, None);
    assert_eq!(evaluation.quality, QuoteQuality::Incomplete);
}

#[test]
fn missing_primary_recovers_fund_identity_but_not_session_eligibility() {
    let empty = quote_from_meta("PERIODIC", &YahooMeta::default(), now());
    let quote = apply(raw_chart(), empty, now());
    assert_eq!(quote.provider_type.as_deref(), Some("MUTUALFUND"));
    assert!(quote.is_historical_nav());
    assert!(
        !evaluate_position(quote.position_quote(100.0), now().timestamp_millis()).session_eligible
    );
}

#[test]
fn nav_identity_currency_type_and_date_safeguards_fail_closed() {
    for (field, value) in [
        ("symbol", json!("WRONG")),
        ("currency", json!("USD")),
        ("currency", Value::Null),
        ("instrumentType", json!("ETF")),
        ("instrumentType", json!("CURRENCY")),
        ("dataGranularity", json!("1mo")),
        ("exchangeTimezoneName", json!("invalid/timezone")),
    ] {
        let mut raw = raw_chart();
        raw["meta"][field] = value;
        let quote = apply(raw, primary(), now());
        assert!(!quote.daily_close, "{field}");
        assert_eq!(quote.last_price, Some(22.06));
    }
    for times in [vec![], vec![2, 1, 3], vec![1]] {
        let mut raw = raw_chart();
        raw["timestamp"] = json!(times);
        assert!(!apply(raw, primary(), now()).daily_close);
    }
    let mut raw = raw_chart();
    raw["indicators"]["quote"][0]["close"] = json!([null, -1, 999]);
    assert!(!apply(raw, primary(), now()).daily_close);
}

#[test]
fn equal_or_newer_valid_primary_navs_win_over_history() {
    for time in [nav_time(), now().timestamp_millis()] {
        let mut quote = primary();
        quote.quote_time_ms = Some(time);
        assert!(!apply(raw_chart(), quote, now()).daily_close);
    }
}

#[test]
fn cached_nav_provenance_and_dates_survive_failures_until_a_newer_primary_recovers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cache.json");
    let quote = apply(raw_chart(), primary(), now());
    let entry = CacheEntry {
        data: QuoteCache::from(&quote),
        timestamp: now().timestamp() - 600,
    };
    let original = serde_json::to_string(&HashMap::from([("PERIODIC", entry)])).unwrap();
    std::fs::write(&path, &original).unwrap();
    let mut cache = Cache::with_path(&path);
    cache.load();
    let client = Client {
        http_client: reqwest::Client::new(),
        cache: Some(Arc::new(Mutex::new(cache))),
    };
    let mut missing = primary();
    missing.last_price = None;
    for result in [
        Ok(primary()),
        Ok(missing),
        Err(anyhow!("NAV history outage")),
    ] {
        let restored = client
            .resolve_fetched_quote("PERIODIC", result, now().timestamp_millis())
            .unwrap();
        assert_eq!(restored.last_price, Some(21.19809));
        assert_eq!(restored.quote_time_ms, Some(nav_time()));
        assert_eq!(
            restored.fetched_at.unwrap().timestamp(),
            now().timestamp() - 600
        );
        assert!(restored.is_historical_nav() && restored.from_cache);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }
    let mut recovered = primary();
    recovered.last_price = Some(21.3);
    recovered.quote_time_ms = Some(now().timestamp_millis());
    let result = client
        .resolve_fetched_quote("PERIODIC", Ok(recovered), now().timestamp_millis())
        .unwrap();
    assert!(!result.daily_close && !result.from_cache);
    assert_eq!(result.last_price, Some(21.3));
    assert_eq!(result.provider_type.as_deref(), Some("MUTUALFUND"));
    assert_eq!(
        client
            .last_valid_cached_quote("PERIODIC")
            .unwrap()
            .last_price,
        Some(21.3)
    );
}

#[test]
fn ticker_labels_nav_history_and_hides_incompatible_intraday_fields() {
    let ticker = Ticker {
        data: apply(raw_chart(), primary(), now()),
    };
    for field in ["open", "dayHigh", "dayLow", "volume"] {
        assert!(
            matches!(ticker.attribute(field), Some(AttributeValue::Text(text)) if text == "N/A")
        );
    }
    assert!(
        matches!(ticker.attribute("priceSource"), Some(AttributeValue::Text(text)) if text == "historical NAV (not live)")
    );
    assert!(
        matches!(ticker.attribute("quoteTime"), Some(AttributeValue::Text(text)) if text.starts_with("2026-10-30"))
    );
}
