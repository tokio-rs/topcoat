export function SiteNav() {
  return (
    <header className="border-b border-slate-200 bg-white">
      <nav className="mx-auto flex w-full max-w-6xl items-center justify-between px-4 py-4">
        <a href="/" className="text-lg font-bold tracking-tight">
          Meridian Supply
        </a>
        <div className="flex items-center gap-6 text-sm font-medium text-slate-600">
          <a href="/" className="hover:text-slate-900">
            Home
          </a>
          <a href="/products" className="hover:text-slate-900">
            Products
          </a>
          <span className="rounded-full bg-indigo-600 px-3 py-1 text-xs font-semibold text-white">
            Cart (3)
          </span>
        </div>
      </nav>
    </header>
  );
}
