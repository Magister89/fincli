package display

import "github.com/charmbracelet/lipgloss"

// Catppuccin Frappe palette values, matching the user's Powerlevel10k zsh theme.
const (
	catppuccinFrappeText     = "#c6d0f5"
	catppuccinFrappeOverlay0 = "#737994"
	catppuccinFrappeBlue     = "#8caaee"
	catppuccinFrappeLavender = "#babbf1"
	catppuccinFrappeGreen    = "#a6d189"
	catppuccinFrappeRed      = "#e78284"
	catppuccinFrappeYellow   = "#e5c890"
)

var (
	greenStyle   = lipgloss.NewStyle().Foreground(lipgloss.Color(catppuccinFrappeGreen))
	redStyle     = lipgloss.NewStyle().Foreground(lipgloss.Color(catppuccinFrappeRed))
	blueStyle    = lipgloss.NewStyle().Foreground(lipgloss.Color(catppuccinFrappeBlue))
	boldStyle    = lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color(catppuccinFrappeText))
	headerStyle  = lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color(catppuccinFrappeLavender))
	dimStyle     = lipgloss.NewStyle().Foreground(lipgloss.Color(catppuccinFrappeOverlay0))
	warningStyle = lipgloss.NewStyle().Foreground(lipgloss.Color(catppuccinFrappeYellow))
)

// RenderWarning applies the Catppuccin warning color to a message.
func RenderWarning(message string) string {
	return warningStyle.Render(message)
}
