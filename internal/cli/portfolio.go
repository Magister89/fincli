package cli

import (
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

		if showTotalOnly {
			display.PrintTotalOnly(p.GetTotalValue(), p.GetTotalPnL())
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

		display.PrintPortfolioTable(rows, true, p.GetTotalValue(), p.GetTotalPnL())
		return nil
	},
}

func init() {
	portfolioCmd.Flags().BoolVarP(&showTotalOnly, "total", "t", false, "Show only total portfolio value")
	portfolioCmd.Flags().StringVarP(&portfolioFile, "file", "f", "portfolio.json", "Path to portfolio JSON file")
}
