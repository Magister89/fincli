package cache

import (
	"encoding/json"
	"os"
	"path/filepath"
	"sync"
	"time"
)

const (
	// CacheTTL is the time-to-live for cached entries (2 minutes)
	CacheTTL = 2 * time.Minute
	// CacheDir is the directory name for cache storage
	CacheDir = ".fincli"
	// CacheFile is the cache filename
	CacheFile = "cache.json"
)

// QuoteCache holds cached quote data
type QuoteCache struct {
	Symbol           string  `json:"symbol"`
	LastPrice        float64 `json:"lastPrice"`
	PreviousClose    float64 `json:"previousClose"`
	Currency         string  `json:"currency"`
	Open             float64 `json:"open"`
	DayHigh          float64 `json:"dayHigh"`
	DayLow           float64 `json:"dayLow"`
	Volume           int64   `json:"volume"`
	MarketCap        int64   `json:"marketCap"`
	FiftyTwoWeekHigh float64 `json:"fiftyTwoWeekHigh"`
	FiftyTwoWeekLow  float64 `json:"fiftyTwoWeekLow"`
}

// CacheEntry holds cached data with timestamp
type CacheEntry struct {
	Data      QuoteCache `json:"data"`
	Timestamp int64      `json:"timestamp"`
}

// Cache provides file-based caching for quote data
type Cache struct {
	entries map[string]CacheEntry
	path    string
	mu      sync.RWMutex
}

// New creates a new Cache instance
func New() (*Cache, error) {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		return nil, err
	}

	cacheDir := filepath.Join(homeDir, CacheDir)
	if err := os.MkdirAll(cacheDir, 0755); err != nil {
		return nil, err
	}

	c := &Cache{
		entries: make(map[string]CacheEntry),
		path:    filepath.Join(cacheDir, CacheFile),
	}

	// Load existing cache (ignore errors for missing file)
	c.load()

	return c, nil
}

// Get retrieves cached data if valid (not expired)
func (c *Cache) Get(symbol string) (*QuoteCache, bool) {
	c.mu.RLock()
	defer c.mu.RUnlock()

	entry, ok := c.entries[symbol]
	if !ok {
		return nil, false
	}

	// Check if entry has expired
	if time.Since(time.Unix(entry.Timestamp, 0)) > CacheTTL {
		return nil, false
	}

	return &entry.Data, true
}

// Set stores data in the cache with current timestamp
func (c *Cache) Set(symbol string, data QuoteCache) {
	c.mu.Lock()
	defer c.mu.Unlock()

	c.entries[symbol] = CacheEntry{
		Data:      data,
		Timestamp: time.Now().Unix(),
	}

	// Save to file (ignore errors, cache is best-effort)
	c.save()
}

// SetMultiple stores multiple entries at once
func (c *Cache) SetMultiple(quotes map[string]QuoteCache) {
	c.mu.Lock()
	defer c.mu.Unlock()

	now := time.Now().Unix()
	for symbol, data := range quotes {
		c.entries[symbol] = CacheEntry{
			Data:      data,
			Timestamp: now,
		}
	}

	c.save()
}

// load reads the cache from disk
func (c *Cache) load() {
	data, err := os.ReadFile(c.path)
	if err != nil {
		return
	}

	json.Unmarshal(data, &c.entries)
}

// save writes the cache to disk atomically
func (c *Cache) save() {
	data, err := json.MarshalIndent(c.entries, "", "  ")
	if err != nil {
		return
	}

	// Write to temp file first, then rename for atomic operation
	tempPath := c.path + ".tmp"
	if err := os.WriteFile(tempPath, data, 0600); err != nil {
		return
	}
	os.Rename(tempPath, c.path)
}
