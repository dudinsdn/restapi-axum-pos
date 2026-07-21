-- `at` is only second-precision, so rapid sequential actions (the common
-- case in tests, and possible in production under load) can tie. `seq`
-- is a true insertion-order tiebreaker, independent of the clock.
ALTER TABLE audit_log ADD COLUMN seq BIGSERIAL;

DROP INDEX idx_audit_log_tenant_at;
CREATE INDEX idx_audit_log_tenant_seq ON audit_log (tenant_id, seq DESC);
