use std::{collections::HashMap, fs, io, path::Path};

use anyhow::{Result, anyhow};
use chrono::{DateTime, Local};
use serde::Deserialize;

use crate::finance::Client;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PortfolioItem {
    pub ticker: String,
    pub shares: i64,
}

pub fn load_portfolio(file_path: impl AsRef<Path>) -> Result<Vec<PortfolioItem>> {
    let file_path = file_path.as_ref();
    let data = fs::read_to_string(file_path).map_err(|err| {
        if err.kind() == io::ErrorKind::NotFound {
            anyhow!("portfolio file not found: {}", file_path.display())
        } else {
            anyhow!("reading portfolio file: {err}")
        }
    })?;

    let items: Vec<PortfolioItem> =
        serde_json::from_str(&data).map_err(|err| anyhow!("invalid JSON format: {err}"))?;

    for (index, item) in items.iter().enumerate() {
        if item.ticker.is_empty() {
            return Err(anyhow!("item {index}: missing 'ticker' field"));
        }
        if item.shares <= 0 {
            return Err(anyhow!("item {index}: 'shares' must be positive"));
        }
    }

    Ok(items)
}

#[derive(Debug, Clone, Default)]
pub struct FetchInfo {
    pub oldest_fetched_at: Option<DateTime<Local>>,
    pub newest_fetched_at: Option<DateTime<Local>>,
    pub all_from_cache: bool,
    pub any_from_cache: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnrichedItem {
    pub ticker: String,
    pub shares: i64,
    pub price: f64,
    pub previous_close: f64,
    pub pnl: f64,
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CurrencyGroup {
    pub currency: String,
    pub items: Vec<EnrichedItem>,
    pub total_value: f64,
    pub total_pnl: f64,
}

#[derive(Debug, Default)]
pub struct Portfolio {
    items: Vec<EnrichedItem>,
    total_value: f64,
    skipped: Vec<String>,
    fetch_info: FetchInfo,
}

impl Portfolio {
    pub async fn new(file_path: impl AsRef<Path>) -> Result<Self> {
        let raw_items = load_portfolio(file_path)?;
        let mut portfolio = Self::default();
        portfolio.enrich(raw_items).await?;
        Ok(portfolio)
    }

    async fn enrich(&mut self, items: Vec<PortfolioItem>) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        let symbols = items
            .iter()
            .map(|item| item.ticker.clone())
            .collect::<Vec<_>>();
        let quotes = Client::new().get_quotes(&symbols).await?;

        self.items = Vec::with_capacity(items.len());
        self.skipped.clear();
        self.total_value = 0.0;
        self.fetch_info = FetchInfo {
            all_from_cache: true,
            ..FetchInfo::default()
        };

        for item in items {
            let Some(quote) = quotes.get(&item.ticker) else {
                self.skipped.push(item.ticker);
                continue;
            };

            self.track_fetch_info(quote.fetched_at, quote.from_cache);

            let price = item.shares as f64 * quote.last_price;
            let previous_close = item.shares as f64 * quote.previous_close;
            let pnl = if previous_close > 0.0 {
                ((price / previous_close) - 1.0) * 100.0
            } else {
                0.0
            };

            self.items.push(EnrichedItem {
                ticker: item.ticker,
                shares: item.shares,
                price,
                previous_close,
                pnl,
                currency: quote.currency.clone(),
            });
            self.total_value += price;
        }

        Ok(())
    }

    fn track_fetch_info(&mut self, fetched_at: DateTime<Local>, from_cache: bool) {
        self.fetch_info.oldest_fetched_at = Some(
            self.fetch_info
                .oldest_fetched_at
                .map_or(fetched_at, |oldest| oldest.min(fetched_at)),
        );
        self.fetch_info.newest_fetched_at = Some(
            self.fetch_info
                .newest_fetched_at
                .map_or(fetched_at, |newest| newest.max(fetched_at)),
        );

        if from_cache {
            self.fetch_info.any_from_cache = true;
        } else {
            self.fetch_info.all_from_cache = false;
        }
    }

    pub fn items(&self) -> &[EnrichedItem] {
        &self.items
    }

    pub fn total_value(&self) -> f64 {
        self.total_value
    }

    pub fn total_pnl(&self) -> f64 {
        let total_previous_close = self
            .items
            .iter()
            .map(|item| item.previous_close)
            .sum::<f64>();

        if total_previous_close == 0.0 {
            0.0
        } else {
            ((self.total_value / total_previous_close) - 1.0) * 100.0
        }
    }

    pub fn currency_groups(&self) -> Vec<CurrencyGroup> {
        let mut groups: Vec<CurrencyGroup> = Vec::new();
        let mut positions: HashMap<String, usize> = HashMap::new();

        for item in &self.items {
            let index = if let Some(index) = positions.get(&item.currency) {
                *index
            } else {
                let index = groups.len();
                positions.insert(item.currency.clone(), index);
                groups.push(CurrencyGroup {
                    currency: item.currency.clone(),
                    items: Vec::new(),
                    total_value: 0.0,
                    total_pnl: 0.0,
                });
                index
            };

            let group = &mut groups[index];
            group.total_value += item.price;
            group.items.push(item.clone());
        }

        for group in &mut groups {
            let total_previous_close = group
                .items
                .iter()
                .map(|item| item.previous_close)
                .sum::<f64>();
            if total_previous_close > 0.0 {
                group.total_pnl = ((group.total_value / total_previous_close) - 1.0) * 100.0;
            }
        }

        groups
    }

    pub fn is_single_currency(&self) -> bool {
        let Some(first) = self.items.first() else {
            return true;
        };
        self.items
            .iter()
            .all(|item| item.currency == first.currency)
    }

    pub fn currency(&self) -> Option<&str> {
        if self.is_single_currency() {
            self.items.first().map(|item| item.currency.as_str())
        } else {
            None
        }
    }

    pub fn skipped(&self) -> &[String] {
        &self.skipped
    }

    pub fn fetch_info(&self) -> &FetchInfo {
        &self.fetch_info
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_temp_file(content: &str) -> std::path::PathBuf {
        let dir = tempdir().expect("tempdir");
        let path = dir.keep().join("portfolio.json");
        fs::write(&path, content).expect("write portfolio");
        path
    }

    #[test]
    fn load_valid_portfolio() {
        let path = create_temp_file(
            r#"[{"ticker": "AAPL", "shares": 10}, {"ticker": "GOOG", "shares": 5}]"#,
        );

        let items = load_portfolio(path).expect("portfolio");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].ticker, "AAPL");
        assert_eq!(items[0].shares, 10);
    }

    #[test]
    fn load_missing_file_fails() {
        assert!(load_portfolio("/nonexistent/path.json").is_err());
    }

    #[test]
    fn load_invalid_json_fails() {
        let path = create_temp_file("not valid json");
        assert!(load_portfolio(path).is_err());
    }

    #[test]
    fn load_missing_ticker_fails() {
        let path = create_temp_file(r#"[{"shares": 10}]"#);
        assert!(load_portfolio(path).is_err());
    }

    #[test]
    fn load_zero_shares_fails() {
        let path = create_temp_file(r#"[{"ticker": "AAPL", "shares": 0}]"#);
        assert!(load_portfolio(path).is_err());
    }

    #[test]
    fn load_negative_shares_fails() {
        let path = create_temp_file(r#"[{"ticker": "AAPL", "shares": -5}]"#);
        assert!(load_portfolio(path).is_err());
    }

    #[test]
    fn load_empty_portfolio() {
        let path = create_temp_file("[]");
        let items = load_portfolio(path).expect("portfolio");
        assert!(items.is_empty());
    }
}
