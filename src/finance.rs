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

const CHART_BASE_URL: &str = "https://query1.finance.yahoo.com/v8/finance/chart";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36";
const MAX_CONCURRENT: usize = 10;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
pub struct QuoteData {
    pub symbol: String,
    pub last_price: f64,
    pub previous_close: f64,
    pub currency: String,
    pub open: f64,
    pub day_high: f64,
    pub day_low: f64,
    pub volume: i64,
    pub market_cap: i64,
    pub fifty_two_week_high: f64,
    pub fifty_two_week_low: f64,
    pub fetched_at: DateTime<Local>,
    pub from_cache: bool,
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

        let url = format!("{CHART_BASE_URL}/{}", escape_path_segment(&symbol));
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
            symbol: result.meta.symbol.unwrap_or_else(|| symbol.clone()),
            last_price: result.meta.regular_market_price,
            previous_close: result.meta.previous_close,
            currency: result.meta.currency.unwrap_or_default(),
            open: result.meta.regular_market_open,
            day_high: result.meta.regular_market_day_high,
            day_low: result.meta.regular_market_day_low,
            volume: result.meta.regular_market_volume,
            market_cap: result.meta.market_cap,
            fifty_two_week_high: result.meta.fifty_two_week_high,
            fifty_two_week_low: result.meta.fifty_two_week_low,
            fetched_at: Local::now(),
            from_cache: false,
        };

        self.store_quote(&symbol, &quote);
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
            "lastPrice" => Some(AttributeValue::Float(data.last_price)),
            "previousClose" => Some(AttributeValue::Float(data.previous_close)),
            "currency" => Some(AttributeValue::Text(data.currency.clone())),
            "open" => Some(AttributeValue::Float(data.open)),
            "dayHigh" => Some(AttributeValue::Float(data.day_high)),
            "dayLow" => Some(AttributeValue::Float(data.day_low)),
            "volume" => Some(AttributeValue::Int(data.volume)),
            "marketCap" => Some(AttributeValue::Int(data.market_cap)),
            "fiftyTwoWeekHigh" => Some(AttributeValue::Float(data.fifty_two_week_high)),
            "fiftyTwoWeekLow" => Some(AttributeValue::Float(data.fifty_two_week_low)),
            _ => None,
        }
    }
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
        last_price: entry.data.last_price,
        previous_close: entry.data.previous_close,
        currency: entry.data.currency,
        open: entry.data.open,
        day_high: entry.data.day_high,
        day_low: entry.data.day_low,
        volume: entry.data.volume,
        market_cap: entry.data.market_cap,
        fifty_two_week_high: entry.data.fifty_two_week_high,
        fifty_two_week_low: entry.data.fifty_two_week_low,
        fetched_at: Local
            .timestamp_opt(entry.timestamp, 0)
            .single()
            .unwrap_or_else(Local::now),
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
    #[serde(default, rename = "regularMarketPrice")]
    regular_market_price: f64,
    #[serde(default, rename = "previousClose")]
    previous_close: f64,
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
}
