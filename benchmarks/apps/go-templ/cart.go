package main

import (
	"net/http"
	"strconv"
)

// cartCookie is the name of the cookie that holds the number of items in the cart.
const cartCookie = "cart_count"

// cartCount reads the number of items in the cart from the cart_count cookie. A missing or
// malformed cookie, anything but 1 to 9 ASCII digits, counts as an empty cart.
func cartCount(r *http.Request) int {
	cookie, err := r.Cookie(cartCookie)
	if err != nil || len(cookie.Value) < 1 || len(cookie.Value) > 9 {
		return 0
	}
	for _, c := range []byte(cookie.Value) {
		if c < '0' || c > '9' {
			return 0
		}
	}
	count, _ := strconv.Atoi(cookie.Value)
	return count
}
