import { Controller } from "@hotwired/stimulus"

// The cart count in the header, updated when a product form adds to the cart.
export default class extends Controller {
  update({ detail: { count } }) {
    this.element.textContent = count > 99 ? "99+" : String(count)
  }
}
