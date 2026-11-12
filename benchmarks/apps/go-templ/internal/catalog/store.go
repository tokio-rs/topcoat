// Package catalog holds the Harbor Market dataset and the indexes built over it at startup.
package catalog

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"slices"
)

// Category is a product category from categories.json.
type Category struct {
	Slug        string `json:"slug"`
	Name        string `json:"name"`
	Description string `json:"description"`

	// ProductCount is the number of products in the category, counted at startup.
	ProductCount int `json:"-"`
	sorted       [len(Sorts)][]*Product
}

// Store is the in-memory dataset with the indexes built at startup.
//
// Every lookup is a constant-time index or a slice, standing in for a database that already did
// the filtering, sorting and aggregation.
type Store struct {
	// Categories are in the order of categories.json.
	Categories []*Category
	// Featured are the featured products, by ascending id.
	Featured []*Product
	// Orders are all orders, newest first.
	Orders       []Order
	OrderSummary OrderSummary

	categoryBySlug map[string]*Category
	products       []Product
	sortedAll      [len(Sorts)][]*Product
}

// Load reads the JSON dataset from dir and builds the indexes.
func Load(dir string) (*Store, error) {
	var s Store
	if err := readJSON(filepath.Join(dir, "categories.json"), &s.Categories); err != nil {
		return nil, err
	}
	if err := readJSON(filepath.Join(dir, "products.json"), &s.products); err != nil {
		return nil, err
	}
	if err := readJSON(filepath.Join(dir, "orders.json"), &s.Orders); err != nil {
		return nil, err
	}

	s.categoryBySlug = make(map[string]*Category, len(s.Categories))
	for _, category := range s.Categories {
		s.categoryBySlug[category.Slug] = category
	}

	slices.SortFunc(s.products, func(a, b Product) int { return a.ID - b.ID })
	all := make([]*Product, len(s.products))
	for i := range s.products {
		product := &s.products[i]
		if product.ID != i+1 {
			return nil, fmt.Errorf("product ids must be 1..%d", i+1)
		}
		if product.Color.Label() == "" {
			return nil, fmt.Errorf("product %d has unknown color %q", product.ID, product.Color)
		}
		product.Category = s.categoryBySlug[product.CategorySlug]
		if product.Category == nil {
			return nil, fmt.Errorf("product %d has unknown category %q", product.ID, product.CategorySlug)
		}
		product.Category.ProductCount++
		if product.Featured {
			s.Featured = append(s.Featured, product)
		}
		all[i] = product
	}
	for _, product := range all {
		for _, id := range product.Related {
			related := s.Product(id)
			if related == nil {
				return nil, fmt.Errorf("product %d has unknown related id %d", product.ID, id)
			}
			product.RelatedProducts = append(product.RelatedProducts, related)
		}
	}

	s.sortedAll = sortAll(all)
	for _, category := range s.Categories {
		products := slices.DeleteFunc(slices.Clone(all), func(p *Product) bool {
			return p.Category != category
		})
		category.sorted = sortAll(products)
	}

	for i := range s.Orders {
		s.Orders[i].PlacedAt = s.Orders[i].PlacedAt.UTC()
	}
	slices.SortFunc(s.Orders, func(a, b Order) int {
		if c := b.PlacedAt.Compare(a.PlacedAt); c != 0 {
			return c
		}
		return b.ID - a.ID
	})
	s.OrderSummary = summarize(s.Orders)
	return &s, nil
}

// Product looks up a product by id, returning nil if there is none.
func (s *Store) Product(id int) *Product {
	if id < 1 || id > len(s.products) {
		return nil
	}
	return &s.products[id-1]
}

// ProductCount is the number of products in the catalog.
func (s *Store) ProductCount() int {
	return len(s.products)
}

// sortAll sorts products in every sort order.
func sortAll(products []*Product) [len(Sorts)][]*Product {
	var sorted [len(Sorts)][]*Product
	for _, sort := range Sorts {
		sorted[sort] = slices.SortedFunc(slices.Values(products), sort.compare)
	}
	return sorted
}

func readJSON(path string, v any) error {
	data, err := os.ReadFile(path)
	if err != nil {
		return err
	}
	if err := json.Unmarshal(data, v); err != nil {
		return fmt.Errorf("%s: %w", path, err)
	}
	return nil
}
