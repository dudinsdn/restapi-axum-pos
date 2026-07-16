-- Connects `products` to `categories` via a real foreign key —
-- `products.category` (the free-form label from migration 0002) becomes a
-- denormalized display-name snapshot instead of the source of truth, same
-- role `orders.customer_name` already plays relative to `customers`.
--
-- Nullable: a product doesn't have to reference a real category (see
-- `DEFAULT_CATEGORY` in `products::model` for the "uncategorized" case).
--
-- `ON DELETE SET NULL` is a database-level backstop, not the primary
-- mechanism — `categories::service::delete_category` already explicitly
-- clears `category_id` on affected products so the in-memory backend
-- (which has no real FK) behaves the same way. This constraint just makes
-- sure Postgres never ends up with a dangling reference even if that
-- service-layer step is ever skipped (e.g. a direct DB delete).
ALTER TABLE products
    ADD COLUMN category_id TEXT REFERENCES categories (id) ON DELETE SET NULL;

CREATE INDEX idx_products_category_id ON products (category_id);
