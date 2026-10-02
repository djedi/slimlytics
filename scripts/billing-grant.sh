#!/usr/bin/env bash
# Grant (comp) a plan to an account, or hand it back to Stripe billing.
#
#   scripts/billing-grant.sh you@example.com unlimited   # no limits, ignores Stripe
#   scripts/billing-grant.sh you@example.com business    # comp a configured plan
#   scripts/billing-grant.sh you@example.com --release   # plan follows Stripe again
#
# Admin plans survive webhooks. Runs against the Compose database (set COMPOSE_FILE or run
# from the deployment directory for production).
set -Eeuo pipefail
die() { printf '%s\n' "$*" >&2; exit 1; }
[[ $# -eq 2 ]] || die 'Usage: scripts/billing-grant.sh EMAIL PLAN|--release'
email="$1"; plan="$2"
[[ "$email" =~ ^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$ ]] || die 'EMAIL must be a plain email address.'
[[ "$plan" == --release || "$plan" =~ ^[A-Za-z0-9_-]{1,64}$ ]] || die 'PLAN must be a plan id (letters, digits, - and _).'

# Values go in as psql variables (:'name' quoting), never interpolated into SQL text.
docker compose exec -T db sh -c 'psql -X -q -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -v email="$1" -v plan="$2" -f -' sh "$email" "$plan" <<'SQL'
\set QUIET on
SELECT id AS user_id FROM users WHERE lower(email) = lower(:'email') \gset
\if :{?user_id}
\else
  \echo 'No account with that email.'
  \quit
\endif
SELECT (:'plan' = '--release') AS release \gset
\if :release
  UPDATE account_billing SET admin_plan = NULL, updated_at = now() WHERE user_id = :'user_id';
  \echo 'Released: the plan now follows the Stripe subscription, or the default plan when there is none.'
\else
  -- `plan` (the Stripe-derived plan) starts as the grant for new rows but is unused while
  -- admin_plan is set; webhooks overwrite it with the subscription's real plan.
  INSERT INTO account_billing(user_id, plan, admin_plan) VALUES (:'user_id', :'plan', :'plan')
  ON CONFLICT (user_id) DO UPDATE SET admin_plan = EXCLUDED.admin_plan, updated_at = now();
  \echo 'Granted.'
\endif
SELECT u.email, coalesce(b.admin_plan, b.plan) AS plan, b.admin_plan IS NOT NULL AS granted FROM users u JOIN account_billing b ON b.user_id = u.id WHERE u.id = :'user_id';
SQL
