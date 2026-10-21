use super::*;
use crate::quote_policy::{QuoteQuality, aggregate, evaluate_position};
use serde_json::{Value, json};

fn at(time: &str) -> DateTime<Local> {
    DateTime::parse_from_rfc3339(time)
        .unwrap()
        .with_timezone(&Local)
}
fn now() -> DateTime<Local> {
    at("2026-10-21T10:00:00Z")
}
fn seconds(time: &str) -> i64 {
    at(time).timestamp()
}
fn raw_chart() -> Value {
    json!({
        "meta": {"symbol":"1GOOG.MI", "instrumentType":"EQUITY", "currency":"EUR",
            "dataGranularity":"1d", "exchangeTimezoneName":"Europe/Rome",
            "regularMarketPrice":301.1, "previousClose":300.4, "chartPreviousClose":9999,
            "regularMarketTime":seconds("2026-10-02T15:29:16Z")},
        "timestamp":[seconds("2026-10-19T07:00:00Z"), seconds("2026-10-20T07:00:00Z"), seconds("2026-10-21T07:00:00Z")],
        "indicators":{"quote":[{"close":[304.25,301.05,null]}], "adjclose":[{"adjclose":[1,2,3]}]}
    })
}
fn chart(raw: Value) -> YahooResult {
    serde_json::from_value(raw).unwrap()
}
fn primary() -> QuoteData {
    quote_from_meta("1GOOG.MI", &chart(raw_chart()).meta, now())
}
fn apply(raw: Value, mut quote: QuoteData, time: DateTime<Local>) -> QuoteData {
    apply_daily_close("1GOOG.MI", &chart(raw), &mut quote, time);
    quote
}

#[test]
fn frozen_quote_uses_daily_prices_and_dates_not_meta_or_adjusted_prices() {
    let quote = apply(raw_chart(), primary(), now());
    assert_eq!(quote.last_price, Some(301.05));
    assert_eq!(quote.previous_close, Some(304.25));
    assert_eq!(
        quote.quote_time_ms,
        Some(seconds("2026-10-20T07:00:00Z") * 1000)
    );
    assert_eq!(quote.fetched_at, Some(now()));
    assert!(quote.daily_close);
    assert!(!quote.from_cache);
    assert_eq!(
        evaluate_position(quote.position_quote(6.0), now().timestamp_millis()).quality,
        QuoteQuality::Available
    );
    assert_eq!(
        evaluate_position(
            quote.position_quote(6.0),
            now().timestamp_millis() + 5 * 86_400_000
        )
        .quality,
        QuoteQuality::Dated
    );
}

#[test]
fn freshly_fetching_an_old_daily_close_does_not_remove_its_dated_status() {
    let mut raw = raw_chart();
    for time in raw["timestamp"].as_array_mut().unwrap() {
        *time = json!(time.as_i64().unwrap() - 7 * 86400);
    }
    let quote = apply(raw, primary(), now());
    assert_eq!(quote.fetched_at, Some(now()));
    assert_eq!(
        evaluate_position(quote.position_quote(6.0), now().timestamp_millis()).quality,
        QuoteQuality::Dated
    );
}

#[test]
fn failed_or_older_refresh_never_rewrites_cache_timestamps_and_newer_primary_recovers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cache.json");
    let quote = apply(raw_chart(), primary(), now());
    let entry = CacheEntry {
        data: QuoteCache::from(&quote),
        timestamp: now().timestamp() - 600,
    };
    let original = serde_json::to_string(&HashMap::from([("1GOOG.MI", entry)])).unwrap();
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
        Err(anyhow!("network unavailable")),
    ] {
        let restored = client
            .resolve_fetched_quote("1GOOG.MI", result, now().timestamp_millis())
            .unwrap();
        assert_eq!(restored.last_price, Some(301.05));
        assert_eq!(restored.quote_time_ms, quote.quote_time_ms);
        assert_eq!(
            restored.fetched_at.unwrap().timestamp(),
            now().timestamp() - 600
        );
        assert!(restored.from_cache && restored.daily_close);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }
    let mut recovered = primary();
    recovered.quote_time_ms = Some(now().timestamp_millis());
    recovered.last_price = Some(302.0);
    let result = client
        .resolve_fetched_quote("1GOOG.MI", Ok(recovered), now().timestamp_millis())
        .unwrap();
    assert!(!result.from_cache && !result.daily_close);
    assert_eq!(result.last_price, Some(302.0));
    assert_eq!(
        client
            .last_valid_cached_quote("1GOOG.MI")
            .unwrap()
            .last_price,
        Some(302.0)
    );
}

