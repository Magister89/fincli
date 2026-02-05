package display

import (
	"fmt"
	"strings"
	"time"

	"github.com/charmbracelet/lipgloss"
	"github.com/giorgio/fincli/internal/portfolio"
)

// FormatWithThousands formats a float64 with thousands separator (comma)
func FormatWithThousands(value float64, decimals int) string {
	// Format the number with specified decimals
	format := fmt.Sprintf("%%.%df", decimals)
	str := fmt.Sprintf(format, value)

	// Split integer and decimal parts
	parts := strings.Split(str, ".")
	intPart := parts[0]

	// Handle negative numbers
	negative := false
	if len(intPart) > 0 && intPart[0] == '-' {
		negative = true
		intPart = intPart[1:]
	}

	// Add thousands separators
	var result strings.Builder
	for i, c := range intPart {
		if i > 0 && (len(intPart)-i)%3 == 0 {
			result.WriteRune(',')
		}
		result.WriteRune(c)
	}

	// Rebuild the number
	formatted := result.String()
	if negative {
		formatted = "-" + formatted
	}
	if len(parts) > 1 {
		formatted = formatted + "." + parts[1]
	}

	return formatted
}

// FormatIntWithThousands formats an int64 with thousands separator (comma)
func FormatIntWithThousands(value int64) string {
	str := fmt.Sprintf("%d", value)

	// Handle negative numbers
	negative := false
	if len(str) > 0 && str[0] == '-' {
		negative = true
		str = str[1:]
	}

	// Add thousands separators
	var result strings.Builder
	for i, c := range str {
		if i > 0 && (len(str)-i)%3 == 0 {
			result.WriteRune(',')
		}
		result.WriteRune(c)
	}

	formatted := result.String()
	if negative {
		formatted = "-" + formatted
	}

	return formatted
}

