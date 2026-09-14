use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Local, TimeZone};
use futures::{StreamExt, stream};
use serde::Deserialize;

use crate::cache::{Cache, CacheEntry, QuoteCache};
use crate::quote_policy::{PositionQuote, sanitize_positive};

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

        match self.fetch_quote(&symbol).await {
            Ok(quote) if quote.last_price.is_none() => {
                Ok(self.last_valid_cached_quote(&symbol).unwrap_or(quote))
            }
            Ok(quote) => {
                self.store_quote(&symbol, &quote);
                Ok(quote)
            }
            Err(error) => self.last_valid_cached_quote(&symbol).ok_or(error),
        }
    }

    fn last_valid_cached_quote(&self, symbol: &str) -> Option<QuoteData> {
        let entry = self.cache.as_ref()?.lock().ok()?.get_stored_entry(symbol)?;
        let quote = quote_from_cache(entry);
        quote.last_price.map(|_| quote)
    }

    async fn fetch_quote(&self, symbol: &str) -> Result<QuoteData> {
        let url = format!("{CHART_BASE_URL}/{}", escape_path_segment(symbol));
        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .context("fetching data")?;

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

        let quote = QuoteData {
            symbol: result.meta.symbol.unwrap_or_else(|| symbol.to_owned()),
            last_price: result.meta.regular_market_price.and_then(sanitize_positive),
            previous_close: result.meta.previous_close.and_then(sanitize_positive),
            currency: result.meta.currency.unwrap_or_default(),
            open: result.meta.regular_market_open,
            day_high: result.meta.regular_market_day_high,
            day_low: result.meta.regular_market_day_low,
            volume: result.meta.regular_market_volume,
            market_cap: result.meta.market_cap,
            fifty_two_week_high: result.meta.fifty_two_week_high,
            fifty_two_week_low: result.meta.fifty_two_week_low,
            quote_time_ms: normalize_quote_time(result.meta.regular_market_time),
            provider_type: result.meta.instrument_type.or(result.meta.quote_type),
            fetched_at: Some(Local::now()),
            from_cache: false,
        };

        Ok(quote)
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
        .map(|seconds| seconds.saturating_mul(1000))
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
}

#[derive(Debug, Deserialize)]
struct YahooError {
    description: String,
}

#[derive(Debug, Default, Deserialize)]
struct YahooMeta {
    currency: Option<String>,
    symbol: Option<String>,
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
