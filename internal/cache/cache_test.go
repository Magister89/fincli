package cache

import (
	"os"
	"path/filepath"
	"testing"
	"time"
)

func TestCache(t *testing.T) {
	t.Run("set and get", func(t *testing.T) {
		c := newTestCache(t)

		data := QuoteCache{
			Symbol:    "AAPL",
			LastPrice: 150.0,
			Currency:  "USD",
		}

		c.Set("AAPL", data)

		got, ok := c.Get("AAPL")
		if !ok {
			t.Fatal("expected to find cached entry")
		}
		if got.Symbol != "AAPL" || got.LastPrice != 150.0 {
			t.Errorf("got %+v, want Symbol=AAPL LastPrice=150.0", got)
		}
	})

	t.Run("get missing key", func(t *testing.T) {
		c := newTestCache(t)

		_, ok := c.Get("MISSING")
		if ok {
			t.Error("expected not to find missing key")
		}
	})

	t.Run("set multiple", func(t *testing.T) {
		c := newTestCache(t)

		quotes := map[string]QuoteCache{
			"AAPL": {Symbol: "AAPL", LastPrice: 150.0},
			"GOOG": {Symbol: "GOOG", LastPrice: 2800.0},
		}

		c.SetMultiple(quotes)

		aapl, ok := c.Get("AAPL")
		if !ok || aapl.LastPrice != 150.0 {
			t.Errorf("AAPL: got %+v, ok=%v", aapl, ok)
		}

		goog, ok := c.Get("GOOG")
		if !ok || goog.LastPrice != 2800.0 {
			t.Errorf("GOOG: got %+v, ok=%v", goog, ok)
		}
	})

	t.Run("expired entry returns false", func(t *testing.T) {
		c := newTestCache(t)

		// Manually insert an expired entry
		c.mu.Lock()
		c.entries["OLD"] = CacheEntry{
			Data:      QuoteCache{Symbol: "OLD", LastPrice: 100.0},
			Timestamp: time.Now().Add(-CacheTTL - time.Minute).Unix(),
		}
		c.mu.Unlock()

		_, ok := c.Get("OLD")
		if ok {
			t.Error("expected expired entry to not be found")
		}
	})

	t.Run("fresh entry returns true", func(t *testing.T) {
		c := newTestCache(t)

		c.Set("FRESH", QuoteCache{Symbol: "FRESH", LastPrice: 200.0})

		_, ok := c.Get("FRESH")
		if !ok {
			t.Error("expected fresh entry to be found")
		}
	})
}

func newTestCache(t *testing.T) *Cache {
	t.Helper()
	dir := t.TempDir()
	path := filepath.Join(dir, "cache.json")

	return &Cache{
		entries: make(map[string]CacheEntry),
		path:    path,
	}
}

func TestCachePersistence(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "cache.json")

	// Create and populate cache
	c1 := &Cache{
		entries: make(map[string]CacheEntry),
		path:    path,
	}
	c1.Set("AAPL", QuoteCache{Symbol: "AAPL", LastPrice: 150.0})

	// Verify file was created
	if _, err := os.Stat(path); os.IsNotExist(err) {
		t.Fatal("cache file was not created")
	}

	// Create new cache and load from file
	c2 := &Cache{
		entries: make(map[string]CacheEntry),
		path:    path,
	}
	c2.load()

	got, ok := c2.Get("AAPL")
	if !ok {
		t.Fatal("expected to find entry after reload")
	}
	if got.LastPrice != 150.0 {
		t.Errorf("got LastPrice=%v, want 150.0", got.LastPrice)
	}
}
