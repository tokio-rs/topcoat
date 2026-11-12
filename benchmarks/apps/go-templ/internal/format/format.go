// Package format formats values for the storefront: money, counts, ratings and dates.
package format

import (
	"strconv"
	"time"
)

// Money formats an amount of cents like "$1,234.50".
func Money(cents int) string {
	b := make([]byte, 0, 16)
	b = append(b, '$')
	b = appendCount(b, cents/100)
	rest := cents % 100
	b = append(b, '.', byte('0'+rest/10), byte('0'+rest%10))
	return string(b)
}

// Count formats an integer with thousands separators, like "1,204".
func Count(n int) string {
	if n < 1000 {
		return strconv.Itoa(n)
	}
	return string(appendCount(make([]byte, 0, 16), n))
}

func appendCount(b []byte, n int) []byte {
	if n < 1000 {
		return strconv.AppendInt(b, int64(n), 10)
	}
	b = appendCount(b, n/1000)
	group := n % 1000
	return append(b, ',', byte('0'+group/100), byte('0'+group/10%10), byte('0'+group%10))
}

// Rating formats a rating with one decimal, like "4.0".
func Rating(rating float64) string {
	return strconv.FormatFloat(rating, 'f', 1, 64)
}

// ShortDate formats a date like "Mar 4, 2026".
func ShortDate(t time.Time) string {
	return t.Format("Jan 2, 2006")
}

// ShortDateTime formats a UTC timestamp like "Mar 4, 2026 14:05".
func ShortDateTime(t time.Time) string {
	return t.Format("Jan 2, 2006 15:04")
}

// ISODate formats a date like "2026-03-04", for datetime attributes.
func ISODate(t time.Time) string {
	return t.Format(time.DateOnly)
}

// ISODateTime formats a UTC timestamp like "2026-03-04T14:05Z", for datetime attributes.
func ISODateTime(t time.Time) string {
	return t.Format("2006-01-02T15:04Z")
}
