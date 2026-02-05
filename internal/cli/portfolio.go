package cli

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/giorgio/fincli/internal/display"
	"github.com/giorgio/fincli/internal/portfolio"
	"github.com/spf13/cobra"
)

var (
	showTotalOnly bool
	portfolioFile string
)

var portfolioCmd = &cobra.Command{
	Use:   "portfolio",
	Short: "Display portfolio with real-time values",
	Long:  "Load a portfolio from JSON and display current values with P&L.",
	RunE: func(cmd *cobra.Command, args []string) error {
		p, err := portfolio.New(portfolioFile)
		if err != nil {
			return err
		}

		// Warn about skipped tickers
		if skipped := p.GetSkipped(); len(skipped) > 0 {
			fmt.Fprintf(os.Stderr, "Warning: failed to fetch data for: %s\n", strings.Join(skipped, ", "))
		}

		// Check if single or multi-currency portfolio
		if p.IsSingleCurrency() {
			currency := p.GetCurrency()

			if showTotalOnly {
				display.PrintTotalOnly(p.GetTotalValue(), p.GetTotalPnL(), currency)
			} else {
				display.PrintPortfolioTable(p.GetItems(), true, p.GetTotalValue(), p.GetTotalPnL(), currency)
			}
		} else {
			// Multi-currency portfolio
			groups := p.GetCurrencyGroups()

			if showTotalOnly {
				display.PrintMultiCurrencyTotalOnly(groups)
			} else {
				display.PrintMultiCurrencyPortfolio(groups)
			}
		}

		display.PrintCacheFooter(p.GetFetchInfo())
		return nil
	},
}

func init() {
	defaultPath := "portfolio.json"
	if homeDir, err := os.UserHomeDir(); err == nil {
		defaultPath = filepath.Join(homeDir, ".fincli", "portfolio.json")
	}

	portfolioCmd.Flags().BoolVarP(&showTotalOnly, "total", "t", false, "Show only total portfolio value")
	portfolioCmd.Flags().StringVarP(&portfolioFile, "file", "f", defaultPath, "Path to portfolio JSON file")
}
