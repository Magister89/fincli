package display

import "testing"

func TestFormatWithThousands(t *testing.T) {
	tests := []struct {
		name     string
		value    float64
		decimals int
		want     string
	}{
		{"zero", 0, 2, "0.00"},
		{"small", 123.45, 2, "123.45"},
		{"thousands", 1234.56, 2, "1,234.56"},
		{"millions", 1234567.89, 2, "1,234,567.89"},
		{"negative", -1234.56, 2, "-1,234.56"},
		{"no decimals", 1234567, 0, "1,234,567"},
		{"large negative", -9876543.21, 2, "-9,876,543.21"},
		{"one decimal", 1234.5, 1, "1,234.5"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := FormatWithThousands(tt.value, tt.decimals)
			if got != tt.want {
				t.Errorf("FormatWithThousands(%v, %d) = %q, want %q", tt.value, tt.decimals, got, tt.want)
			}
		})
	}
}

func TestFormatIntWithThousands(t *testing.T) {
	tests := []struct {
		name  string
		value int64
		want  string
	}{
		{"zero", 0, "0"},
		{"small", 123, "123"},
		{"thousands", 1234, "1,234"},
		{"millions", 1234567, "1,234,567"},
		{"billions", 1234567890, "1,234,567,890"},
		{"negative", -1234, "-1,234"},
		{"negative millions", -1234567, "-1,234,567"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := FormatIntWithThousands(tt.value)
			if got != tt.want {
				t.Errorf("FormatIntWithThousands(%d) = %q, want %q", tt.value, got, tt.want)
			}
		})
	}
}
