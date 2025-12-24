package finance

// QuoteData contains the essential financial data for a ticker
type QuoteData struct {
	Symbol        string
	LastPrice     float64
	PreviousClose float64
	Currency      string
	Open          float64
	DayHigh       float64
	DayLow        float64
	Volume        int64
	MarketCap     int64
	FiftyTwoWeekHigh float64
	FiftyTwoWeekLow  float64
}

// yahooChartResponse represents the Yahoo Finance API response structure
type yahooChartResponse struct {
	Chart struct {
		Result []struct {
			Meta struct {
				Currency           string  `json:"currency"`
				Symbol             string  `json:"symbol"`
				RegularMarketPrice float64 `json:"regularMarketPrice"`
				PreviousClose      float64 `json:"previousClose"`
				ChartPreviousClose float64 `json:"chartPreviousClose"`
				RegularMarketVolume int64  `json:"regularMarketVolume"`
				RegularMarketDayHigh float64 `json:"regularMarketDayHigh"`
				RegularMarketDayLow  float64 `json:"regularMarketDayLow"`
				RegularMarketOpen    float64 `json:"regularMarketOpen"`
				FiftyTwoWeekHigh     float64 `json:"fiftyTwoWeekHigh"`
				FiftyTwoWeekLow      float64 `json:"fiftyTwoWeekLow"`
			} `json:"meta"`
		} `json:"result"`
		Error *struct {
			Code        string `json:"code"`
			Description string `json:"description"`
		} `json:"error"`
	} `json:"chart"`
}

// yahooQuoteResponse for the quote endpoint (used for batch queries and more info)
type yahooQuoteResponse struct {
	QuoteResponse struct {
		Result []struct {
			Symbol             string  `json:"symbol"`
			RegularMarketPrice float64 `json:"regularMarketPrice"`
			RegularMarketPreviousClose float64 `json:"regularMarketPreviousClose"`
			Currency           string  `json:"currency"`
			RegularMarketOpen  float64 `json:"regularMarketOpen"`
			RegularMarketDayHigh float64 `json:"regularMarketDayHigh"`
			RegularMarketDayLow  float64 `json:"regularMarketDayLow"`
			RegularMarketVolume int64   `json:"regularMarketVolume"`
			MarketCap          int64   `json:"marketCap"`
			FiftyTwoWeekHigh   float64 `json:"fiftyTwoWeekHigh"`
			FiftyTwoWeekLow    float64 `json:"fiftyTwoWeekLow"`
		} `json:"result"`
		Error *struct {
			Code        string `json:"code"`
			Description string `json:"description"`
		} `json:"error"`
	} `json:"quoteResponse"`
}
