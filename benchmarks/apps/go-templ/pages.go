package main

import (
	"errors"
	"net/http"
	"net/url"
	"strconv"

	"github.com/a-h/templ"

	"harbor-go-templ/internal/catalog"
	"harbor-go-templ/internal/views"
)

// render writes a page with templ's handler, which renders into a pooled buffer before sending
// the response.
func render(w http.ResponseWriter, r *http.Request, status int, page templ.Component) {
	templ.Handler(page, templ.WithStatus(status)).ServeHTTP(w, r)
}

func (a *app) home(w http.ResponseWriter, r *http.Request) {
	render(w, r, http.StatusOK, views.HomePage(a.store, cartCount(r)))
}

func (a *app) about(w http.ResponseWriter, r *http.Request) {
	render(w, r, http.StatusOK, views.AboutPage(cartCount(r)))
}

func (a *app) listing(w http.ResponseWriter, r *http.Request) {
	params, err := listingParams(r)
	if err != nil {
		a.errorPage(w, r, http.StatusUnprocessableEntity)
		return
	}
	listing, err := a.store.Listing(params)
	if err != nil {
		a.errorPage(w, r, listingErrorStatus(err))
		return
	}
	render(w, r, http.StatusOK, views.ListingPage(a.store, &listing, cartCount(r)))
}

func (a *app) detail(w http.ResponseWriter, r *http.Request) {
	product := a.product(r)
	if product == nil {
		a.notFound(w, r)
		return
	}
	render(w, r, http.StatusOK, views.DetailPage(product, cartCount(r)))
}

func (a *app) orders(w http.ResponseWriter, r *http.Request) {
	render(w, r, http.StatusOK, views.OrdersPage(a.store, cartCount(r)))
}

func (a *app) notFound(w http.ResponseWriter, r *http.Request) {
	a.errorPage(w, r, http.StatusNotFound)
}

func (a *app) errorPage(w http.ResponseWriter, r *http.Request, status int) {
	render(w, r, status, views.ErrorPage(status, cartCount(r)))
}

// product looks up the product named by the {id} path segment, returning nil if there is none.
func (a *app) product(r *http.Request) *catalog.Product {
	id, err := strconv.Atoi(r.PathValue("id"))
	if err != nil {
		return nil
	}
	return a.store.Product(id)
}

// listingParams reads the listing query parameters. A malformed query string is an error.
func listingParams(r *http.Request) (catalog.ListingParams, error) {
	query, err := url.ParseQuery(r.URL.RawQuery)
	if err != nil {
		return catalog.ListingParams{}, err
	}
	return catalog.ListingParams{
		Category: query.Get("category"),
		Sort:     query.Get("sort"),
		Page:     query.Get("page"),
	}, nil
}

// listingErrorStatus is the HTTP status of a listing error.
func listingErrorStatus(err error) int {
	if errors.Is(err, catalog.ErrCategoryNotFound) || errors.Is(err, catalog.ErrPageNotFound) {
		return http.StatusNotFound
	}
	return http.StatusUnprocessableEntity
}
