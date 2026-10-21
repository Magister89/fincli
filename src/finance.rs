use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Local, TimeZone, Utc};
use futures::{StreamExt, stream};
use serde::Deserialize;

use crate::cache::{Cache, CacheEntry, QuoteCache};
use crate::quote_policy::{
    ComparisonPeriod, PositionQuote, comparison_period, quote_time_is_recent, sanitize_positive,
};

const CHART_BASE_URL: &str = "https://query1.finance.yahoo.com/v8/finance/chart";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36";
const MAX_CONCURRENT: usize = 10;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
pub struct QuoteData {
    pub symbol: String,
    pub last_price: Option<f64>,
    pub previous_close: Option<f64>,
    pub currency: String,
    pub open: f64,
    pub day_high: f64,
    pub day_low: f64,
    pub volume: i64,
    pub market_cap: i64,
    pub fifty_two_week_high: f64,
    pub fifty_two_week_low: f64,
    /// Provider quote time, Unix milliseconds.
    pub quote_time_ms: Option<i64>,
    /// Provider instrument/quote type as reported (e.g. "ETF").
    pub provider_type: Option<String>,
    pub fetched_at: Option<DateTime<Local>>,
    pub from_cache: bool,
    /// True when price and baseline come from completed daily chart bars.
    pub daily_close: bool,
}

