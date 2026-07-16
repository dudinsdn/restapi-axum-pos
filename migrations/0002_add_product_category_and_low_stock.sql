-- Adds `category` (a free-form grouping label) and `low_stock_threshold`
-- (per-product reorder point) to `products` — see `Product::category` and
-- `Product::low_stock_threshold` for the reasoning.
--
-- A separate migration rather than editing `0001_init.sql` — once a
-- migration has shipped (even to just one environment), editing it in
-- place means `sqlx migrate run` on an already-migrated database silently
-- does nothing, while a fresh database gets a different schema. Additive
-- migrations avoid that split entirely.
--
-- Defaults are provided so this is safe to run against a table that
-- already has rows (existing products all become "Uncategorized" with a
-- threshold of 5, matching `DEFAULT_CATEGORY` /
-- `DEFAULT_LOW_STOCK_THRESHOLD` in `products::model`).
ALTER TABLE products
    ADD COLUMN category TEXT NOT NULL DEFAULT 'Uncategorized',
    ADD COLUMN low_stock_threshold INTEGER NOT NULL DEFAULT 5;

CREATE INDEX idx_products_tenant_category ON products (tenant_id, category);
