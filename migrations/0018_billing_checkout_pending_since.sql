-- When the unresolved checkout attempt started. An attempt whose replay still gets no
-- definitive answer long after it began is retired (any session it made is expired by then).
ALTER TABLE account_billing ADD COLUMN checkout_pending_since timestamptz;
