use std::{collections::HashMap, fs, io, path::Path};

use anyhow::{Result, anyhow};
use chrono::{DateTime, Local};
use serde::Deserialize;

use crate::finance::Client;
use crate::quote_policy::{self, PortfolioSummary, PositionEvaluation};

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PortfolioItem {
    pub ticker: String,
    pub shares: f64,
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
        if !item.shares.is_finite() || item.shares <= 0.0 {
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
    pub shares: f64,
    pub currency: String,
    pub from_cache: bool,
    /// Policy verdict for this position: valuation, session eligibility, and
    /// the raw comparison with its period.
    pub evaluation: PositionEvaluation,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CurrencyGroup {
    pub currency: String,
    pub items: Vec<EnrichedItem>,
    /// Policy aggregate for this group only. Unlike currencies are never
    /// summed, so each group carries its own totals.
    pub summary: PortfolioSummary,
}

#[derive(Debug, Default)]
pub struct Portfolio {
    items: Vec<EnrichedItem>,
    summary: PortfolioSummary,
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
        let now_ms = Local::now().timestamp_millis();

        self.items = Vec::with_capacity(items.len());
        self.skipped.clear();
        self.fetch_info = FetchInfo {
            all_from_cache: true,
            ..FetchInfo::default()
        };

        for item in items {
            let Some(quote) = quotes.get(&item.ticker) else {
                self.skipped.push(item.ticker);
                continue;
            };

            if let Some(fetched_at) = quote.fetched_at {
                self.track_fetch_info(fetched_at, quote.from_cache);
            }

            let mut evaluation =
                quote_policy::evaluate_position(quote.position_quote(item.shares), now_ms);
            // Unknown monetary units cannot be summed into a currency subtotal.
            if quote.currency.trim().is_empty() {
                evaluation.value = None;
                evaluation.session_date = None;
                evaluation.session_eligible = false;
                evaluation.session_amount = None;
                evaluation.session_percent = None;
                evaluation.comparison_amount = None;
                evaluation.quality = quote_policy::QuoteQuality::Incomplete;
            }

            self.items.push(EnrichedItem {
                ticker: item.ticker,
                shares: item.shares,
                currency: quote.currency.clone(),
                from_cache: quote.from_cache,
                evaluation,
            });
        }

        let evaluations: Vec<PositionEvaluation> = self
            .items
            .iter()
            .map(|item| item.evaluation.clone())
            .collect();
        self.summary = quote_policy::aggregate(&evaluations, self.skipped.len());

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

    pub fn summary(&self) -> &PortfolioSummary {
        &self.summary
    }

    /// Tickers whose quote had no usable price. They keep their rows (marked
    /// unknown) instead of disappearing silently.
    pub fn unpriced(&self) -> Vec<&str> {
        self.items
            .iter()
            .filter(|item| item.evaluation.value.is_none())
            .map(|item| item.ticker.as_str())
            .collect()
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
                    summary: PortfolioSummary::default(),
                });
                index
            };

            groups[index].items.push(item.clone());
        }

        for group in &mut groups {
            let evaluations: Vec<PositionEvaluation> = group
                .items
                .iter()
                .map(|item| item.evaluation.clone())
                .collect();
            // The currency of failed quotes is unknown: no group may claim
            // complete coverage until those requested positions are priced.
            group.summary = quote_policy::aggregate(&evaluations, self.skipped.len());
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
    use crate::quote_policy::SessionStatus;
    use tempfile::tempdir;

    fn create_temp_file(content: &str) -> std::path::PathBuf {
        let dir = tempdir().expect("tempdir");
        let path = dir.keep().join("portfolio.json");
        fs::write(&path, content).expect("write portfolio");
        path
    }

    #[allow(clippy::too_many_arguments)] // mirrors the policy input tuple used by fixtures
    fn enriched(
        ticker: &str,
        shares: f64,
        currency: &str,
        price: Option<f64>,
        previous_close: Option<f64>,
        provider_type: Option<&str>,
        quote_time: Option<i64>,
        fetched_at: Option<i64>,
        now_ms: i64,
    ) -> EnrichedItem {
        EnrichedItem {
            ticker: ticker.to_string(),
            shares,
            currency: currency.to_string(),
            from_cache: false,
            evaluation: quote_policy::evaluate_position(
                quote_policy::PositionQuote {
                    shares,
                    price,
                    previous_close,
                    provider_type,
                    quote_time,
                    fetched_at,
                },
                now_ms,
            ),
        }
    }

    fn portfolio_with(items: Vec<EnrichedItem>, skipped: Vec<String>) -> Portfolio {
        let evaluations: Vec<PositionEvaluation> =
            items.iter().map(|item| item.evaluation.clone()).collect();
        let summary = quote_policy::aggregate(&evaluations, skipped.len());
        Portfolio {
            items,
            summary,
            skipped,
            fetch_info: FetchInfo::default(),
        }
    }

    #[test]
    fn load_valid_portfolio() {
        let path = create_temp_file(
            r#"[{"ticker": "AAPL", "shares": 10}, {"ticker": "GOOG", "shares": 5}]"#,
        );

        let items = load_portfolio(path).expect("portfolio");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].ticker, "AAPL");
        assert_eq!(items[0].shares, 10.0);
    }

    #[test]
    fn load_fractional_shares() {
        let path = create_temp_file(r#"[{"ticker": "FUND", "shares": 756.344}]"#);

        let items = load_portfolio(path).expect("portfolio");
        assert_eq!(items[0].shares, 756.344);
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

    #[test]
    fn currency_groups_keep_currencies_and_sessions_separate() {
        let now = 1_789_387_200_000i64;
        let portfolio = portfolio_with(
            vec![
                enriched(
                    "REGULAR",
                    10.0,
                    "EUR",
                    Some(100.0),
                    Some(95.0),
                    Some("ETF"),
                    Some(now - 7_200_000),
                    Some(now),
                    now,
                ),
                enriched(
                    "PERIODIC",
                    100.0,
                    "EUR",
                    Some(21.0),
                    Some(20.0),
                    Some("MUTUALFUND"),
                    Some(now - 7_200_000),
                    Some(now),
                    now,
                ),
                enriched(
                    "STOCK",
                    2.0,
                    "USD",
                    Some(50.0),
                    Some(48.0),
                    Some("EQUITY"),
                    Some(now - 7_200_000),
                    Some(now),
                    now,
                ),
            ],
            Vec::new(),
        );

        let groups = portfolio.currency_groups();
        assert_eq!(groups.len(), 2);

        assert_eq!(groups[0].currency, "EUR");
        assert!((groups[0].summary.total_value - 3100.0).abs() < 1e-9);
        assert_eq!(groups[0].summary.session.status, SessionStatus::Partial);
        assert!((groups[0].summary.session.amount.unwrap() - 50.0).abs() < 1e-9);
        assert_eq!(groups[0].summary.session.included_positions, 1);
        assert_eq!(groups[0].summary.session.total_positions, 2);

        assert_eq!(groups[1].currency, "USD");
        assert!((groups[1].summary.total_value - 100.0).abs() < 1e-9);
        assert_eq!(groups[1].summary.session.status, SessionStatus::Complete);
        assert!((groups[1].summary.session.amount.unwrap() - 4.0).abs() < 1e-9);
    }

    #[test]
    fn group_coverage_is_unknown_with_unpriced_position() {
        let now = 1_789_387_200_000i64;
        let portfolio = portfolio_with(
            vec![
                enriched(
                    "REGULAR",
                    10.0,
                    "EUR",
                    Some(100.0),
                    Some(95.0),
                    Some("ETF"),
                    Some(now - 7_200_000),
                    Some(now),
                    now,
                ),
                enriched(
                    "UNPRICED",
                    100.0,
                    "EUR",
                    None,
                    Some(20.0),
                    Some("ETF"),
                    Some(now - 7_200_000),
                    Some(now),
                    now,
                ),
            ],
            Vec::new(),
        );

        assert_eq!(portfolio.unpriced(), vec!["UNPRICED"]);

        let groups = portfolio.currency_groups();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].summary.coverage_percent, None);
        assert_eq!(groups[0].summary.session.status, SessionStatus::Partial);
    }

    #[test]
    fn missing_quote_blocks_complete_coverage_across_modes() {
        let now = 1_789_387_200_000i64;
        let portfolio = portfolio_with(
            vec![enriched(
                "REGULAR",
                10.0,
                "EUR",
                Some(100.0),
                Some(95.0),
                Some("ETF"),
                Some(now - 7_200_000),
                Some(now),
                now,
            )],
            vec!["GONE".to_string()],
        );

        let summary = portfolio.summary();
        assert_eq!(summary.session.status, SessionStatus::Partial);
        assert_eq!(summary.session.total_positions, 2);
        assert_eq!(summary.coverage_percent, None);
        for group in portfolio.currency_groups() {
            assert_eq!(group.summary.session.status, SessionStatus::Partial);
            assert_eq!(group.summary.session.total_positions, 2);
            assert_eq!(group.summary.coverage_percent, None);
        }
    }
}
