import { Controller } from "@hotwired/stimulus"

const CART_MAX = 999999999

// The quantity stepper and add-to-cart button of a product page. Adding writes the `cart_count`
// cookie and dispatches `product-form:added`, which the header badge listens for.
export default class extends Controller {
  static targets = ["decrement", "increment", "quantity", "status"]
  static values = { max: Number, quantity: { type: Number, default: 1 } }

  decrement() {
    this.quantityValue = Math.max(1, this.quantityValue - 1)
  }

  increment() {
    this.quantityValue = Math.min(this.maxValue, this.quantityValue + 1)
  }

  quantityValueChanged(quantity) {
    this.quantityTarget.textContent = String(quantity)
    this.decrementTarget.disabled = quantity <= 1
    this.incrementTarget.disabled = quantity >= this.maxValue
  }

  add() {
    const quantity = this.quantityValue
    const count = Math.min(CART_MAX, readCartCount() + quantity)
    document.cookie = `cart_count=${count}; Path=/; Max-Age=2592000; SameSite=Lax`
    this.statusTarget.textContent = `Added ${quantity} to your cart.`
    this.dispatch("added", { detail: { count } })
  }
}

// The same rule as the server: 1 to 9 digits, anything else is an empty cart.
function readCartCount() {
  const match = document.cookie.match(/(?:^|;\s*)cart_count=([^;]*)/)
  return match && /^[0-9]{1,9}$/.test(match[1]) ? Number(match[1]) : 0
}
