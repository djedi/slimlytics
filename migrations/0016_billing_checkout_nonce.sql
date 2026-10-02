-- Numbers checkout attempts, so a retry after an ambiguous Stripe failure reuses the same
-- idempotency key (and thus the same Checkout Session) instead of creating a second one.
ALTER TABLE account_billing ADD COLUMN checkout_nonce bigint NOT NULL DEFAULT 0;
