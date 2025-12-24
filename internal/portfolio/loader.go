package portfolio

import (
	"encoding/json"
	"fmt"
	"os"
)

// PortfolioItem represents a single holding in the portfolio JSON
type PortfolioItem struct {
	Ticker string `json:"ticker"`
	Shares int    `json:"shares"`
}

// LoadPortfolio loads and validates a portfolio from a JSON file
func LoadPortfolio(filePath string) ([]PortfolioItem, error) {
	data, err := os.ReadFile(filePath)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, fmt.Errorf("portfolio file not found: %s", filePath)
		}
		return nil, fmt.Errorf("reading portfolio file: %w", err)
	}

	var items []PortfolioItem
	if err := json.Unmarshal(data, &items); err != nil {
		return nil, fmt.Errorf("invalid JSON format: %w", err)
	}

	// Validate each item
	for i, item := range items {
		if item.Ticker == "" {
			return nil, fmt.Errorf("item %d: missing 'ticker' field", i)
		}
		if item.Shares <= 0 {
			return nil, fmt.Errorf("item %d: 'shares' must be positive", i)
		}
	}

	return items, nil
}
