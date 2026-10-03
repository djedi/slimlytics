#!/usr/bin/env bash
# Grant or remove platform admin access. Admin status is never settable through the API.
#
#   scripts/admin-grant.sh you@example.com            # make an admin
#   scripts/admin-grant.sh you@example.com --revoke   # remove admin access
#
# Admins must also add a passkey (Account security) before the admin portal opens: every
# admin request needs a session that verified a passkey within the last 12 hours.
# Runs against the Compose database (set COMPOSE_FILE or run from the deployment directory
# for production).
set -Eeuo pipefail
die() { printf '%s\n' "$*" >&2; exit 1; }
[[ $# -eq 1 || ( $# -eq 2 && "$2" == --revoke ) ]] || die 'Usage: scripts/admin-grant.sh EMAIL [--revoke]'
email="$1"; mode="${2:---grant}"
[[ "$email" =~ ^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$ ]] || die 'EMAIL must be a plain email address.'

# Values go in as psql variables (:'name' quoting), never interpolated into SQL text.
docker compose exec -T db sh -c 'psql -X -q -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -v email="$1" -v mode="$2" -f -' sh "$email" "$mode" <<'SQL'
\set QUIET on
SELECT id AS user_id, email AS user_email FROM users WHERE lower(email) = lower(:'email') \gset
\if :{?user_id}
\else
  \echo 'No account with that email. Register it first, then run this again.'
  \quit
\endif
SELECT (:'mode' = '--revoke') AS revoke \gset
BEGIN;
\if :revoke
  UPDATE users SET is_admin = false, updated_at = now() WHERE id = :'user_id';
  INSERT INTO admin_audit_log(actor_email, action, target_user_id, target_email)
  VALUES ('scripts/admin-grant.sh', 'admin.revoke', :'user_id', :'user_email');
  \echo 'Admin access removed.'
\else
  UPDATE users SET is_admin = true, updated_at = now() WHERE id = :'user_id';
  INSERT INTO admin_audit_log(actor_email, action, target_user_id, target_email)
  VALUES ('scripts/admin-grant.sh', 'admin.grant', :'user_id', :'user_email');
  \echo 'Admin access granted. Add a passkey under Account security to open /admin.'
\endif
COMMIT;
SELECT email, is_admin, (SELECT count(*) FROM user_passkeys p WHERE p.user_id = users.id) AS passkeys
FROM users WHERE id = :'user_id';
SQL
