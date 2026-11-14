import { Controller } from "@hotwired/stimulus"

// Shows and hides the main navigation on narrow screens.
export default class extends Controller {
  static targets = ["button", "nav"]

  toggle() {
    const open = this.buttonTarget.getAttribute("aria-expanded") !== "true"
    this.buttonTarget.setAttribute("aria-expanded", String(open))
    this.navTarget.classList.toggle("hidden", !open)
  }
}
