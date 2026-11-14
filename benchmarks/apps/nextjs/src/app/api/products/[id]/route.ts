import { getStore, productJson } from "@/lib/store";

/** `GET /api/products/{id}`: one product with all its fields. */
export async function GET(_request: Request, { params }: RouteContext<"/api/products/[id]">) {
  const { id } = await params;
  const product = /^[0-9]+$/.test(id) ? getStore().product(Number(id)) : undefined;
  if (!product) {
    return Response.json({ error: "product not found" }, { status: 404 });
  }
  return Response.json(productJson(product));
}