#[test]
fn current_day_bar_is_never_a_completed_close_even_when_populated() {
    let mut raw = raw_chart();
    raw["indicators"]["quote"][0]["close"][2] = json!(999);
    assert_eq!(apply(raw, primary(), now()).last_price, Some(301.05));
}

#[test]
fn missing_adjacent_baseline_is_not_replaced_or_bridged() {
    let mut raw = raw_chart();
    raw["timestamp"]
        .as_array_mut()
        .unwrap()
        .insert(0, json!(seconds("2026-10-16T07:00:00Z")));
    raw["indicators"]["quote"][0]["close"] = json!([302.55, null, 301.05, null]);
    let quote = apply(raw, primary(), now());
    assert_eq!(quote.previous_close, None);
    assert_eq!(
        evaluate_position(quote.position_quote(6.0), now().timestamp_millis()).quality,
        QuoteQuality::Incomplete
    );
}

#[test]
fn latest_valid_completed_price_retains_its_own_date() {
    let mut raw = raw_chart();
    raw["indicators"]["quote"][0]["close"] = json!([304.25, 0, null]);
    let quote = apply(raw, primary(), now());
    assert_eq!(quote.last_price, Some(304.25));
    assert_eq!(quote.previous_close, None);
    assert_eq!(
        quote.quote_time_ms,
        Some(seconds("2026-10-19T07:00:00Z") * 1000)
    );
}

#[test]
fn fresh_quotes_weekends_and_funds_do_not_need_another_request() {
    let mut quote = primary();
    assert!(needs_daily_close(&quote, now().timestamp_millis()));
    quote.quote_time_ms = Some(now().timestamp_millis() - 3 * 86_400_000);
    assert!(!needs_daily_close(&quote, now().timestamp_millis()));
    quote.last_price = None;
    assert!(needs_daily_close(&quote, now().timestamp_millis()));
    quote.provider_type = Some("MUTUALFUND".to_string());
    assert!(!needs_daily_close(&quote, now().timestamp_millis()));
}

#[test]
fn wrong_symbol_type_currency_timezone_granularity_and_array_shapes_fail_closed() {
    for (field, value) in [
        ("symbol", json!("GOOG")),
        ("currency", json!("USD")),
        ("currency", Value::Null),
        ("instrumentType", json!("MUTUALFUND")),
        ("dataGranularity", json!("1wk")),
        ("exchangeTimezoneName", Value::Null),
        ("exchangeTimezoneName", json!("invalid/timezone")),
    ] {
        let mut raw = raw_chart();
        raw["meta"][field] = value;
        let quote = apply(raw, primary(), now());
        assert!(!quote.daily_close, "{field}");
        assert_eq!(quote.last_price, Some(301.1));
    }
    for times in [
        vec![],
        vec![0, 2, 3],
        vec![3, 2, 1],
        vec![1, 1, 2],
        vec![1],
        vec![i64::MAX, 2, 3],
    ] {
        let mut raw = raw_chart();
        raw["timestamp"] = json!(times);
        assert!(!apply(raw, primary(), now()).daily_close);
    }
    let mut raw = raw_chart();
    raw["indicators"]["quote"][0]["close"] = json!([null, -1, null]);
    assert!(!apply(raw, primary(), now()).daily_close);
}

