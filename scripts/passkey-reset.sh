#!/usr/bin/env bash
# Remove every passkey from an account, for someone who lost all of theirs. Also signs the
# account out everywhere and revokes its API tokens and MCP connections. They can then sign
# in with their password and add a new passkey.
#
#   scripts/passkey-reset.sh you@example.com
#
# Confirm the person's identity out of band first: this is the recovery path, and passkey
# removal through the app always requires an existing passkey.
# Runs against the Compose database (set COMPOSE_FILE or run from the deployment directory
# for production).
set -Eeuo pipefail
die() { printf '%s\n' "$*" >&2; exit 1; }
[[ $# -eq 1 ]] || die 'Usage: scripts/passkey-reset.sh EMAIL'
email="$1"
[[ "$email" =~ ^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$ ]] || die 'EMAIL must be a plain email address.'

# Values go in as psql variables (:'name' quoting), never interpolated into SQL text.
docker compose exec -T db sh -c 'psql -X -q -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -v email="$1" -f -' sh "$email" <<'SQL'
\set QUIET on
SELECT id AS user_id, email AS user_email FROM users WHERE lower(email) = lower(:'email') \gset
\if :{?user_id}
\else
  \echo 'No account with that email.'
  \quit
\endif
BEGIN;
-- Lock the account first, as sign-in and enrollment do, so none can slip in mid-reset.
SELECT 1 FROM users WHERE id = :'user_id' FOR NO KEY UPDATE;
DELETE FROM user_passkeys WHERE user_id = :'user_id';
UPDATE user_sessions SET revoked_at = now() WHERE user_id = :'user_id' AND revoked_at IS NULL;
-- API tokens and MCP connections may have been minted from a compromised session.
UPDATE api_tokens SET revoked_at = now() WHERE user_id = :'user_id' AND revoked_at IS NULL;
DELETE FROM oauth_codes WHERE user_id = :'user_id';
INSERT INTO admin_audit_log(actor_email, action, target_user_id, target_email)
VALUES ('scripts/passkey-reset.sh', 'passkeys.reset', :'user_id', :'user_email');
COMMIT;
\echo 'Passkeys removed and every session signed out. Sign in with the password and add a new passkey.'
SQL
