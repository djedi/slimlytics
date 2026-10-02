-- The price of a checkout attempt that got no definitive answer from Stripe. Its idempotency
-- key is replayed with this exact price, settling it before any checkout for another price.
ALTER TABLE account_billing ADD COLUMN checkout_pending_price text;