impl QuoteData {
    /// Build the pure-policy input for a position of `shares` in this quote.
    /// Cache state is not part of the policy: freshness is carried by the
    /// timestamps alone.
    pub fn position_quote(&self, shares: f64) -> PositionQuote<'_> {
        PositionQuote {
            shares,
            price: self.last_price,
            previous_close: self.previous_close,
            provider_type: self.provider_type.as_deref(),
            quote_time: self.quote_time_ms,
            fetched_at: self.fetched_at.map(|time| time.timestamp_millis()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AttributeValue {
    Float(f64),
    Int(i64),
    Text(String),
}

#[derive(Clone)]
pub struct Client {
    http_client: reqwest::Client,
    cache: Option<Arc<Mutex<Cache>>>,
}

impl Client {
    pub fn new() -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(USER_AGENT)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            http_client,
            cache: Cache::new().ok().map(|cache| Arc::new(Mutex::new(cache))),
        }
    }

    pub async fn get_quote(&self, symbol: &str) -> Result<QuoteData> {
        let symbol = symbol.trim().to_uppercase();
        validate_symbol(&symbol)?;

        if let Some(entry) = self.cache_entry(&symbol) {
            return Ok(quote_from_cache(entry));
        }

        self.resolve_fetched_quote(
            &symbol,
            self.fetch_quote(&symbol).await,
            Local::now().timestamp_millis(),
        )
    }

    fn resolve_fetched_quote(
        &self,
        symbol: &str,
        fetched: Result<QuoteData>,
        now_ms: i64,
    ) -> Result<QuoteData> {
        match fetched {
            Ok(quote) if quote.last_price.is_none() => {
                Ok(self.last_valid_cached_quote(symbol).unwrap_or(quote))
            }
            Ok(quote) => {
                if let Some(cached) = self.last_valid_cached_quote(symbol)
                    && keep_newer_cache(&quote, &cached, now_ms)
                {
                    return Ok(cached);
                }
                self.store_quote(symbol, &quote);
                Ok(quote)
            }
            Err(error) => self.last_valid_cached_quote(symbol).ok_or(error),
        }
    }

    fn last_valid_cached_quote(&self, symbol: &str) -> Option<QuoteData> {
        let entry = self.cache.as_ref()?.lock().ok()?.get_stored_entry(symbol)?;
        let quote = quote_from_cache(entry);
        quote.last_price.map(|_| quote)
    }

    async fn fetch_quote(&self, symbol: &str) -> Result<QuoteData> {
        let response = self.fetch_chart(symbol, false).await;
        let now = Local::now();
        let mut quote = quote_from_meta(
            symbol,
            response
                .as_ref()
                .map(|r| &r.meta)
                .unwrap_or(&YahooMeta::default()),
            now,
        );
        if needs_daily_close(&quote, now.timestamp_millis())
            && let Ok(chart) = self.fetch_chart(symbol, true).await
        {
            apply_daily_close(symbol, &chart, &mut quote, Local::now());
        }
        if quote.last_price.is_none()
            && let Err(error) = response
        {
            return Err(error);
        }
        Ok(quote)
    }

    async fn fetch_chart(&self, symbol: &str, daily: bool) -> Result<YahooResult> {
        let url = format!("{CHART_BASE_URL}/{}", escape_path_segment(symbol));
        let request = self.http_client.get(url);
        let request = if daily {
            request.query(&[("range", "1mo"), ("interval", "1d")])
        } else {
            request
        };
        let response = request.send().await.context("fetching data")?;

        if !response.status().is_success() {
            return Err(anyhow!("unexpected status: {}", response.status().as_u16()));
        }

        let chart_response: YahooChartResponse =
            response.json().await.context("parsing response")?;

        if let Some(error) = chart_response.chart.error {
            return Err(anyhow!("API error: {}", error.description));
        }

        let result = chart_response
            .chart
            .result
            .and_then(|mut result| result.drain(..).next())
            .ok_or_else(|| anyhow!("no data found for symbol: {symbol}"))?;

        if result
            .meta
            .symbol
            .as_deref()
            .is_some_and(|returned| returned != symbol)
        {
            return Err(anyhow!("response symbol mismatch for {symbol}"));
        }
        Ok(result)
    }

    pub async fn get_quotes(&self, symbols: &[String]) -> Result<HashMap<String, QuoteData>> {
        if symbols.is_empty() {
            return Err(anyhow!("no symbols provided"));
        }

        let outcomes = stream::iter(symbols.iter().cloned().map(|symbol| {
            let client = self.clone();
            async move {
                let quote = client.get_quote(&symbol).await;
                (symbol, quote)
            }
        }))
        .buffer_unordered(MAX_CONCURRENT)
        .collect::<Vec<_>>()
        .await;

        let mut result = HashMap::new();
        let mut errors = Vec::new();

        for (symbol, quote) in outcomes {
            match quote {
                Ok(quote) => {
                    result.insert(symbol, quote);
                }
                Err(err) => errors.push(format!("{symbol}: {err}")),
            }
        }

        if !errors.is_empty() && result.is_empty() {
            return Err(anyhow!(errors.remove(0)));
        }

        Ok(result)
    }

    fn cache_entry(&self, symbol: &str) -> Option<CacheEntry> {
        let cache = self.cache.as_ref()?;
        cache.lock().ok()?.get_entry(symbol)
    }

    fn store_quote(&self, symbol: &str, quote: &QuoteData) {
        let Some(cache) = &self.cache else {
            return;
        };
        let Ok(mut cache) = cache.lock() else {
            return;
        };
        cache.set(symbol, QuoteCache::from(quote));
    }
}

fn quote_from_meta(symbol: &str, meta: &YahooMeta, now: DateTime<Local>) -> QuoteData {
    QuoteData {
        symbol: meta.symbol.clone().unwrap_or_else(|| symbol.to_owned()),
        last_price: meta.regular_market_price.and_then(sanitize_positive),
        previous_close: meta.previous_close.and_then(sanitize_positive),
        currency: meta
            .currency
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_uppercase(),
        open: meta.regular_market_open,
        day_high: meta.regular_market_day_high,
        day_low: meta.regular_market_day_low,
        volume: meta.regular_market_volume,
        market_cap: meta.market_cap,
        fifty_two_week_high: meta.fifty_two_week_high,
        fifty_two_week_low: meta.fifty_two_week_low,
        quote_time_ms: normalize_quote_time(meta.regular_market_time),
        provider_type: meta
            .instrument_type
            .as_ref()
            .or(meta.quote_type.as_ref())
            .map(|kind| kind.trim().to_uppercase())
            .filter(|kind| !kind.is_empty()),
        fetched_at: Some(now),
        from_cache: false,
        daily_close: false,
    }
}

fn needs_daily_close(quote: &QuoteData, now_ms: i64) -> bool {
    if quote.provider_type.is_some()
        && comparison_period(quote.provider_type.as_deref()) != ComparisonPeriod::Session
    {
        return false;
    }
    quote.last_price.is_none() || !quote_time_is_recent(quote.quote_time_ms, now_ms)
}

fn usable_source_time(time: Option<i64>, now_ms: i64) -> Option<i64> {
    time.filter(|ms| {
        *ms > 0
            && *ms <= now_ms.saturating_add(300_000)
            && DateTime::<Utc>::from_timestamp_millis(*ms).is_some()
    })
}

fn keep_newer_cache(incoming: &QuoteData, cached: &QuoteData, now_ms: i64) -> bool {
    cached.last_price.is_some()
        && (incoming.currency.is_empty() || incoming.currency == cached.currency)
        && (incoming.provider_type.is_none() || incoming.provider_type == cached.provider_type)
        && usable_source_time(cached.quote_time_ms, now_ms).is_some_and(|cached_time| {
            usable_source_time(incoming.quote_time_ms, now_ms).is_none_or(|time| cached_time > time)
        })
}

/// Only completed exchange-local dates; raw close and adjacent baseline come
/// from the same series. Meta prices and adjusted closes are never substituted.
fn apply_daily_close(
    symbol: &str,
    chart: &YahooResult,
    quote: &mut QuoteData,
    now: DateTime<Local>,
) -> Option<()> {
    let meta = &chart.meta;
    let kind = meta.instrument_type.as_deref()?.trim().to_uppercase();
    let currency = meta.currency.as_deref()?.trim().to_uppercase();
    if meta.symbol.as_deref() != Some(symbol)
        || meta.data_granularity.as_deref() != Some("1d")
        || comparison_period(Some(&kind)) != ComparisonPeriod::Session
        || currency.is_empty()
        || quote
            .provider_type
            .as_ref()
            .is_some_and(|primary| primary != &kind)
        || (!quote.currency.is_empty() && quote.currency != currency)
    {
        return None;
    }
    let timezone: chrono_tz::Tz = meta.exchange_timezone_name.as_deref()?.parse().ok()?;
    let today = now.with_timezone(&timezone).date_naive();
    let times = chart.timestamp.as_ref()?;
    let closes = chart
        .indicators
        .as_ref()?
        .quote
        .as_ref()?
        .first()?
        .close
        .as_ref()?;
    if times.is_empty() || times.len() != closes.len() {
        return None;
    }
    let milliseconds: Vec<i64> = times
        .iter()
        .map(|time| normalize_quote_time(Some(*time)))
        .collect::<Option<_>>()?;
    if milliseconds.windows(2).any(|pair| pair[1] <= pair[0]) {
        return None;
    }
    let date = |ms| {
        DateTime::<Utc>::from_timestamp_millis(ms)
            .map(|dt| dt.with_timezone(&timezone).date_naive())
    };
    for i in (0..times.len()).rev() {
        let quote_time = milliseconds[i];
        let bar_date = date(quote_time)?;
        if quote_time > now.timestamp_millis() || bar_date >= today {
            continue;
        }
        let Some(price) = closes[i].and_then(sanitize_positive) else {
            continue;
        };
        if quote.last_price.is_some()
            && usable_source_time(quote.quote_time_ms, now.timestamp_millis())
                .is_some_and(|time| quote_time <= time)
        {
            return None;
        }
        let previous = if i > 0 && date(milliseconds[i - 1])? < bar_date {
            closes[i - 1].and_then(sanitize_positive)
        } else {
            None
        };
        quote.last_price = Some(price);
        quote.previous_close = previous;
        quote.quote_time_ms = Some(quote_time);
        quote.provider_type = Some(kind);
        quote.currency = currency;
        quote.fetched_at = Some(now);
        quote.daily_close = true;
        quote.from_cache = false;
        return Some(());
    }
    None
}

pub struct Ticker {
    data: QuoteData,
}

impl Ticker {
    pub async fn new(symbol: &str) -> Result<Self> {
        let data = Client::new().get_quote(symbol).await?;
        Ok(Self { data })
    }