#[test]
fn exchange_local_day_boundary_and_dst_exclude_partial_bars() {
    for (zone, time, yesterday, today) in [
        (
            "America/New_York",
            "2026-10-22T00:30:00Z",
            "2026-10-20T13:30:00Z",
            "2026-10-21T13:30:00Z",
        ),
        (
            "Asia/Tokyo",
            "2026-10-21T02:00:00Z",
            "2026-10-20T00:00:00Z",
            "2026-10-21T00:00:00Z",
        ),
        (
            "Europe/Rome",
            "2026-10-26T12:00:00Z",
            "2026-10-23T07:00:00Z",
            "2026-10-26T08:00:00Z",
        ),
    ] {
        let mut raw = raw_chart();
        raw["meta"]["exchangeTimezoneName"] = json!(zone);
        raw["timestamp"] = json!([seconds(yesterday), seconds(today)]);
        raw["indicators"]["quote"][0]["close"] = json!([100, 200]);
        assert_eq!(apply(raw, primary(), at(time)).last_price, Some(100.0));
    }
}

#[test]
fn cache_keeps_daily_close_provenance_and_dates_and_primary_recovery_wins() {
    let quote = apply(raw_chart(), primary(), now());
    let entry = CacheEntry {
        data: QuoteCache::from(&quote),
        timestamp: now().timestamp(),
    };
    let restored =
        quote_from_cache(serde_json::from_str(&serde_json::to_string(&entry).unwrap()).unwrap());
    assert!(restored.daily_close && restored.from_cache);
    assert_eq!(restored.quote_time_ms, quote.quote_time_ms);
    assert_eq!(restored.fetched_at, quote.fetched_at);
    assert!(keep_newer_cache(
        &primary(),
        &restored,
        now().timestamp_millis()
    ));
    let mut recovered = primary();
    recovered.quote_time_ms = Some(now().timestamp_millis());
    assert!(!keep_newer_cache(
        &recovered,
        &restored,
        now().timestamp_millis()
    ));
    assert!(!apply(raw_chart(), recovered, now()).daily_close);
    assert!(
        !quote_from_cache(CacheEntry {
            data: QuoteCache::from(&primary()),
            timestamp: now().timestamp()
        })
        .daily_close
    );
}

#[test]
fn missing_primary_is_recovered_from_chart_identity() {
    let empty = quote_from_meta("1GOOG.MI", &YahooMeta::default(), now());
    let quote = apply(raw_chart(), empty, now());
    assert_eq!(quote.last_price, Some(301.05));
    assert_eq!(quote.provider_type.as_deref(), Some("EQUITY"));
    assert_eq!(quote.currency, "EUR");
}

#[test]
fn recent_close_is_not_dated_but_never_added_to_a_different_session() {
    let quote = apply(raw_chart(), primary(), now());
    let close = evaluate_position(quote.position_quote(6.0), now().timestamp_millis());
    let own = aggregate(std::slice::from_ref(&close), 0);
    assert_eq!(
        own.session.reference_date.unwrap().to_string(),
        "2026-10-20"
    );
    assert!((own.session.amount.unwrap() - (301.05 - 304.25) * 6.0).abs() < 1e-6);
    let mut regular = primary();
    regular.last_price = Some(100.0);
    regular.previous_close = Some(90.0);
    regular.quote_time_ms = Some(now().timestamp_millis());
    let summary = aggregate(
        &[
            close,
            evaluate_position(regular.position_quote(1.0), now().timestamp_millis()),
        ],
        0,
    );
    assert_eq!(summary.session.amount, Some(10.0));
    assert_eq!(summary.session.included_positions, 1);
    assert!((summary.total_value - (6.0 * 301.05 + 100.0)).abs() < 1e-6);
}

#[test]
fn ticker_does_not_mix_frozen_intraday_fields_with_a_new_daily_price() {
    let ticker = Ticker {
        data: apply(raw_chart(), primary(), now()),
    };
    for field in ["open", "dayHigh", "dayLow", "volume"] {
        assert!(
            matches!(ticker.attribute(field), Some(AttributeValue::Text(text)) if text == "N/A")
        );
    }
    assert!(
        matches!(ticker.attribute("priceSource"), Some(AttributeValue::Text(text)) if text.contains("not live"))
    );
}
