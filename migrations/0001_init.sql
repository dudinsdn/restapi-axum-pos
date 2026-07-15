-- Initial schema for the Postgres-backed repositories. Mirrors the
-- in-memory implementations field-for-field, so switching backends (see
-- `main.rs`) changes nothing about what data is stored or how it's shaped
-- — only where it lives and whether it survives a restart.
--
-- IDs are application-generated strings (e.g. "prod-<uuid>"), not
-- database-generated, so every table's primary key is just TEXT.

CREATE TABLE tenants (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    address TEXT
);

CREATE TABLE users (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants (id),
    name TEXT NOT NULL,
    -- Email is globally unique (used as the login identity), not scoped
    -- per tenant — same rule as `InMemoryUserRepository::create`.
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL
);

CREATE INDEX idx_users_tenant_id ON users (tenant_id);

CREATE TABLE products (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants (id),
    name TEXT NOT NULL,
    sku TEXT NOT NULL,
    -- Whole-Rupiah amounts as BIGINT (i64) — see the `f64` -> `i64` money
    -- migration; there's no fractional subunit to model.
    price BIGINT NOT NULL,
    cost_price BIGINT NOT NULL,
    stock INTEGER NOT NULL,
    -- `Actor { user_id, name }`, stored as-is rather than split into
    -- columns — it's a denormalized snapshot, never queried by its own
    -- fields, only ever read back whole.
    created_by JSONB NOT NULL,
    -- sku is unique per tenant, not globally — same rule as
    -- `InMemoryProductRepository::create`.
    UNIQUE (tenant_id, sku)
);

CREATE INDEX idx_products_tenant_id ON products (tenant_id);

CREATE TABLE customers (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants (id),
    name TEXT NOT NULL,
    phone TEXT NOT NULL,
    email TEXT,
    address TEXT,
    created_by JSONB NOT NULL,
    -- phone is unique per tenant, not globally — same rule as
    -- `InMemoryCustomerRepository::create`.
    UNIQUE (tenant_id, phone)
);

CREATE INDEX idx_customers_tenant_id ON customers (tenant_id);

CREATE TABLE orders (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants (id),
    customer_id TEXT NOT NULL,
    -- Snapshot of the customer's name at order time — see `Order::customer_name`.
    customer_name TEXT NOT NULL,
    -- `Vec<OrderItem>`, each item already a full snapshot of
    -- sku/name/unit_price/unit_cost at order time — never joined against
    -- the live `products` table, so JSONB (not a child table) is the
    -- right fit: an order's items are immutable history, not relational
    -- data that changes independently.
    items JSONB NOT NULL,
    total BIGINT NOT NULL,
    created_by JSONB NOT NULL,
    -- Unix timestamp (seconds). Stored as BIGINT rather than a native
    -- timestamp type to match `Order::created_at: u64` exactly and avoid
    -- any timezone-conversion surprises — see the `reports` module, which
    -- filters on this raw integer.
    created_at BIGINT NOT NULL
);

CREATE INDEX idx_orders_tenant_id ON orders (tenant_id);
CREATE INDEX idx_orders_tenant_created_at ON orders (tenant_id, created_at);

CREATE TABLE audit_log (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants (id),
    actor JSONB NOT NULL,
    action TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    label TEXT NOT NULL,
    -- `Vec<FieldChange>`, empty for Created/Deleted — see `AuditLogEntry::changes`.
    changes JSONB NOT NULL,
    at BIGINT NOT NULL
);

-- Newest-first per tenant is the only access pattern
-- (`AuditLogRepository::list_by_tenant`), so index for exactly that.
CREATE INDEX idx_audit_log_tenant_at ON audit_log (tenant_id, at DESC);
