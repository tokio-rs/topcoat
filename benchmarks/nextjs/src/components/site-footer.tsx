const COLUMNS: [string, [string, string][]][] = [
  [
    "Shop",
    [
      ["All products", "/products"],
      ["Audio", "/products?category=audio"],
      ["Displays", "/products?category=displays"],
      ["Wearables", "/products?category=wearables"],
    ],
  ],
  [
    "Support",
    [
      ["Contact", "#"],
      ["Shipping", "#"],
      ["Returns", "#"],
      ["Warranty", "#"],
    ],
  ],
  [
    "Company",
    [
      ["About", "#"],
      ["Careers", "#"],
      ["Press", "#"],
      ["Sustainability", "#"],
    ],
  ],
  [
    "Legal",
    [
      ["Privacy", "#"],
      ["Terms", "#"],
      ["Imprint", "#"],
      ["Cookie settings", "#"],
    ],
  ],
];

export function SiteFooter() {
  return (
    <footer className="border-t border-slate-200 bg-white">
      <div className="mx-auto grid w-full max-w-6xl grid-cols-2 gap-8 px-4 py-10 text-sm md:grid-cols-4">
        {COLUMNS.map(([title, links]) => (
          <div key={title}>
            <h3 className="mb-3 font-semibold text-slate-900">{title}</h3>
            <ul className="space-y-2 text-slate-500">
              {links.map(([label, href]) => (
                <li key={label}>
                  <a href={href} className="hover:text-slate-900">
                    {label}
                  </a>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
      <div className="border-t border-slate-100">
        <p className="mx-auto w-full max-w-6xl px-4 py-4 text-xs text-slate-400">
          (c) 2026 Meridian Supply. All rights reserved.
        </p>
      </div>
    </footer>
  );
}
