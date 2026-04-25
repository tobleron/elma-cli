package discover

import (
	"fmt"
	"io"
	"time"
)

// NewProgressPrinter returns a ProgressFunc that prints a single-line indicator.
func NewProgressPrinter(w io.Writer, threshold int, minInterval time.Duration) ProgressFunc {
	if threshold <= 0 {
		threshold = 500
	}
	if minInterval <= 0 {
		minInterval = 250 * time.Millisecond
	}

	var (
		started   bool
		lastPrint time.Time
	)

	return func(stats Stats) {
		if stats.Done {
			if started {
				if stats.Markdown > 0 {
					_, _ = fmt.Fprintf(w, "\rScanned %d entries, found %d PDFs and %d Markdown files in %s\n", stats.Visited, stats.PDFs, stats.Markdown, stats.Elapsed.Truncate(time.Second))
				} else {
					_, _ = fmt.Fprintf(w, "\rScanned %d entries, found %d PDFs in %s\n", stats.Visited, stats.PDFs, stats.Elapsed.Truncate(time.Second))
				}
			}
			return
		}
		if stats.Visited < threshold {
			return
		}
		now := time.Now()
		if !started {
			started = true
			lastPrint = time.Time{}
		}
		if lastPrint.IsZero() || now.Sub(lastPrint) >= minInterval {
			if stats.Markdown > 0 {
				_, _ = fmt.Fprintf(w, "\rScanning... %d entries, %d PDFs, %d Markdown", stats.Visited, stats.PDFs, stats.Markdown)
			} else {
				_, _ = fmt.Fprintf(w, "\rScanning... %d entries, %d PDFs", stats.Visited, stats.PDFs)
			}
			lastPrint = now
		}
	}
}