// Column widths
const (
	colTicker = 12
	colQty    = 8
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

// PrintPortfolioTable prints a formatted portfolio table (single currency)
func PrintPortfolioTable(items []portfolio.EnrichedItem, showTotal bool, totalValue float64, totalPnL float64, currency string) {
	printHeader()
	separator := getSeparator()
	fmt.Println(dimStyle.Render(separator))

	// Print rows
	for _, item := range items {
		printItem(item)
	}

	if showTotal {
		fmt.Println(dimStyle.Render(separator))
		printTotal("Total", totalValue, totalPnL, currency)
	}
}

// PrintMultiCurrencyPortfolio prints a portfolio grouped by currency with subtotals
func PrintMultiCurrencyPortfolio(groups []portfolio.CurrencyGroup) {
	printHeader()
	separator := getSeparator()
	fmt.Println(dimStyle.Render(separator))

	for i, group := range groups {
		// Print rows for this currency group
		for _, item := range group.Items {
			printItem(item)
		}

		// Print subtotal for this currency
		fmt.Println(dimStyle.Render(separator))
		printTotal("Subtotal", group.TotalValue, group.TotalPnL, group.Currency)

		// Add spacing between currency groups (except after last)
		if i < len(groups)-1 {
			fmt.Println()
			printHeader()
			fmt.Println(dimStyle.Render(separator))
		}
	}
}

// printHeader prints the table header
func printHeader() {
	header := fmt.Sprintf("%-*s  %*s  %-*s  %-*s",
		colTicker, "Ticker",
		colQty, "Qty",
		colValue, "Value",
		colPnL, "P&L",
	)
	fmt.Println(headerStyle.Render(header))
}

// getSeparator returns the separator line
func getSeparator() string {
	return fmt.Sprintf("%-*s  %*s  %-*s  %-*s",
		colTicker, "────────────",
		colQty, "────────",
		colValue, "──────────────────",
		colPnL, "────────────",
	)
}

// printItem prints a single portfolio item
func printItem(item portfolio.EnrichedItem) {
	tickerStr := fmt.Sprintf("%-*s", colTicker, item.Ticker)
	qtyStr := fmt.Sprintf("%*s", colQty, FormatIntWithThousands(int64(item.Shares)))
	formattedValue := FormatWithThousands(item.Price, 2)
	valueStr := fmt.Sprintf("%*s %s", colValue-4, formattedValue, item.Currency)
	pnlStr := formatPnL(item.PnL)

	fmt.Printf("%s  %s  %s  %s\n",
		blueStyle.Render(tickerStr),
		qtyStr,
		valueStr,
		pnlStr,
	)
}

// printTotal prints a total/subtotal row
func printTotal(label string, value float64, pnl float64, currency string) {
	totalLabel := fmt.Sprintf("%-*s", colTicker, label)
	qtyPad := fmt.Sprintf("%*s", colQty, "")
	formattedTotal := FormatWithThousands(value, 2)
	totalValueStr := fmt.Sprintf("%*s %s", colValue-4, formattedTotal, currency)
	totalPnLStr := formatPnL(pnl)

	fmt.Printf("%s  %s  %s  %s\n",
		boldStyle.Render(totalLabel),
		qtyPad,
		boldStyle.Render(totalValueStr),
		totalPnLStr,
	)
}

// PrintTotalOnly prints only the total value with P&L (single currency)
func PrintTotalOnly(totalValue float64, totalPnL float64, currency string) {
	// Header
	header := fmt.Sprintf("%-*s  %-*s", 16, "Total Value", 12, "P&L")
	fmt.Println(headerStyle.Render(header))

	// Separator
	separator := fmt.Sprintf("%-*s  %-*s", 16, "────────────────", 12, "────────────")
	fmt.Println(dimStyle.Render(separator))

	// Value row
	formattedValue := FormatWithThousands(totalValue, 2)
	valueStr := fmt.Sprintf("%*s %s", 12, formattedValue, currency)
	pnlStr := formatPnL(totalPnL)
	fmt.Printf("%s  %s\n", boldStyle.Render(valueStr), pnlStr)
}

// PrintMultiCurrencyTotalOnly prints totals per currency when using --total with multi-currency
func PrintMultiCurrencyTotalOnly(groups []portfolio.CurrencyGroup) {
	// Header
	header := fmt.Sprintf("%-*s  %-*s", 16, "Total Value", 12, "P&L")
	fmt.Println(headerStyle.Render(header))

	// Separator
	separator := fmt.Sprintf("%-*s  %-*s", 16, "────────────────", 12, "────────────")
	fmt.Println(dimStyle.Render(separator))

	for _, group := range groups {
		formattedValue := FormatWithThousands(group.TotalValue, 2)
		valueStr := fmt.Sprintf("%*s %s", 12, formattedValue, group.Currency)
		pnlStr := formatPnL(group.TotalPnL)
		fmt.Printf("%s  %s\n", boldStyle.Render(valueStr), pnlStr)
	}
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

// formatDuration returns a human-readable duration string
func formatDuration(d time.Duration) string {
	if d < 5*time.Second {
		return "just now"
	}
	if d < time.Minute {
		return fmt.Sprintf("%d sec ago", int(d.Seconds()))
	}
	return fmt.Sprintf("%d min ago", int(d.Minutes()))
}

// PrintCacheFooter prints a dim footer line showing when data was fetched
func PrintCacheFooter(info portfolio.FetchInfo) {
	if info.OldestFetchedAt.IsZero() {
		return
	}

	var msg string
	now := time.Now()

	if info.AllFromCache {
		age := now.Sub(info.OldestFetchedAt)
		msg = fmt.Sprintf("Data from cache (%s)", formatDuration(age))
	} else if info.AnyFromCache {
		oldestAge := now.Sub(info.OldestFetchedAt)
		msg = fmt.Sprintf("Last updated: %s (oldest data: %s)",
			info.NewestFetchedAt.Format("15:04:05"),
			formatDuration(oldestAge))
	} else {
		msg = fmt.Sprintf("Last updated: %s", info.NewestFetchedAt.Format("15:04:05"))
	}

	fmt.Printf("\n%s\n", dimStyle.Render(msg))
}

// formatPnL formats the P&L value with color and arrow, right-aligned to colPnL
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

	raw := fmt.Sprintf("%s %.2f%%", arrow, pnl)
	return style.Render(fmt.Sprintf("%*s", colPnL, raw))
}
