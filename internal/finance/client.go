package finance

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"sync"
	"time"
)

const (
	chartBaseURL = "https://query1.finance.yahoo.com/v8/finance/chart"
	userAgent    = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36"
)

// Client is the Yahoo Finance HTTP client
type Client struct {
	httpClient *http.Client
}

// NewClient creates a new Yahoo Finance client
func NewClient() *Client {
	return &Client{
		httpClient: &http.Client{
			Timeout: 10 * time.Second,
		},
	}
}

// GetQuote fetches quote data for a single ticker
func (c *Client) GetQuote(symbol string) (*QuoteData, error) {
	url := fmt.Sprintf("%s/%s", chartBaseURL, url.PathEscape(symbol))

	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return nil, fmt.Errorf("creating request: %w", err)
	}
	req.Header.Set("User-Agent", userAgent)

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("fetching data: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("unexpected status: %d", resp.StatusCode)
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("reading response: %w", err)
	}

	var chartResp yahooChartResponse
	if err := json.Unmarshal(body, &chartResp); err != nil {
		return nil, fmt.Errorf("parsing response: %w", err)
	}

	if chartResp.Chart.Error != nil {
		return nil, fmt.Errorf("API error: %s", chartResp.Chart.Error.Description)
	}

	if len(chartResp.Chart.Result) == 0 {
		return nil, fmt.Errorf("no data found for symbol: %s", symbol)
	}

	meta := chartResp.Chart.Result[0].Meta
	return &QuoteData{
		Symbol:           meta.Symbol,
		LastPrice:        meta.RegularMarketPrice,
		PreviousClose:    meta.PreviousClose,
		Currency:         meta.Currency,
		Open:             meta.RegularMarketOpen,
		DayHigh:          meta.RegularMarketDayHigh,
		DayLow:           meta.RegularMarketDayLow,
		Volume:           meta.RegularMarketVolume,
		FiftyTwoWeekHigh: meta.FiftyTwoWeekHigh,
		FiftyTwoWeekLow:  meta.FiftyTwoWeekLow,
	}, nil
}

// GetQuotes fetches quote data for multiple tickers using concurrent requests
func (c *Client) GetQuotes(symbols []string) (map[string]*QuoteData, error) {
	if len(symbols) == 0 {
		return nil, fmt.Errorf("no symbols provided")
	}

	result := make(map[string]*QuoteData)
	var mu sync.Mutex
	var wg sync.WaitGroup
	errors := make(chan error, len(symbols))

	for _, symbol := range symbols {
		wg.Add(1)
		go func(sym string) {
			defer wg.Done()
			quote, err := c.GetQuote(sym)
			if err != nil {
				errors <- fmt.Errorf("%s: %w", sym, err)
				return
			}
			mu.Lock()
			result[sym] = quote
			mu.Unlock()
		}(symbol)
	}

	wg.Wait()
	close(errors)

	// Collect any errors
	var errs []error
	for err := range errors {
		errs = append(errs, err)
	}

	if len(errs) > 0 && len(result) == 0 {
		return nil, errs[0]
	}

	return result, nil
}
