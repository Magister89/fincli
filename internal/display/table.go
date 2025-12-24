package display

import (
	"fmt"

	"github.com/charmbracelet/lipgloss"
)

var (
	greenStyle  = lipgloss.NewStyle().Foreground(lipgloss.Color("10"))
	redStyle    = lipgloss.NewStyle().Foreground(lipgloss.Color("9"))
	blueStyle   = lipgloss.NewStyle().Foreground(lipgloss.Color("12"))
	boldStyle   = lipgloss.NewStyle().Bold(true)
	headerStyle = lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("15"))
)

// PortfolioRow represents a row in the portfolio table
type PortfolioRow struct {
	Ticker   string
	Value    float64
	PnL      float64
	Currency string
}

// PrintPortfolioTable prints a formatted portfolio table
func PrintPortfolioTable(rows []PortfolioRow, showTotal bool, totalValue float64) {
	// Print header
	fmt.Printf("%-15s %-20s %s\n",
		headerStyle.Render("Ticker"),
		headerStyle.Render("Value"),
		headerStyle.Render("P&L"),
	)
	fmt.Println()

	// Print rows
	for _, row := range rows {
		ticker := blueStyle.Render(fmt.Sprintf("%-15s", row.Ticker))
		value := fmt.Sprintf("%-20s", fmt.Sprintf("%.2f %s", row.Value, row.Currency))
		pnl := formatPnL(row.PnL)

		fmt.Printf("%s %s %s\n", ticker, value, pnl)
	}

	if showTotal {
		fmt.Println()
		fmt.Printf("%s: %.2f\n", boldStyle.Render("Total"), totalValue)
	}
}

// PrintTotalOnly prints only the total value
func PrintTotalOnly(totalValue float64) {
	fmt.Printf("%s: %.2f\n", boldStyle.Render("Total Portfolio Value"), totalValue)
}

// TickerInfoRow represents a row in the ticker info table
type TickerInfoRow struct {
	Attribute string
	Value     string
}

// PrintTickerInfo prints formatted ticker information
func PrintTickerInfo(symbol string, rows []TickerInfoRow) {
	fmt.Printf("%s\n\n", boldStyle.Render(symbol))

	// Print header
	fmt.Printf("%-20s %s\n",
		headerStyle.Render("Attribute"),
		headerStyle.Render("Value"),
	)
	fmt.Println()

	// Print rows
	for _, row := range rows {
		attr := blueStyle.Render(fmt.Sprintf("%-20s", row.Attribute))
		fmt.Printf("%s %s\n", attr, row.Value)
	}
}

// PrintSingleAttribute prints a single attribute value
func PrintSingleAttribute(symbol, attribute, value string) {
	fmt.Printf("%s %s: %s\n", boldStyle.Render(symbol), blueStyle.Render(attribute), value)
}

// formatPnL formats the P&L value with color and arrow
func formatPnL(pnl float64) string {
	var arrow string
	var style lipgloss.Style

	if pnl >= 0 {
		arrow = "▲"
		style = greenStyle
	} else {
		arrow = "▼"
		style = redStyle
	}

	return style.Render(fmt.Sprintf("%s %.2f%%", arrow, pnl))
}
