package cli

import (
	"os"
	"path/filepath"

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

		// Check if single or multi-currency portfolio
		if p.IsSingleCurrency() {
			currency := p.GetCurrency()

			if showTotalOnly {
				display.PrintTotalOnly(p.GetTotalValue(), p.GetTotalPnL(), currency)
				return nil
			}

			// Build display rows
			items := p.GetItems()
			rows := make([]display.PortfolioRow, len(items))
			for i, item := range items {
				rows[i] = display.PortfolioRow{
					Ticker:   item.Ticker,
					Value:    item.Price,
					PnL:      item.PnL,
					Currency: item.Currency,
				}
			}

			display.PrintPortfolioTable(rows, true, p.GetTotalValue(), p.GetTotalPnL(), currency)
			return nil
		}

		// Multi-currency portfolio
		currencyGroups := p.GetCurrencyGroups()
		displayGroups := make([]display.CurrencyGroup, len(currencyGroups))

		for i, group := range currencyGroups {
			rows := make([]display.PortfolioRow, len(group.Items))
			for j, item := range group.Items {
				rows[j] = display.PortfolioRow{
					Ticker:   item.Ticker,
					Value:    item.Price,
					PnL:      item.PnL,
					Currency: item.Currency,
				}
			}
			displayGroups[i] = display.CurrencyGroup{
				Currency:   group.Currency,
				Rows:       rows,
				TotalValue: group.TotalValue,
				TotalPnL:   group.TotalPnL,
			}
		}

		if showTotalOnly {
			display.PrintMultiCurrencyTotalOnly(displayGroups)
			return nil
		}

		display.PrintMultiCurrencyPortfolio(displayGroups)
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
