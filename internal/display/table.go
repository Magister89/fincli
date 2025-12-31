package display

import (
	"fmt"

	"github.com/charmbracelet/lipgloss"
)

// Column widths
const (
	colTicker = 12
	colValue  = 18
	colPnL    = 12
)

var (
	greenStyle  = lipgloss.NewStyle().Foreground(lipgloss.Color("10"))
	redStyle    = lipgloss.NewStyle().Foreground(lipgloss.Color("9"))
	blueStyle   = lipgloss.NewStyle().Foreground(lipgloss.Color("12"))
	boldStyle   = lipgloss.NewStyle().Bold(true)
	headerStyle = lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("15"))
	dimStyle    = lipgloss.NewStyle().Foreground(lipgloss.Color("8"))
)

// PortfolioRow represents a row in the portfolio table
type PortfolioRow struct {
	Ticker   string
	Value    float64
	PnL      float64
	Currency string
}

// PrintPortfolioTable prints a formatted portfolio table
func PrintPortfolioTable(rows []PortfolioRow, showTotal bool, totalValue float64, totalPnL float64) {
	// Print header with proper alignment
	header := fmt.Sprintf("%-*s  %-*s  %-*s",
		colTicker, "Ticker",
		colValue, "Value",
		colPnL, "P&L",
	)
	fmt.Println(headerStyle.Render(header))

	// Separator line
	separator := fmt.Sprintf("%-*s  %-*s  %-*s",
		colTicker, "────────────",
		colValue, "──────────────────",
		colPnL, "────────────",
	)
	fmt.Println(dimStyle.Render(separator))

	// Print rows
	for _, row := range rows {
		tickerStr := fmt.Sprintf("%-*s", colTicker, row.Ticker)
		valueStr := fmt.Sprintf("%*.2f %s", colValue-4, row.Value, row.Currency)
		pnlStr := formatPnL(row.PnL)

		fmt.Printf("%s  %s  %s\n",
			blueStyle.Render(tickerStr),
			valueStr,
			pnlStr,
		)
	}

	if showTotal {
		// Total separator
		fmt.Println(dimStyle.Render(separator))

		// Total row aligned with columns
		totalLabel := fmt.Sprintf("%-*s", colTicker, "Total")
		totalValueStr := fmt.Sprintf("%*.2f EUR", colValue-4, totalValue)
		totalPnLStr := formatPnL(totalPnL)

		fmt.Printf("%s  %s  %s\n",
			boldStyle.Render(totalLabel),
			boldStyle.Render(totalValueStr),
			totalPnLStr,
		)
	}
}

// PrintTotalOnly prints only the total value with P&L
func PrintTotalOnly(totalValue float64, totalPnL float64) {
	// Header
	header := fmt.Sprintf("%-*s  %-*s", 16, "Total Value", 12, "P&L")
	fmt.Println(headerStyle.Render(header))

	// Separator
	separator := fmt.Sprintf("%-*s  %-*s", 16, "────────────────", 12, "────────────")
	fmt.Println(dimStyle.Render(separator))

	// Value row
	valueStr := fmt.Sprintf("%*.2f EUR", 12, totalValue)
	pnlStr := formatPnL(totalPnL)
	fmt.Printf("%s  %s\n", boldStyle.Render(valueStr), pnlStr)
}

// TickerInfoRow represents a row in the ticker info table
type TickerInfoRow struct {
	Attribute string
	Value     string
}

// Column widths for ticker info
const (
	colAttr = 18
	colVal  = 14
)

// PrintTickerInfo prints formatted ticker information
func PrintTickerInfo(symbol string, rows []TickerInfoRow) {
	fmt.Printf("%s\n\n", boldStyle.Render(symbol))

	// Print header with proper alignment
	header := fmt.Sprintf("%-*s  %-*s",
		colAttr, "Attribute",
		colVal, "Value",
	)
	fmt.Println(headerStyle.Render(header))

	// Separator line
	separator := fmt.Sprintf("%-*s  %-*s",
		colAttr, "──────────────────",
		colVal, "──────────────",
	)
	fmt.Println(dimStyle.Render(separator))

	// Print rows
	for _, row := range rows {
		attrStr := fmt.Sprintf("%-*s", colAttr, row.Attribute)
		valueStr := fmt.Sprintf("%*s", colVal, row.Value)

		fmt.Printf("%s  %s\n",
			blueStyle.Render(attrStr),
			valueStr,
		)
	}
}

// PrintSingleAttribute prints a single attribute value in table format
func PrintSingleAttribute(symbol, attribute, value string) {
	fmt.Printf("%s\n\n", boldStyle.Render(symbol))

	// Header
	header := fmt.Sprintf("%-*s  %-*s", colAttr, "Attribute", colVal, "Value")
	fmt.Println(headerStyle.Render(header))

	// Separator
	separator := fmt.Sprintf("%-*s  %-*s", colAttr, "──────────────────", colVal, "──────────────")
	fmt.Println(dimStyle.Render(separator))

	// Value row
	attrStr := fmt.Sprintf("%-*s", colAttr, attribute)
	valueStr := fmt.Sprintf("%*s", colVal, value)
	fmt.Printf("%s  %s\n", blueStyle.Render(attrStr), valueStr)
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