    pub fn data(&self) -> &QuoteData {
        &self.data
    }

    pub fn attribute(&self, attr: &str) -> Option<AttributeValue> {
        let data = &self.data;
        // Intraday fields belong to the frozen metadata, not the daily close.
        if data.daily_close && matches!(attr, "open" | "dayHigh" | "dayLow" | "volume") {
            return Some(missing_attribute());
        }
        match attr {
            "symbol" => Some(AttributeValue::Text(data.symbol.clone())),
            "lastPrice" => Some(optional_float(data.last_price)),
            "previousClose" => Some(optional_float(data.previous_close)),
            "currency" => Some(AttributeValue::Text(data.currency.clone())),
            "open" => Some(AttributeValue::Float(data.open)),
            "dayHigh" => Some(AttributeValue::Float(data.day_high)),
            "dayLow" => Some(AttributeValue::Float(data.day_low)),
            "volume" => Some(AttributeValue::Int(data.volume)),
            "marketCap" => Some(AttributeValue::Int(data.market_cap)),
            "fiftyTwoWeekHigh" => Some(AttributeValue::Float(data.fifty_two_week_high)),
            "fiftyTwoWeekLow" => Some(AttributeValue::Float(data.fifty_two_week_low)),
            "priceSource" => Some(AttributeValue::Text(
                if data.daily_close {
                    "daily close (not live)"
                } else {
                    "quote"
                }
                .to_string(),
            )),
            "quoteType" => Some(
                data.provider_type
                    .clone()
                    .map(AttributeValue::Text)
                    .unwrap_or_else(missing_attribute),
            ),
            "quoteTime" => Some(
                data.quote_time_ms
                    .map(|ms| AttributeValue::Text(format_quote_time(ms)))
                    .unwrap_or_else(missing_attribute),
            ),
            _ => None,
        }
    }
}

fn optional_float(value: Option<f64>) -> AttributeValue {
    value
        .map(AttributeValue::Float)
        .unwrap_or_else(missing_attribute)
}

fn missing_attribute() -> AttributeValue {
    AttributeValue::Text("N/A".to_string())
}

fn normalize_quote_time(seconds: Option<i64>) -> Option<i64> {
    seconds
        .filter(|seconds| *seconds > 0)
        .and_then(|seconds| seconds.checked_mul(1000))
        .filter(|ms| DateTime::<Utc>::from_timestamp_millis(*ms).is_some())
}

pub fn format_quote_time(timestamp_ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(timestamp_ms)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "N/A".to_string())
}

