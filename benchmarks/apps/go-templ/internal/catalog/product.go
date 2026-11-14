package catalog

import (
	"fmt"
	"math"
	"strings"
	"time"
)

// Product is a product from products.json.
//
// The summary fields are embedded so the listing API can encode them on their own; encoding/json
// treats the fields of an embedded struct as fields of the outer one.
type Product struct {
	ProductSummary
	Description string   `json:"description"`
	Specs       []Spec   `json:"specs"`
	Related     []int    `json:"related"`
	Reviews     []Review `json:"reviews"`

	// Category is resolved from CategorySlug at startup.
	Category *Category `json:"-"`
	// RelatedProducts are resolved from Related at startup, in data order.
	RelatedProducts []*Product `json:"-"`
}

// ProductSummary holds the product fields included in list responses.
type ProductSummary struct {
	ID             int     `json:"id"`
	Slug           string  `json:"slug"`
	Name           string  `json:"name"`
	CategorySlug   string  `json:"category"`
	PriceCents     int     `json:"price_cents"`
	CompareAtCents *int    `json:"compare_at_cents"`
	Rating         float64 `json:"rating"`
	ReviewCount    int     `json:"review_count"`
	Stock          int     `json:"stock"`
	Color          Color   `json:"color"`
	Tagline        string  `json:"tagline"`
	CreatedAt      Date    `json:"created_at"`
	Featured       bool    `json:"featured"`
}

// Spec is one row of a product's specification table.
type Spec struct {
	Label string `json:"label"`
	Value string `json:"value"`
}

// Review is a customer review of a product.
type Review struct {
	Author string `json:"author"`
	Rating int    `json:"rating"`
	Title  string `json:"title"`
	Body   string `json:"body"`
	Date   Date   `json:"date"`
}

// MaxQuantity is the most a customer can add to the cart at once.
func (p *Product) MaxQuantity() int {
	return min(p.Stock, 10)
}

// SoldOut reports whether the product is out of stock.
func (p *Product) SoldOut() bool {
	return p.Stock == 0
}

// RatingTenths is the rating in tenths of a star, for example 43 for a 4.3 rating.
func (p *Product) RatingTenths() int {
	return int(math.Round(p.Rating * 10))
}

// Paragraphs splits the description into its paragraphs.
func (p *Product) Paragraphs() []string {
	return strings.Split(p.Description, "\n\n")
}

// Color is the color swatch of a product.
type Color string

const (
	Charcoal   Color = "charcoal"
	Sand       Color = "sand"
	Sage       Color = "sage"
	Navy       Color = "navy"
	Terracotta Color = "terracotta"
	Ochre      Color = "ochre"
	Plum       Color = "plum"
	Sky        Color = "sky"
)

// Label is the display name of the color.
func (c Color) Label() string {
	switch c {
	case Charcoal:
		return "Charcoal"
	case Sand:
		return "Sand"
	case Sage:
		return "Sage"
	case Navy:
		return "Navy"
	case Terracotta:
		return "Terracotta"
	case Ochre:
		return "Ochre"
	case Plum:
		return "Plum"
	case Sky:
		return "Sky"
	}
	return ""
}

// Date is a calendar date, encoded in JSON as "2006-01-02".
type Date time.Time

// Time returns the date as midnight UTC.
func (d Date) Time() time.Time {
	return time.Time(d)
}

// MarshalText implements encoding.TextMarshaler.
func (d Date) MarshalText() ([]byte, error) {
	return time.Time(d).AppendFormat(make([]byte, 0, len(time.DateOnly)), time.DateOnly), nil
}

// UnmarshalText implements encoding.TextUnmarshaler.
func (d *Date) UnmarshalText(text []byte) error {
	t, err := time.Parse(time.DateOnly, string(text))
	if err != nil {
		return fmt.Errorf("invalid date %q: %w", text, err)
	}
	*d = Date(t)
	return nil
}
