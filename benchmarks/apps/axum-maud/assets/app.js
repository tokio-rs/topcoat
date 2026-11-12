// Harbor Market client behavior: the mobile menu toggle and the product quantity stepper.

const CART_COOKIE = "cart_count";
const CART_MAX = 999999999;

function readCartCount() {
  const match = document.cookie.match(/(?:^|;\s*)cart_count=([^;]*)/);
  return match && /^[0-9]{1,9}$/.test(match[1]) ? Number(match[1]) : 0;
}

function badgeText(count) {
  return count > 99 ? "99+" : String(count);
}

for (const toggle of document.querySelectorAll("[data-menu-toggle]")) {
  const menu = document.getElementById(toggle.getAttribute("aria-controls"));
  toggle.addEventListener("click", () => {
    const open = toggle.getAttribute("aria-expanded") !== "true";
    toggle.setAttribute("aria-expanded", String(open));
    menu.classList.toggle("hidden", !open);
  });
}

for (const form of document.querySelectorAll("[data-product-form]")) {
  const max = Number(form.dataset.max);
  const output = form.querySelector("[data-quantity]");
  const decrement = form.querySelector('[data-step="-1"]');
  const increment = form.querySelector('[data-step="1"]');
  const status = form.querySelector("[data-cart-status]");
  let quantity = 1;

  const render = () => {
    output.textContent = String(quantity);
    decrement.disabled = quantity <= 1;
    increment.disabled = quantity >= max;
  };

  for (const button of [decrement, increment]) {
    button.addEventListener("click", () => {
      quantity = Math.min(max, Math.max(1, quantity + Number(button.dataset.step)));
      render();
    });
  }

  form.querySelector("[data-add-to-cart]").addEventListener("click", () => {
    const count = Math.min(CART_MAX, readCartCount() + quantity);
    document.cookie = `${CART_COOKIE}=${count}; Path=/; Max-Age=2592000; SameSite=Lax`;
    for (const badge of document.querySelectorAll("[data-cart-count]")) {
      badge.textContent = badgeText(count);
    }
    status.textContent = `Added ${quantity} to your cart.`;
  });
}
