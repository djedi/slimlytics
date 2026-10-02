-- When the unresolved checkout attempt was last sent (first send or replay). It is retired only
-- once it is old (checkout_pending_since) and has not been sent recently, so a permanently
-- cached failure can't block checkout forever, yet a replay's effects settle first.
ALTER TABLE account_billing ADD COLUMN checkout_last_sent timestamptz;