pub fn validate_symbol(symbol: &str) -> Result<()> {
    let symbol = symbol.trim();
    if symbol.is_empty() {
        return Err(anyhow!("empty ticker symbol"));
    }

    let len = symbol.chars().count();
    if len > 20 || !symbol.chars().all(is_valid_symbol_char) {
        return Err(anyhow!("invalid ticker symbol: {symbol}"));
    }

    Ok(())
}

fn is_valid_symbol_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '^' | '.' | '_' | '-')
}

fn quote_from_cache(entry: CacheEntry) -> QuoteData {
    QuoteData {
        symbol: entry.data.symbol,
        last_price: entry.data.last_price.and_then(sanitize_positive),
        previous_close: entry.data.previous_close.and_then(sanitize_positive),
        currency: entry.data.currency,
        open: entry.data.open,
        day_high: entry.data.day_high,
        day_low: entry.data.day_low,
        volume: entry.data.volume,
        market_cap: entry.data.market_cap,
        fifty_two_week_high: entry.data.fifty_two_week_high,
        fifty_two_week_low: entry.data.fifty_two_week_low,
        quote_time_ms: entry.data.quote_time_ms,
        provider_type: entry.data.provider_type,
        fetched_at: (entry.timestamp > 0)
            .then(|| Local.timestamp_opt(entry.timestamp, 0).single())
            .flatten(),
        from_cache: true,
        daily_close: entry.data.daily_close,
    }
}

impl From<&QuoteData> for QuoteCache {
    fn from(quote: &QuoteData) -> Self {
        Self {
            symbol: quote.symbol.clone(),
            last_price: quote.last_price,
            previous_close: quote.previous_close,
            currency: quote.currency.clone(),
            open: quote.open,
            day_high: quote.day_high,
            day_low: quote.day_low,
            volume: quote.volume,
            market_cap: quote.market_cap,
            fifty_two_week_high: quote.fifty_two_week_high,
            fifty_two_week_low: quote.fifty_two_week_low,
            quote_time_ms: quote.quote_time_ms,
            provider_type: quote.provider_type.clone(),
            daily_close: quote.daily_close,
        }
    }
}

fn escape_path_segment(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                escaped.push(byte as char);
            }
            _ => escaped.push_str(&format!("%{byte:02X}")),
        }
    }
    escaped
}

#[derive(Debug, Deserialize)]
struct YahooChartResponse {
    chart: YahooChart,
}

