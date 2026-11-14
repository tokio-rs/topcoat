package main

import (
	"net/http"

	"harbor-go-templ/assets"
	"harbor-go-templ/internal/catalog"
)

// app holds what the handlers share: the dataset.
type app struct {
	store *catalog.Store
}

func routes(a *app) *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /health", health)
	mux.HandleFunc("GET /{$}", a.home)
	mux.HandleFunc("GET /about", a.about)
	mux.HandleFunc("GET /products", a.listing)
	mux.HandleFunc("GET /products/{id}", a.detail)
	mux.HandleFunc("GET /admin/orders", a.orders)
	mux.HandleFunc("GET /api/products", a.apiListProducts)
	mux.HandleFunc("GET /api/products/{id}", a.apiProduct)
	mux.HandleFunc("POST /api/cart", a.apiAddToCart)
	assets.Register(mux)
	mux.HandleFunc("/", a.notFound)
	return mux
}

func health(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/plain; charset=utf-8")
	w.Write([]byte("ok"))
}
