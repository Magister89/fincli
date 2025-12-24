package finance

// Ticker wraps a single ticker with its quote data
type Ticker struct {
	client *Client
	data   *QuoteData
}

// NewTicker creates a new Ticker and fetches its data
func NewTicker(symbol string) (*Ticker, error) {
	client := NewClient()
	data, err := client.GetQuote(symbol)
	if err != nil {
		return nil, err
	}
	return &Ticker{
		client: client,
		data:   data,
	}, nil
}

// GetData returns the quote data
func (t *Ticker) GetData() *QuoteData {
	return t.data
}

// GetFastInfo returns commonly used fields as a map
func (t *Ticker) GetFastInfo() map[string]interface{} {
	return map[string]interface{}{
		"symbol":        t.data.Symbol,
		"lastPrice":     t.data.LastPrice,
		"previousClose": t.data.PreviousClose,
		"currency":      t.data.Currency,
		"open":          t.data.Open,
		"dayHigh":       t.data.DayHigh,
		"dayLow":        t.data.DayLow,
		"volume":        t.data.Volume,
	}
}

// GetInfo returns all available fields as a map
func (t *Ticker) GetInfo() map[string]interface{} {
	return map[string]interface{}{
		"symbol":           t.data.Symbol,
		"lastPrice":        t.data.LastPrice,
		"previousClose":    t.data.PreviousClose,
		"currency":         t.data.Currency,
		"open":             t.data.Open,
		"dayHigh":          t.data.DayHigh,
		"dayLow":           t.data.DayLow,
		"volume":           t.data.Volume,
		"marketCap":        t.data.MarketCap,
		"fiftyTwoWeekHigh": t.data.FiftyTwoWeekHigh,
		"fiftyTwoWeekLow":  t.data.FiftyTwoWeekLow,
	}
}
