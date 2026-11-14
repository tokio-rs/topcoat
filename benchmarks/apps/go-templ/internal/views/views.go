// Package views holds the templ components: shared views and one page per file.
//
// Class strings selected by data live in this file as literals so Tailwind sees them.
package views

import (
	"strconv"
	"strings"

	"harbor-go-templ/internal/catalog"
)

// link is a navigation link.
type link struct {
	Href  string
	Label string
}

// palette returns the Tailwind classes of a product color: the background of the image tile, and
// the fill of the product shape and the color swatch.
func palette(color catalog.Color) (tile, accent string) {
	switch color {
	case catalog.Charcoal:
		return "bg-stone-200", "bg-stone-700"
	case catalog.Sand:
		return "bg-amber-100", "bg-amber-300"
	case catalog.Sage:
		return "bg-green-100", "bg-green-700/70"
	case catalog.Navy:
		return "bg-blue-100", "bg-blue-900"
	case catalog.Terracotta:
		return "bg-orange-100", "bg-orange-700"
	case catalog.Ochre:
		return "bg-yellow-100", "bg-yellow-600"
	case catalog.Plum:
		return "bg-purple-100", "bg-purple-800"
	case catalog.Sky:
		return "bg-sky-100", "bg-sky-500"
	}
	return "", ""
}

func accent(color catalog.Color) string {
	_, accent := palette(color)
	return accent
}

func tile(color catalog.Color) string {
	tile, _ := palette(color)
	return tile
}

// statusColors returns the Tailwind classes of an order status badge.
func statusColors(status catalog.OrderStatus) string {
	switch status {
	case catalog.Pending:
		return "bg-amber-50 text-amber-800 ring-amber-600/20"
	case catalog.Paid:
		return "bg-sky-50 text-sky-800 ring-sky-600/20"
	case catalog.Shipped:
		return "bg-indigo-50 text-indigo-700 ring-indigo-600/20"
	case catalog.Delivered:
		return "bg-emerald-50 text-emerald-800 ring-emerald-600/20"
	case catalog.Refunded:
		return "bg-stone-100 text-stone-700 ring-stone-500/20"
	case catalog.Cancelled:
		return "bg-rose-50 text-rose-700 ring-rose-600/20"
	}
	return ""
}

// badgeText is the cart badge text, which caps at 99+.
func badgeText(cart int) string {
	if cart > 99 {
		return "99+"
	}
	return strconv.Itoa(cart)
}

// documentTitle is the page title followed by the store name, or just the store name.
func documentTitle(title string) string {
	if title == "" {
		return "Harbor Market"
	}
	return title + " | Harbor Market"
}

// productURL is the URL of a product detail page.
func productURL(id int) string {
	return "/products/" + strconv.Itoa(id)
}

// categoryURL is the URL of the first page of a category in the default order.
func categoryURL(slug string) string {
	return listingURL(slug, catalog.Newest, 1)
}

// listingURL is the URL of a listing page. Parameters with default values are left out.
func listingURL(category string, sort catalog.Sort, page int) string {
	var b strings.Builder
	b.WriteString("/products")
	separator := "?"
	if category != "" {
		b.WriteString(separator + "category=" + category)
		separator = "&"
	}
	if sort != catalog.Newest {
		b.WriteString(separator + "sort=" + sort.Param())
		separator = "&"
	}
	if page != 1 {
		b.WriteString(separator + "page=" + strconv.Itoa(page))
	}
	return b.String()
}

// filledStars is the number of filled stars for a rating in tenths: the rating rounded half up.
func filledStars(ratingTenths int) int {
	return (ratingTenths + 5) / 10
}
