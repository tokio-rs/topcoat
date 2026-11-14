package catalog

import (
	"errors"
	"strconv"
)

// PageSize is the number of products on one listing page.
const PageSize = 24

// Sort is the order of products in a listing.
type Sort int

const (
	Newest Sort = iota
	PriceAsc
	PriceDesc
	TopRated
)

// Sorts lists every sort order, in the order of the sort links.
var Sorts = [...]Sort{Newest, PriceAsc, PriceDesc, TopRated}

// ParseSort parses the value of the sort query parameter.
func ParseSort(value string) (Sort, bool) {
	for _, sort := range Sorts {
		if sort.Param() == value {
			return sort, true
		}
	}
	return Newest, false
}

// Param is the value of the sort query parameter.
func (s Sort) Param() string {
	switch s {
	case PriceAsc:
		return "price-asc"
	case PriceDesc:
		return "price-desc"
	case TopRated:
		return "rating"
	}
	return "newest"
}

// Label is the text of the sort link.
func (s Sort) Label() string {
	switch s {
	case PriceAsc:
		return "Price: low to high"
	case PriceDesc:
		return "Price: high to low"
	case TopRated:
		return "Top rated"
	}
	return "Newest"
}

// compare orders two products. Ties are broken by ascending id.
func (s Sort) compare(a, b *Product) int {
	var primary int
	switch s {
	case Newest:
		primary = b.CreatedAt.Time().Compare(a.CreatedAt.Time())
	case PriceAsc:
		primary = a.PriceCents - b.PriceCents
	case PriceDesc:
		primary = b.PriceCents - a.PriceCents
	case TopRated:
		primary = b.RatingTenths() - a.RatingTenths()
		if primary == 0 {
			primary = b.ReviewCount - a.ReviewCount
		}
	}
	if primary != 0 {
		return primary
	}
	return a.ID - b.ID
}

// Listing errors, in the order the parameters are validated. The messages are the API's error
// messages.
var (
	ErrCategoryNotFound = errors.New("category not found")
	ErrInvalidSort      = errors.New("invalid sort")
	ErrInvalidPage      = errors.New("invalid page")
	ErrPageNotFound     = errors.New("page not found")
)

// ListingParams are the raw query parameters of a product listing. Empty values count as absent.
type ListingParams struct {
	Category string
	Sort     string
	Page     string
}

// Listing is one page of a product listing.
type Listing struct {
	// Category is the selected category, or nil for all products.
	Category  *Category
	Sort      Sort
	Page      int
	PageCount int
	Total     int
	// Products are the products on this page.
	Products []*Product
}

// Listing resolves listing parameters against the store. Parameters are validated in order:
// category, then sort, then page.
func (s *Store) Listing(params ListingParams) (Listing, error) {
	var category *Category
	if params.Category != "" {
		category = s.categoryBySlug[params.Category]
		if category == nil {
			return Listing{}, ErrCategoryNotFound
		}
	}
	sort := Newest
	if params.Sort != "" {
		var ok bool
		if sort, ok = ParseSort(params.Sort); !ok {
			return Listing{}, ErrInvalidSort
		}
	}
	sorted := s.sortedAll[sort]
	if category != nil {
		sorted = category.sorted[sort]
	}
	total := len(sorted)
	pageCount := max(1, (total+PageSize-1)/PageSize)
	page := 1
	if params.Page != "" {
		var err error
		if page, err = parsePage(params.Page); err != nil {
			return Listing{}, err
		}
	}
	if page > pageCount {
		return Listing{}, ErrPageNotFound
	}
	start := (page - 1) * PageSize
	end := min(start+PageSize, total)
	return Listing{
		Category:  category,
		Sort:      sort,
		Page:      page,
		PageCount: pageCount,
		Total:     total,
		Products:  sorted[start:end],
	}, nil
}

// CategorySlug is the slug of the selected category, or "" for all products.
func (l *Listing) CategorySlug() string {
	if l.Category == nil {
		return ""
	}
	return l.Category.Slug
}

// FirstPosition is the 1-based position of the first product on this page, or 0 if the listing
// is empty.
func (l *Listing) FirstPosition() int {
	if l.Total == 0 {
		return 0
	}
	return (l.Page-1)*PageSize + 1
}

// LastPosition is the 1-based position of the last product on this page.
func (l *Listing) LastPosition() int {
	return (l.Page-1)*PageSize + len(l.Products)
}

// parsePage parses a page number made of ASCII digits. Numbers too large to represent are past
// any last page.
func parsePage(value string) (int, error) {
	for i := 0; i < len(value); i++ {
		if value[i] < '0' || value[i] > '9' {
			return 0, ErrInvalidPage
		}
	}
	page, err := strconv.Atoi(value)
	switch {
	case err != nil:
		return 0, ErrPageNotFound
	case page == 0:
		return 0, ErrInvalidPage
	}
	return page, nil
}
