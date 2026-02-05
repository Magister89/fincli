package portfolio

import (
	"time"

	"github.com/giorgio/fincli/internal/finance"
)

// FetchInfo holds metadata about when and how quote data was fetched
type FetchInfo struct {
	OldestFetchedAt time.Time
	NewestFetchedAt time.Time
	AllFromCache    bool
	AnyFromCache    bool
}

// EnrichedItem contains portfolio item with real-time market data
type EnrichedItem struct {
	Ticker        string
	Shares        int
	Price         float64 // shares * lastPrice
	PreviousClose float64 // shares * previousClose
	PnL           float64 // percentage change
	Currency      string
}

// Portfolio holds the enriched portfolio data
type Portfolio struct {
	items      []EnrichedItem
	totalValue float64
	skipped    []string // tickers that failed to fetch
	fetchInfo  FetchInfo
}

// New creates a new Portfolio from a file
func New(filePath string) (*Portfolio, error) {
	rawItems, err := LoadPortfolio(filePath)
	if err != nil {
		return nil, err
	}

	p := &Portfolio{}
	if err := p.enrich(rawItems); err != nil {
		return nil, err
	}

	return p, nil
}

// enrich fetches market data and calculates values
func (p *Portfolio) enrich(items []PortfolioItem) error {
	if len(items) == 0 {
		return nil
	}

	// Collect all symbols
	symbols := make([]string, len(items))
	for i, item := range items {
		symbols[i] = item.Ticker
	}

	// Batch fetch quotes
	client := finance.NewClient()
	quotes, err := client.GetQuotes(symbols)
	if err != nil {
		return err
	}

	// Enrich each item
	p.items = make([]EnrichedItem, 0, len(items))
	p.skipped = make([]string, 0)
	p.totalValue = 0
	p.fetchInfo = FetchInfo{AllFromCache: true}

	first := true
	for _, item := range items {
		quote, ok := quotes[item.Ticker]
		if !ok {
			p.skipped = append(p.skipped, item.Ticker)
			continue
		}

		// Track fetch metadata
		if first {
			p.fetchInfo.OldestFetchedAt = quote.FetchedAt
			p.fetchInfo.NewestFetchedAt = quote.FetchedAt
			first = false
		} else {
			if quote.FetchedAt.Before(p.fetchInfo.OldestFetchedAt) {
				p.fetchInfo.OldestFetchedAt = quote.FetchedAt
			}
			if quote.FetchedAt.After(p.fetchInfo.NewestFetchedAt) {
				p.fetchInfo.NewestFetchedAt = quote.FetchedAt
			}
		}
		if quote.FromCache {
			p.fetchInfo.AnyFromCache = true
		} else {
			p.fetchInfo.AllFromCache = false
		}

		price := float64(item.Shares) * quote.LastPrice
		prevClose := float64(item.Shares) * quote.PreviousClose

		var pnl float64
		if prevClose > 0 {
			pnl = ((price / prevClose) - 1) * 100
		}

		p.items = append(p.items, EnrichedItem{
			Ticker:        item.Ticker,
			Shares:        item.Shares,
			Price:         price,
			PreviousClose: prevClose,
			PnL:           pnl,
			Currency:      quote.Currency,
		})

		p.totalValue += price
	}

	return nil
}

// GetItems returns the enriched portfolio items
func (p *Portfolio) GetItems() []EnrichedItem {
	return p.items
}

// GetTotalValue returns the total portfolio value
func (p *Portfolio) GetTotalValue() float64 {
	return p.totalValue
}

// GetTickers returns the list of ticker symbols
func (p *Portfolio) GetTickers() []string {
	tickers := make([]string, len(p.items))
	for i, item := range p.items {
		tickers[i] = item.Ticker
	}
	return tickers
}

// GetTotalPnL returns the total portfolio P&L percentage
func (p *Portfolio) GetTotalPnL() float64 {
	var totalPrevClose float64
	for _, item := range p.items {
		totalPrevClose += item.PreviousClose
	}
	if totalPrevClose == 0 {
		return 0
	}
	return ((p.totalValue / totalPrevClose) - 1) * 100
}

// CurrencyGroup holds items and totals for a single currency
type CurrencyGroup struct {
	Currency   string
	Items      []EnrichedItem
	TotalValue float64
	TotalPnL   float64
}

// GetCurrencyGroups returns items grouped by currency with subtotals
func (p *Portfolio) GetCurrencyGroups() []CurrencyGroup {
	// Group items by currency
	groups := make(map[string]*CurrencyGroup)
	order := []string{} // preserve order of first occurrence

	for _, item := range p.items {
		if _, exists := groups[item.Currency]; !exists {
			groups[item.Currency] = &CurrencyGroup{
				Currency: item.Currency,
				Items:    []EnrichedItem{},
			}
			order = append(order, item.Currency)
		}
		groups[item.Currency].Items = append(groups[item.Currency].Items, item)
		groups[item.Currency].TotalValue += item.Price
	}

	// Calculate P&L for each group
	for _, group := range groups {
		var totalPrevClose float64
		for _, item := range group.Items {
			totalPrevClose += item.PreviousClose
		}
		if totalPrevClose > 0 {
			group.TotalPnL = ((group.TotalValue / totalPrevClose) - 1) * 100
		}
	}

	// Return in order of first occurrence
	result := make([]CurrencyGroup, len(order))
	for i, currency := range order {
		result[i] = *groups[currency]
	}

	return result
}

// IsSingleCurrency returns true if all items have the same currency
func (p *Portfolio) IsSingleCurrency() bool {
	if len(p.items) == 0 {
		return true
	}
	currency := p.items[0].Currency
	for _, item := range p.items {
		if item.Currency != currency {
			return false
		}
	}
	return true
}

// GetCurrency returns the currency if single currency portfolio, empty string otherwise
func (p *Portfolio) GetCurrency() string {
	if len(p.items) == 0 {
		return ""
	}
	if p.IsSingleCurrency() {
		return p.items[0].Currency
	}
	return ""
}

// GetSkipped returns tickers that failed to fetch
func (p *Portfolio) GetSkipped() []string {
	return p.skipped
}

// GetFetchInfo returns metadata about when data was fetched
func (p *Portfolio) GetFetchInfo() FetchInfo {
	return p.fetchInfo
}
