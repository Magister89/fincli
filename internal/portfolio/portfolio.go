package portfolio

import (
	"github.com/giorgio/fincli/internal/finance"
)

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
	p.totalValue = 0

	for _, item := range items {
		quote, ok := quotes[item.Ticker]
		if !ok {
			// Skip items without quote data
			continue
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
