import type { Metadata } from "next";
import { cookies } from "next/headers";

import { CartProvider } from "@/components/cart";
import { Footer } from "@/components/footer";
import { Header } from "@/components/header";
import { CART_COOKIE, parseCartCount } from "@/lib/cart";
import "./globals.css";

export const metadata: Metadata = {
  title: { default: "Harbor Market", template: "%s | Harbor Market" },
  icons: { icon: "data:," },
};

export default async function RootLayout({ children }: LayoutProps<"/">) {
  const cartCount = parseCartCount((await cookies()).get(CART_COOKIE)?.value);
  return (
    <html lang="en">
      <body className="flex min-h-screen flex-col bg-stone-50 text-stone-900 antialiased">
        <CartProvider initialCount={cartCount}>
          <a
            href="#main"
            className="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-50 focus:rounded-md focus:bg-white focus:px-4 focus:py-2 focus:text-sm focus:font-medium focus:shadow"
          >
            Skip to content
          </a>
          <Header />
          <main id="main" className="flex-1">
            {children}
          </main>
          <Footer />
        </CartProvider>
      </body>
    </html>
  );
}
