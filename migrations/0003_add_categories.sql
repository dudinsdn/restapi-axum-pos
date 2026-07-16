-- Categories as their own manageable resource — see `Category` in
-- `categories::model` for why this exists alongside the free-form
-- `products.category` column rather than replacing it.
CREATE TABLE categories (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants (id),
    name TEXT NOT NULL,
    created_by JSONB NOT NULL
);

CREATE INDEX idx_categories_tenant_id ON categories (tenant_id);

-- Case-insensitive uniqueness per tenant, matching
-- `InMemoryCategoryRepository::create`'s `eq_ignore_ascii_case` check —
-- a plain `UNIQUE (tenant_id, name)` would let "Snacks" and "snacks"
-- coexist as two different categories, which the in-memory backend
-- already refuses.
CREATE UNIQUE INDEX idx_categories_tenant_name_ci
    ON categories (tenant_id, LOWER(name));
