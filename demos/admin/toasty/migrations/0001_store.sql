CREATE TABLE "orders" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "customer_id" INTEGER NOT NULL,
    "status" TEXT NOT NULL CHECK ("status" IN ('pending', 'paid', 'shipped', 'delivered', 'cancelled', 'refunded')),
    "total" BIGINT NOT NULL,
    "placed_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE INDEX "index_orders_by_status" ON "orders" ("status");
-- #[toasty::breakpoint]
CREATE INDEX "index_orders_by_customer_id" ON "orders" ("customer_id");
-- #[toasty::breakpoint]
CREATE TABLE "customers" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "name" TEXT NOT NULL,
    "email" TEXT NOT NULL,
    "city" TEXT NOT NULL,
    "joined_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_customers_by_email" ON "customers" ("email");
-- #[toasty::breakpoint]
CREATE TABLE "order_items" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "order_id" INTEGER NOT NULL,
    "product_id" INTEGER NOT NULL,
    "quantity" INTEGER NOT NULL,
    "unit_price" BIGINT NOT NULL
);
-- #[toasty::breakpoint]
CREATE INDEX "index_order_items_by_order_id" ON "order_items" ("order_id");
-- #[toasty::breakpoint]
CREATE INDEX "index_order_items_by_product_id" ON "order_items" ("product_id");
-- #[toasty::breakpoint]
CREATE TABLE "products" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "name" TEXT NOT NULL,
    "sku" TEXT NOT NULL,
    "price" BIGINT NOT NULL,
    "stock" INTEGER NOT NULL
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_products_by_sku" ON "products" ("sku");
