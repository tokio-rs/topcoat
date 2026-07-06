import { ProductCard } from "../components/product-card";
import { getFeatured } from "../lib/catalog";

export const dynamic = "force-dynamic";

export default async function HomePage() {
  const featured = getFeatured();

  return (
    <>
      <section className="rounded-2xl bg-indigo-600 px-8 py-16 text-white">
        <h1 className="max-w-2xl text-4xl font-bold tracking-tight">
          Gear that earns its place on your desk
        </h1>
        <p className="mt-4 max-w-xl text-lg text-indigo-100">
          Five hundred products, zero filler. Everything in the catalog is tested daily by the
          people who build it.
        </p>
        <a
          href="/products"
          className="mt-8 inline-block rounded-lg bg-white px-6 py-3 text-sm font-semibold text-indigo-700 hover:bg-indigo-50"
        >
          Browse all products
        </a>
      </section>
      <section className="mt-12">
        <div className="flex items-baseline justify-between">
          <h2 className="text-2xl font-bold tracking-tight">Featured products</h2>
          <a href="/products" className="text-sm font-medium text-indigo-600 hover:text-indigo-500">
            View all
          </a>
        </div>
        <div className="mt-6 grid grid-cols-2 gap-6 md:grid-cols-3 lg:grid-cols-4">
          {featured.map((product) => (
            <ProductCard key={product.id} product={product} />
          ))}
        </div>
      </section>
    </>
  );
}