#[derive(Debug, Deserialize)]
struct YahooChart {
    result: Option<Vec<YahooResult>>,
    error: Option<YahooError>,
}

#[derive(Debug, Deserialize)]
struct YahooResult {
    meta: YahooMeta,
    timestamp: Option<Vec<i64>>,
    indicators: Option<YahooIndicators>,
}

#[derive(Debug, Deserialize)]
struct YahooIndicators {
    quote: Option<Vec<YahooDailyQuotes>>,
}

#[derive(Debug, Deserialize)]
struct YahooDailyQuotes {
    close: Option<Vec<Option<f64>>>,
}

#[derive(Debug, Deserialize)]
struct YahooError {
    description: String,
}

#[derive(Debug, Default, Deserialize)]
struct YahooMeta {
    currency: Option<String>,
    symbol: Option<String>,
    #[serde(rename = "dataGranularity")]
    data_granularity: Option<String>,
    #[serde(rename = "exchangeTimezoneName")]
    exchange_timezone_name: Option<String>,
    #[serde(rename = "regularMarketPrice")]
    regular_market_price: Option<f64>,
    #[serde(rename = "previousClose")]
    previous_close: Option<f64>,
    #[serde(rename = "regularMarketTime")]
    regular_market_time: Option<i64>,
    #[serde(rename = "instrumentType")]
    instrument_type: Option<String>,
    #[serde(rename = "quoteType")]
    quote_type: Option<String>,
    #[serde(default, rename = "regularMarketVolume")]
    regular_market_volume: i64,
    #[serde(default, rename = "regularMarketDayHigh")]
    regular_market_day_high: f64,
    #[serde(default, rename = "regularMarketDayLow")]
    regular_market_day_low: f64,
    #[serde(default, rename = "regularMarketOpen")]
    regular_market_open: f64,
    #[serde(default, rename = "marketCap")]
    market_cap: i64,
    #[serde(default, rename = "fiftyTwoWeekHigh")]
    fifty_two_week_high: f64,
    #[serde(default, rename = "fiftyTwoWeekLow")]
    fifty_two_week_low: f64,
}

