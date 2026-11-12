package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log"
	"mime"
	"net/http"

	"harbor-go-templ/internal/catalog"
)

// productPage is a page of products in the listing API.
type productPage struct {
	Items     []*catalog.ProductSummary `json:"items"`
	Page      int                       `json:"page"`
	PageCount int                       `json:"page_count"`
	Total     int                       `json:"total"`
}

// cartRequest is the body of an add-to-cart request. Pointers tell missing and null fields apart
// from zero.
type cartRequest struct {
	ProductID *int `json:"product_id"`
	Quantity  *int `json:"quantity"`
}

// cartLine is a priced cart line.
type cartLine struct {
	ProductID      int    `json:"product_id"`
	Name           string `json:"name"`
	Quantity       int    `json:"quantity"`
	UnitPriceCents int    `json:"unit_price_cents"`
	LineTotalCents int    `json:"line_total_cents"`
}

// apiListProducts is GET /api/products: the same query as the listing page, as JSON.
func (a *app) apiListProducts(w http.ResponseWriter, r *http.Request) {
	params, err := listingParams(r)
	if err != nil {
		writeError(w, http.StatusUnprocessableEntity, "invalid query")
		return
	}
	listing, err := a.store.Listing(params)
	if err != nil {
		writeError(w, listingErrorStatus(err), err.Error())
		return
	}
	items := make([]*catalog.ProductSummary, len(listing.Products))
	for i, product := range listing.Products {
		items[i] = &product.ProductSummary
	}
	writeJSON(w, http.StatusOK, productPage{
		Items:     items,
		Page:      listing.Page,
		PageCount: listing.PageCount,
		Total:     listing.Total,
	})
}

// apiProduct is GET /api/products/{id}: one product with all its fields.
func (a *app) apiProduct(w http.ResponseWriter, r *http.Request) {
	product := a.product(r)
	if product == nil {
		writeError(w, http.StatusNotFound, "product not found")
		return
	}
	writeJSON(w, http.StatusOK, product)
}

// apiAddToCart is POST /api/cart: validates a cart line and prices it. Nothing is stored.
func (a *app) apiAddToCart(w http.ResponseWriter, r *http.Request) {
	var request cartRequest
	if err := decodeJSON(w, r, &request); err != nil || request.ProductID == nil || request.Quantity == nil {
		writeError(w, http.StatusUnprocessableEntity, "invalid request body")
		return
	}
	product := a.store.Product(*request.ProductID)
	if product == nil {
		writeError(w, http.StatusUnprocessableEntity, "product not found")
		return
	}
	if product.SoldOut() {
		writeError(w, http.StatusUnprocessableEntity, "product is sold out")
		return
	}
	quantity, maxQuantity := *request.Quantity, product.MaxQuantity()
	if quantity < 1 || quantity > maxQuantity {
		writeError(w, http.StatusUnprocessableEntity, fmt.Sprintf("quantity must be between 1 and %d", maxQuantity))
		return
	}
	writeJSON(w, http.StatusOK, cartLine{
		ProductID:      product.ID,
		Name:           product.Name,
		Quantity:       quantity,
		UnitPriceCents: product.PriceCents,
		LineTotalCents: product.PriceCents * quantity,
	})
}

// decodeJSON decodes a request body that must be a single JSON value with an application/json
// Content-Type.
func decodeJSON(w http.ResponseWriter, r *http.Request, v any) error {
	mediaType, _, err := mime.ParseMediaType(r.Header.Get("Content-Type"))
	if err != nil || mediaType != "application/json" {
		return errors.New("expected application/json")
	}
	decoder := json.NewDecoder(http.MaxBytesReader(w, r.Body, 1<<20))
	if err := decoder.Decode(v); err != nil {
		return err
	}
	if decoder.Decode(&struct{}{}) != io.EOF {
		return errors.New("body must contain a single JSON value")
	}
	return nil
}

func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	if err := json.NewEncoder(w).Encode(v); err != nil {
		log.Printf("writing JSON response: %v", err)
	}
}

func writeError(w http.ResponseWriter, status int, message string) {
	writeJSON(w, status, map[string]string{"error": message})
}
