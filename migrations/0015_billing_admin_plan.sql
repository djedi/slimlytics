-- Keep admin grants separate from the Stripe-derived plan, so releasing a grant falls back to
-- the subscription's real plan instead of leaving the granted one in place.
ALTER TABLE account_billing ADD COLUMN admin_plan text;
UPDATE account_billing SET admin_plan = plan WHERE plan_source = 'admin';
ALTER TABLE account_billing DROP COLUMN plan_source;
