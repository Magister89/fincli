package portfolio

import (
	"os"
	"path/filepath"
	"testing"
)

func TestLoadPortfolio(t *testing.T) {
	t.Run("valid portfolio", func(t *testing.T) {
		content := `[{"ticker": "AAPL", "shares": 10}, {"ticker": "GOOG", "shares": 5}]`
		path := createTempFile(t, content)

		items, err := LoadPortfolio(path)
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}

		if len(items) != 2 {
			t.Errorf("got %d items, want 2", len(items))
		}
		if items[0].Ticker != "AAPL" || items[0].Shares != 10 {
			t.Errorf("item 0: got %+v, want {Ticker:AAPL Shares:10}", items[0])
		}
	})

	t.Run("file not found", func(t *testing.T) {
		_, err := LoadPortfolio("/nonexistent/path.json")
		if err == nil {
			t.Error("expected error for missing file")
		}
	})

	t.Run("invalid JSON", func(t *testing.T) {
		path := createTempFile(t, "not valid json")

		_, err := LoadPortfolio(path)
		if err == nil {
			t.Error("expected error for invalid JSON")
		}
	})

	t.Run("missing ticker", func(t *testing.T) {
		content := `[{"shares": 10}]`
		path := createTempFile(t, content)

		_, err := LoadPortfolio(path)
		if err == nil {
			t.Error("expected error for missing ticker")
		}
	})

	t.Run("zero shares", func(t *testing.T) {
		content := `[{"ticker": "AAPL", "shares": 0}]`
		path := createTempFile(t, content)

		_, err := LoadPortfolio(path)
		if err == nil {
			t.Error("expected error for zero shares")
		}
	})

	t.Run("negative shares", func(t *testing.T) {
		content := `[{"ticker": "AAPL", "shares": -5}]`
		path := createTempFile(t, content)

		_, err := LoadPortfolio(path)
		if err == nil {
			t.Error("expected error for negative shares")
		}
	})

	t.Run("empty portfolio", func(t *testing.T) {
		content := `[]`
		path := createTempFile(t, content)

		items, err := LoadPortfolio(path)
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		if len(items) != 0 {
			t.Errorf("got %d items, want 0", len(items))
		}
	})
}

func createTempFile(t *testing.T, content string) string {
	t.Helper()
	dir := t.TempDir()
	path := filepath.Join(dir, "portfolio.json")
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		t.Fatalf("failed to create temp file: %v", err)
	}
	return path
}
