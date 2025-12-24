package cli

import (
	"github.com/spf13/cobra"
)

var rootCmd = &cobra.Command{
	Use:   "fincli",
	Short: "A CLI tool for financial data",
	Long:  "FinCLI is a command-line tool to fetch stock/ETF data and manage portfolios.",
}

func Execute() error {
	return rootCmd.Execute()
}

func init() {
	rootCmd.AddCommand(tickerCmd)
	rootCmd.AddCommand(portfolioCmd)
}
