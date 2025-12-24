package main

import (
	"os"

	"github.com/giorgio/fincli/internal/cli"
)

func main() {
	if err := cli.Execute(); err != nil {
		os.Exit(1)
	}
}
