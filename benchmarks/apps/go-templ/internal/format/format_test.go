package format

import (
	"testing"
	"time"
)

func TestMoney(t *testing.T) {
	for cents, want := range map[int]string{
		5:         "$0.05",
		123450:    "$1,234.50",
		100000000: "$1,000,000.00",
	} {
		if got := Money(cents); got != want {
			t.Errorf("Money(%d) = %q, want %q", cents, got, want)
		}
	}
}

func TestCount(t *testing.T) {
	for n, want := range map[int]string{
		999:      "999",
		1004:     "1,004",
		12000070: "12,000,070",
	} {
		if got := Count(n); got != want {
			t.Errorf("Count(%d) = %q, want %q", n, got, want)
		}
	}
}

func TestDates(t *testing.T) {
	ts := time.Date(2026, time.March, 4, 4, 5, 0, 0, time.UTC)
	if got := ShortDate(ts); got != "Mar 4, 2026" {
		t.Errorf("ShortDate = %q", got)
	}
	if got := ShortDateTime(ts); got != "Mar 4, 2026 04:05" {
		t.Errorf("ShortDateTime = %q", got)
	}
	if got := ISODateTime(ts); got != "2026-03-04T04:05Z" {
		t.Errorf("ISODateTime = %q", got)
	}
}
