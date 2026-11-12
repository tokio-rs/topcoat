import { getStore, maxQuantity } from "@/lib/store";

const invalid = (error: string) => Response.json({ error }, { status: 422 });

/** `POST /api/cart`: validates a cart line and prices it. Nothing is stored. */
export async function POST(request: Request) {
  const contentType = request.headers.get("Content-Type") ?? "";
  const isJson = contentType.split(";")[0].trim().toLowerCase() === "application/json";
  let body: unknown = null;
  try {
    if (isJson) body = await request.json();
  } catch {
    // Malformed JSON is an invalid body like any other.
  }
  if (
    typeof body !== "object" ||
    body === null ||
    !("product_id" in body && Number.isInteger(body.product_id)) ||
    !("quantity" in body && Number.isInteger(body.quantity))
  ) {
    return invalid("invalid request body");
  }
  const { product_id: productId, quantity } = body as { product_id: number; quantity: number };

  const product = getStore().product(productId);
  if (!product) return invalid("product not found");
  if (product.stock === 0) return invalid("product is sold out");
  const max = maxQuantity(product);
  if (quantity < 1 || quantity > max) return invalid(`quantity must be between 1 and ${max}`);

  return Response.json({
    product_id: product.id,
    name: product.name,
    quantity,
    unit_price_cents: product.price_cents,
    line_total_cents: product.price_cents * quantity,
  });
}
