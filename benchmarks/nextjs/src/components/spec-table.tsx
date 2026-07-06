import type { Spec } from "../lib/catalog";

export function SpecTable({ specs }: { specs: Spec[] }) {
  return (
    <div className="mt-6 overflow-hidden rounded-xl border border-slate-200 bg-white">
      <table className="w-full text-sm">
        <tbody>
          {specs.map((spec) => (
            <tr key={spec.key} className="border-b border-slate-100 last:border-0">
              <th scope="row" className="w-1/3 px-4 py-3 text-left font-medium text-slate-500">
                {spec.key}
              </th>
              <td className="px-4 py-3 text-slate-900">{spec.value}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
