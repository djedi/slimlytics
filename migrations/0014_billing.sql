-- Hosted-plan billing. No row means the account is on the configured default plan.
CREATE TABLE account_billing (
  user_id uuid PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  plan text NOT NULL,
  -- 'stripe' plans follow the subscription; 'admin' plans are comped and webhook-proof.
  plan_source text NOT NULL DEFAULT 'stripe' CHECK (plan_source IN ('stripe', 'admin')),
  stripe_customer_id text UNIQUE,
  stripe_subscription_id text,
  subscription_status text,
  billing_interval text CHECK (billing_interval IN ('month', 'year')),
  current_period_end timestamptz,
  updated_at timestamptz NOT NULL DEFAULT now()
);

-- Stripe retries and may redeliver events; each is applied at most once.
CREATE TABLE stripe_webhook_events (
  id text PRIMARY KEY,
  type text NOT NULL,
  received_at timestamptz NOT NULL DEFAULT now()
);
