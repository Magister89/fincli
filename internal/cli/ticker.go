package cli

import (
	"fmt"

	"github.com/giorgio/fincli/internal/display"
	"github.com/giorgio/fincli/internal/finance"
	"github.com/spf13/cobra"
)

var (
	showInfo      bool
	attributeFlag string
)

var tickerCmd = &cobra.Command{
	Use:   "ticker <symbol>",
	Short: "Get information about a stock or ETF",
	Long:  "Fetch and display financial data for a given ticker symbol.",
	Args:  cobra.ExactArgs(1),
	PreRunE: func(cmd *cobra.Command, args []string) error {
		// Validate mutually exclusive options
		if showInfo && attributeFlag != "" {
			return fmt.Errorf("--info and --attribute are mutually exclusive")
		}
		return nil
	},
	RunE: func(cmd *cobra.Command, args []string) error {
		symbol := args[0]

		ticker, err := finance.NewTicker(symbol)
		if err != nil {
			return fmt.Errorf("fetching ticker data: %w", err)
		}

		if attributeFlag != "" {
			return printAttribute(symbol, ticker, attributeFlag)
		}

		if showInfo {
			return printFullInfo(symbol, ticker)
		}

		// Default: show fast info
		return printFastInfo(symbol, ticker)
	},
}

func init() {
	tickerCmd.Flags().BoolVarP(&showInfo, "info", "i", false, "Show full ticker information")
	tickerCmd.Flags().StringVarP(&attributeFlag, "attribute", "a", "", "Show specific attribute")
}

func printFastInfo(symbol string, ticker *finance.Ticker) error {
	info := ticker.GetFastInfo()
	rows := []display.TickerInfoRow{
		{Attribute: "lastPrice", Value: formatValue(info["lastPrice"])},
		{Attribute: "previousClose", Value: formatValue(info["previousClose"])},
		{Attribute: "open", Value: formatValue(info["open"])},
		{Attribute: "dayHigh", Value: formatValue(info["dayHigh"])},
		{Attribute: "dayLow", Value: formatValue(info["dayLow"])},
		{Attribute: "volume", Value: formatValue(info["volume"])},
		{Attribute: "currency", Value: formatValue(info["currency"])},
	}
	display.PrintTickerInfo(symbol, rows)
	return nil
}

func printFullInfo(symbol string, ticker *finance.Ticker) error {
	info := ticker.GetInfo()
	rows := []display.TickerInfoRow{
		{Attribute: "lastPrice", Value: formatValue(info["lastPrice"])},
		{Attribute: "previousClose", Value: formatValue(info["previousClose"])},
		{Attribute: "open", Value: formatValue(info["open"])},
		{Attribute: "dayHigh", Value: formatValue(info["dayHigh"])},
		{Attribute: "dayLow", Value: formatValue(info["dayLow"])},
		{Attribute: "volume", Value: formatValue(info["volume"])},
		{Attribute: "marketCap", Value: formatValue(info["marketCap"])},
		{Attribute: "fiftyTwoWeekHigh", Value: formatValue(info["fiftyTwoWeekHigh"])},
		{Attribute: "fiftyTwoWeekLow", Value: formatValue(info["fiftyTwoWeekLow"])},
		{Attribute: "currency", Value: formatValue(info["currency"])},
	}
	display.PrintTickerInfo(symbol, rows)
	return nil
}

func printAttribute(symbol string, ticker *finance.Ticker, attr string) error {
	info := ticker.GetInfo()
	value, ok := info[attr]
	if !ok {
		return fmt.Errorf("unknown attribute: %s", attr)
	}
	display.PrintSingleAttribute(symbol, attr, formatValue(value))
	return nil
}

func formatValue(v interface{}) string {
	switch val := v.(type) {
	case float64:
		return fmt.Sprintf("%.2f", val)
	case int64:
		return fmt.Sprintf("%d", val)
	case string:
		return val
	default:
		return fmt.Sprintf("%v", val)
	}
}