#[cfg(test)]
mod daily_close_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_quote_data() -> QuoteData {
        QuoteData {
            symbol: "VWCE.MI".to_string(),
            last_price: Some(100.0),
            previous_close: Some(95.0),
            currency: "EUR".to_string(),
            open: 99.0,
            day_high: 101.0,
            day_low: 98.0,
            volume: 1,
            market_cap: 2,
            fifty_two_week_high: 120.0,
            fifty_two_week_low: 80.0,
            quote_time_ms: Some(1_789_380_000_000),
            provider_type: Some("ETF".to_string()),
            fetched_at: Local.timestamp_opt(1_789_387_200, 0).single(),
            from_cache: false,
            daily_close: false,
        }
    }

    #[test]
    fn validate_symbol_accepts_yahoo_symbols() {
        for symbol in ["AAPL", "VWCE.MI", "^GSPC", "BRK-B", "ABC_1"] {
            validate_symbol(symbol).expect(symbol);
        }
    }

    #[test]
    fn validate_symbol_rejects_invalid_values() {
        for symbol in [
            "",
            "AAPL/../../etc/passwd",
            "symbol with spaces",
            "123456789012345678901",
        ] {
            assert!(validate_symbol(symbol).is_err(), "{symbol}");
        }
    }

    #[test]
    fn escape_symbol_for_path_segment() {
        assert_eq!(escape_path_segment("^GSPC"), "%5EGSPC");
        assert_eq!(escape_path_segment("VWCE.MI"), "VWCE.MI");
    }

    #[test]
    fn quote_time_normalization_converts_seconds_to_ms() {
        assert_eq!(
            normalize_quote_time(Some(1_789_380_000)),
            Some(1_789_380_000_000)
        );
        assert_eq!(normalize_quote_time(Some(0)), None);
        assert_eq!(normalize_quote_time(Some(-1)), None);
        assert_eq!(normalize_quote_time(None), None);
    }

    #[test]
    fn cache_round_trip_preserves_quote_metadata() {
        let quote = sample_quote_data();
        let entry = CacheEntry {
            data: QuoteCache::from(&quote),
            timestamp: quote.fetched_at.unwrap().timestamp(),
        };

        let restored = quote_from_cache(entry);
        assert_eq!(restored.last_price, Some(100.0));
        assert_eq!(restored.previous_close, Some(95.0));
        assert_eq!(restored.quote_time_ms, Some(1_789_380_000_000));
        assert_eq!(restored.provider_type.as_deref(), Some("ETF"));
        assert!(restored.from_cache);
    }

    #[test]
    fn legacy_cache_entry_acquires_no_invented_metadata() {
        let mut cached = QuoteCache::from(&sample_quote_data());
        cached.quote_time_ms = None;
        cached.provider_type = None;
        let entry = CacheEntry {
            data: cached,
            timestamp: 1_789_387_200,
        };

        let restored = quote_from_cache(entry);
        assert_eq!(restored.quote_time_ms, None);
        assert_eq!(restored.provider_type, None);
        assert_eq!(restored.last_price, Some(100.0));
    }

    #[test]
    fn invalid_cached_prices_are_sanitized_to_missing() {
        let mut cached = QuoteCache::from(&sample_quote_data());
        cached.last_price = Some(0.0);
        cached.previous_close = Some(-95.0);
        let restored = quote_from_cache(CacheEntry {
            data: cached,
            timestamp: 1_789_387_200,
        });

        assert_eq!(restored.last_price, None);
        assert_eq!(restored.previous_close, None);
    }

    #[test]
    fn corrupt_fetch_time_is_not_replaced_with_now() {
        for timestamp in [0, -1, i64::MAX] {
            let quote = quote_from_cache(CacheEntry {
                data: QuoteCache::from(&sample_quote_data()),
                timestamp,
            });
            assert!(quote.fetched_at.is_none());
            assert_eq!(quote.last_price, Some(100.0));
            assert!(
                !crate::quote_policy::evaluate_position(
                    quote.position_quote(10.0),
                    1_789_387_200_000,
                )
                .session_eligible
            );
        }
    }

    #[test]
    fn expired_valuation_fallback_keeps_original_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let quote = sample_quote_data();
        let path = directory.path().join("cache.json");
        let entry = CacheEntry {
            data: QuoteCache::from(&quote),
            timestamp: 1_789_387_200,
        };
        std::fs::write(
            &path,
            serde_json::to_string(&HashMap::from([("FUND", entry)])).unwrap(),
        )
        .unwrap();
        let mut cache = Cache::with_path(path);
        cache.load();
        let client = Client {
            http_client: reqwest::Client::new(),
            cache: Some(Arc::new(Mutex::new(cache))),
        };
        let fallback = client.last_valid_cached_quote("FUND").unwrap();
        assert_eq!(fallback.fetched_at, quote.fetched_at);
        assert_eq!(fallback.quote_time_ms, quote.quote_time_ms);
        assert!(fallback.from_cache);
    }

    #[test]
    fn chart_meta_parses_type_and_quote_time() {
        let raw = r#"{
            "chart": {
                "result": [{
                    "meta": {
                        "currency": "EUR",
                        "symbol": "FUND",
                        "instrumentType": "MUTUALFUND",
                        "regularMarketPrice": 21.0,
                        "regularMarketTime": 1789380000
                    }
                }],
                "error": null
            }
        }"#;

        let parsed: YahooChartResponse = serde_json::from_str(raw).expect("parse chart");
        let result = parsed
            .chart
            .result
            .expect("result")
            .into_iter()
            .next()
            .expect("first result");

        assert_eq!(result.meta.instrument_type.as_deref(), Some("MUTUALFUND"));
        assert_eq!(result.meta.regular_market_time, Some(1_789_380_000));
        assert_eq!(result.meta.previous_close, None);
        assert_eq!(result.meta.regular_market_price, Some(21.0));
    }

    #[test]
    fn missing_ticker_attributes_render_na() {
        let mut data = sample_quote_data();
        data.last_price = None;
        data.previous_close = None;
        data.provider_type = None;
        data.quote_time_ms = None;
        let ticker = Ticker { data };

        for attribute in ["lastPrice", "previousClose", "quoteType", "quoteTime"] {
            match ticker.attribute(attribute) {
                Some(AttributeValue::Text(text)) => assert_eq!(text, "N/A", "{attribute}"),
                other => panic!("{attribute}: unexpected {other:?}"),
            }
        }
        assert!(ticker.attribute("notAnAttribute").is_none());
    }
}
