export function Breadcrumbs({
  category,
  categorySlug,
  name,
}: {
  category: string;
  categorySlug: string;
  name: string;
}) {
  return (
    <nav aria-label="Breadcrumb" className="text-sm text-slate-500">
      <ol className="flex flex-wrap items-center gap-2">
        <li>
          <a href="/" className="hover:text-slate-900">
            Home
          </a>
        </li>
        <li>/</li>
        <li>
          <a href="/products" className="hover:text-slate-900">
            Products
          </a>
        </li>
        <li>/</li>
        <li>
          <a href={`/products?category=${categorySlug}`} className="hover:text-slate-900">
            {category}
          </a>
        </li>
        <li>/</li>
        <li className="font-medium text-slate-900">{name}</li>
      </ol>
    </nav>
  );
}
