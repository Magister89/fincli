use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const CACHE_TTL: Duration = Duration::from_secs(120);
const CACHE_DIR: &str = ".fincli";
const CACHE_FILE: &str = "cache.json";

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct QuoteCache {
    pub symbol: String,
    #[serde(rename = "lastPrice")]
    pub last_price: f64,
    #[serde(rename = "previousClose")]
    pub previous_close: f64,
    pub currency: String,
    pub open: f64,
    #[serde(rename = "dayHigh")]
    pub day_high: f64,
    #[serde(rename = "dayLow")]
    pub day_low: f64,
    pub volume: i64,
    #[serde(rename = "marketCap")]
    pub market_cap: i64,
    #[serde(rename = "fiftyTwoWeekHigh")]
    pub fifty_two_week_high: f64,
    #[serde(rename = "fiftyTwoWeekLow")]
    pub fifty_two_week_low: f64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct CacheEntry {
    pub data: QuoteCache,
    pub timestamp: i64,
}

#[derive(Debug)]
pub struct Cache {
    entries: HashMap<String, CacheEntry>,
    path: PathBuf,
}

impl Cache {
    pub fn new() -> Result<Self> {
        let home_dir = home_dir().context("could not determine home directory")?;
        let cache_dir = home_dir.join(CACHE_DIR);
        fs::create_dir_all(&cache_dir).context("creating cache directory")?;

        let mut cache = Self::with_path(cache_dir.join(CACHE_FILE));
        cache.load();
        Ok(cache)
    }

    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            entries: HashMap::new(),
            path: path.into(),
        }
    }

    #[cfg(test)]
    pub fn get(&self, symbol: &str) -> Option<QuoteCache> {
        self.get_entry(symbol).map(|entry| entry.data)
    }

    pub fn get_entry(&self, symbol: &str) -> Option<CacheEntry> {
        let entry = self.entries.get(symbol)?;
        if entry_is_fresh(entry) {
            Some(entry.clone())
        } else {
            None
        }
    }

    pub fn set(&mut self, symbol: &str, data: QuoteCache) {
        self.entries.insert(
            symbol.to_owned(),
            CacheEntry {
                data,
                timestamp: now_unix(),
            },
        );
        self.save();
    }

    #[cfg(test)]
    pub fn set_multiple(&mut self, quotes: HashMap<String, QuoteCache>) {
        let timestamp = now_unix();
        for (symbol, data) in quotes {
            self.entries.insert(symbol, CacheEntry { data, timestamp });
        }
        self.save();
    }

    pub fn load(&mut self) {
        let Ok(data) = fs::read_to_string(&self.path) else {
            return;
        };

        if let Ok(entries) = serde_json::from_str(&data) {
            self.entries = entries;
        }
    }

    fn save(&self) {
        let Ok(data) = serde_json::to_string_pretty(&self.entries) else {
            return;
        };

        if let Some(parent) = self.path.parent() {
            if fs::create_dir_all(parent).is_err() {
                return;
            }
        }

        let temp_path = temp_path(&self.path);
        if fs::write(&temp_path, data).is_err() {
            return;
        }
        let _ = fs::rename(temp_path, &self.path);
    }
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
}

fn now_unix() -> i64 {
    chrono::Local::now().timestamp()
}

fn entry_is_fresh(entry: &CacheEntry) -> bool {
    now_unix().saturating_sub(entry.timestamp) <= CACHE_TTL.as_secs() as i64
}

fn temp_path(path: &Path) -> PathBuf {
    let mut temp = path.to_path_buf();
    let file_name = path
        .file_name()
        .map(|name| format!("{}.tmp", name.to_string_lossy()))
        .unwrap_or_else(|| "cache.json.tmp".to_string());
    temp.set_file_name(file_name);
    temp
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_quote(symbol: &str, price: f64) -> QuoteCache {
        QuoteCache {
            symbol: symbol.to_string(),
            last_price: price,
            previous_close: 0.0,
            currency: "USD".to_string(),
            open: 0.0,
            day_high: 0.0,
            day_low: 0.0,
            volume: 0,
            market_cap: 0,
            fifty_two_week_high: 0.0,
            fifty_two_week_low: 0.0,
        }
    }

    fn new_test_cache() -> Cache {
        let dir = tempdir().expect("tempdir");
        Cache::with_path(dir.keep().join("cache.json"))
    }

    #[test]
    fn set_and_get() {
        let mut cache = new_test_cache();
        cache.set("AAPL", sample_quote("AAPL", 150.0));

        let got = cache.get("AAPL").expect("cached entry");
        assert_eq!(got.symbol, "AAPL");
        assert_eq!(got.last_price, 150.0);
    }

    #[test]
    fn get_missing_key() {
        let cache = new_test_cache();
        assert!(cache.get("MISSING").is_none());
    }

    #[test]
    fn set_multiple() {
        let mut cache = new_test_cache();
        cache.set_multiple(HashMap::from([
            ("AAPL".to_string(), sample_quote("AAPL", 150.0)),
            ("GOOG".to_string(), sample_quote("GOOG", 2800.0)),
        ]));

        assert_eq!(cache.get("AAPL").unwrap().last_price, 150.0);
        assert_eq!(cache.get("GOOG").unwrap().last_price, 2800.0);
    }

    #[test]
    fn expired_entry_returns_none() {
        let mut cache = new_test_cache();
        cache.entries.insert(
            "OLD".to_string(),
            CacheEntry {
                data: sample_quote("OLD", 100.0),
                timestamp: now_unix() - CACHE_TTL.as_secs() as i64 - 60,
            },
        );

        assert!(cache.get("OLD").is_none());
    }

    #[test]
    fn fresh_entry_returns_some() {
        let mut cache = new_test_cache();
        cache.set("FRESH", sample_quote("FRESH", 200.0));
        assert!(cache.get("FRESH").is_some());
    }

    #[test]
    fn cache_persistence() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("cache.json");

        let mut first = Cache::with_path(&path);
        first.set("AAPL", sample_quote("AAPL", 150.0));
        assert!(path.exists());

        let mut second = Cache::with_path(path);
        second.load();

        let got = second.get("AAPL").expect("reloaded entry");
        assert_eq!(got.last_price, 150.0);
    }
}
