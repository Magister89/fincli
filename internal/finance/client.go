package finance

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"regexp"
	"strings"
	"sync"
	"time"

	"github.com/giorgio/fincli/internal/cache"
)

var validTickerRegex = regexp.MustCompile(`^[A-Za-z0-9^._-]{1,20}$`)

const (
	chartBaseURL   = "https://query1.finance.yahoo.com/v8/finance/chart"
	userAgent      = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36"
	maxConcurrent  = 10              // Limit concurrent requests to avoid rate limiting
	requestTimeout = 10 * time.Second // Timeout for individual requests
)

// Client is the Yahoo Finance HTTP client
type Client struct {
	httpClient *http.Client
	cache      *cache.Cache
}

// NewClient creates a new Yahoo Finance client
func NewClient() *Client {
	// Initialize cache (ignore errors, cache is optional)
	c, _ := cache.New()

	// Use a custom transport that respects HTTP_PROXY/HTTPS_PROXY env vars
	transport := &http.Transport{
		Proxy: http.ProxyFromEnvironment,
	}

	return &Client{
		httpClient: &http.Client{
			Timeout:   10 * time.Second,
			Transport: transport,
		},
		cache: c,
	}
}

// ValidateSymbol checks if a ticker symbol has a valid format
func ValidateSymbol(symbol string) error {
	symbol = strings.TrimSpace(symbol)
	if symbol == "" {
		return fmt.Errorf("empty ticker symbol")
	}
	if !validTickerRegex.MatchString(symbol) {
		return fmt.Errorf("invalid ticker symbol: %s", symbol)
	}
	return nil
}

// GetQuote fetches quote data for a single ticker
func (c *Client) GetQuote(symbol string) (*QuoteData, error) {
	symbol = strings.TrimSpace(strings.ToUpper(symbol))
	if err := ValidateSymbol(symbol); err != nil {
		return nil, err
	}

	// Check cache first
	if c.cache != nil {
		if cached, ok := c.cache.Get(symbol); ok {
			return &QuoteData{
				Symbol:           cached.Symbol,
				LastPrice:        cached.LastPrice,
				PreviousClose:    cached.PreviousClose,
				Currency:         cached.Currency,
				Open:             cached.Open,
				DayHigh:          cached.DayHigh,
				DayLow:           cached.DayLow,
				Volume:           cached.Volume,
				MarketCap:        cached.MarketCap,
				FiftyTwoWeekHigh: cached.FiftyTwoWeekHigh,
				FiftyTwoWeekLow:  cached.FiftyTwoWeekLow,
			}, nil
		}
	}

	reqURL := fmt.Sprintf("%s/%s", chartBaseURL, url.PathEscape(symbol))

	ctx, cancel := context.WithTimeout(context.Background(), requestTimeout)
	defer cancel()

	req, err := http.NewRequestWithContext(ctx, "GET", reqURL, nil)
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
	quote := &QuoteData{
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
	}

	// Cache the result
	if c.cache != nil {
		c.cache.Set(symbol, cache.QuoteCache{
			Symbol:           quote.Symbol,
			LastPrice:        quote.LastPrice,
			PreviousClose:    quote.PreviousClose,
			Currency:         quote.Currency,
			Open:             quote.Open,
			DayHigh:          quote.DayHigh,
			DayLow:           quote.DayLow,
			Volume:           quote.Volume,
			MarketCap:        quote.MarketCap,
			FiftyTwoWeekHigh: quote.FiftyTwoWeekHigh,
			FiftyTwoWeekLow:  quote.FiftyTwoWeekLow,
		})
	}

	return quote, nil
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
	sem := make(chan struct{}, maxConcurrent)

	for _, symbol := range symbols {
		wg.Add(1)
		go func(sym string) {
			sem <- struct{}{}        // acquire semaphore
			defer func() { <-sem }() // release semaphore
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
