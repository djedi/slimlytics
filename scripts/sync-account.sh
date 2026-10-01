#!/usr/bin/env bash
# Copy one account from a remote Slimlytics deployment into the local Docker Compose
# database, so you can sign in locally with the same email/password and see real data.
#
# What is copied: the user row (including its password hash), the sites the user is a
# member of, the user's memberships, and those sites' analytics (events, goals, goal
# completions, funnels, rollups, collection health, Search Console metrics), plus the
# user's own annotations and report subscriptions (imported disabled so local never posts
# to real webhooks).
#
# What is never copied: other users (even co-members of a shared site), API tokens,
# OAuth codes/states, idempotency keys, the agent audit log, live-stream buffers, and
# Search Console connections (their refresh tokens are encrypted with the remote key).
#
# Configuration lives in .env (gitignored):
#   SYNC_SOURCE_SSH             SSH destination of the remote host, e.g. deploy@analytics.example.com
#   SYNC_SOURCE_PATH            Absolute path of the remote Slimlytics checkout
#   SYNC_SOURCE_COMPOSE_FILES   Space-separated remote Compose files (default: compose.yaml)
#   SYNC_EMAIL                  Account to copy
#   SYNC_EVENT_DAYS             Optional: copy only events from the last N days (default: all)
#
# The remote side is only read. Locally, the account's existing user row and the synced
# sites (matched by ID, write key, or domain) are replaced; nothing else is touched.
set -Eeuo pipefail
umask 077

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

usage() {
  cat <<'EOF'
Usage: scripts/sync-account.sh [--email EMAIL] [--yes]

Copies one account and its sites' analytics from a remote Slimlytics deployment into
the local Docker Compose database. Configure SYNC_* settings in .env (see the header of
this script).

  --email EMAIL   Override SYNC_EMAIL
  --yes           Do not ask for confirmation before replacing local data
EOF
}

assume_yes=false
email_override=''
while (($#)); do
  case "$1" in
    --yes) assume_yes=true ;;
    --email) shift; email_override="${1:-}" ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'Unknown option: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

die() { printf '%s\n' "$*" >&2; exit 1; }

# Read KEY=value pairs from .env without executing it.
env_value() {
  local key="$1" line value=''
  [[ -f .env ]] || return 0
  while IFS= read -r line || [[ -n "$line" ]]; do
    [[ "$line" == "$key="* ]] || continue
    value="${line#*=}"
    value="${value%\"}"; value="${value#\"}"
    value="${value%\'}"; value="${value#\'}"
  done < .env
  printf '%s' "$value"
}

[[ -f .env ]] || die 'Missing .env. Run make env first.'
source_ssh="${SYNC_SOURCE_SSH:-$(env_value SYNC_SOURCE_SSH)}"
source_path="${SYNC_SOURCE_PATH:-$(env_value SYNC_SOURCE_PATH)}"
compose_files="${SYNC_SOURCE_COMPOSE_FILES:-$(env_value SYNC_SOURCE_COMPOSE_FILES)}"
compose_files="${compose_files:-compose.yaml}"
email="${email_override:-${SYNC_EMAIL:-$(env_value SYNC_EMAIL)}}"
event_days="${SYNC_EVENT_DAYS:-$(env_value SYNC_EVENT_DAYS)}"
local_base_url="$(env_value SLIMLYTICS_BASE_URL)"

# Strict validation: these values are interpolated into remote shell and SQL text.
[[ "$source_ssh" =~ ^([A-Za-z0-9._-]+@)?[A-Za-z0-9.-]+$ ]] \
  || die 'SYNC_SOURCE_SSH must be user@host (letters, digits, dot, dash, underscore).'
[[ "$source_path" =~ ^/[A-Za-z0-9._/-]+$ && "$source_path" != *..* ]] \
  || die 'SYNC_SOURCE_PATH must be an absolute path without whitespace or "..".'
[[ "$compose_files" =~ ^[A-Za-z0-9._-]+(\ [A-Za-z0-9._-]+)*$ ]] \
  || die 'SYNC_SOURCE_COMPOSE_FILES must be space-separated file names.'
[[ "$email" =~ ^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$ ]] \
  || die 'SYNC_EMAIL (or --email) must be a plain email address.'
[[ -z "$event_days" || "$event_days" =~ ^[1-9][0-9]{0,4}$ ]] \
  || die 'SYNC_EVENT_DAYS must be a positive whole number of days.'
# Never let this overwrite a non-local deployment.
[[ "${local_base_url:-http://localhost:8080}" =~ ^https?://(localhost|127\.0\.0\.1)(:[0-9]+)?/?$ ]] \
  || die "Refusing to run: local SLIMLYTICS_BASE_URL ($local_base_url) is not a loopback address."

for command in ssh docker; do
  command -v "$command" >/dev/null || die "Required command not found: $command"
done
# A loopback base URL does not prove where `docker compose` writes: DOCKER_HOST or a remote
# Docker context would point every local write below at another machine's database.
# DOCKER_HOST takes precedence over the context, matching the Docker CLI.
docker_endpoint="${DOCKER_HOST:-$(docker context inspect --format '{{.Endpoints.docker.Host}}' 2>/dev/null || true)}"
[[ "$docker_endpoint" == unix://* || "$docker_endpoint" == npipe://* ]] \
  || die "Refusing to run: Docker endpoint '${docker_endpoint:-unknown}' is not a local socket. Unset DOCKER_HOST or switch to a local Docker context."
docker compose ps --status running --services 2>/dev/null | grep -qx db \
  || die 'The local db service is not running. Start it with make up or make dev.'

work="$(mktemp -d "${TMPDIR:-/tmp}/slimlytics-sync.XXXXXX")"
control="$work/ssh.sock"
container_dir="/tmp/slimlytics-sync-$$"
cleanup() {
  ssh -o ControlPath="$control" -O exit "$source_ssh" >/dev/null 2>&1 || true
  docker compose exec -T db rm -rf "$container_dir" >/dev/null 2>&1 || true
  rm -rf "$work"
}
trap cleanup EXIT

compose_args=''
for file in $compose_files; do compose_args+=" -f $file"; done

# psql on the remote db; SQL is read from stdin so no values pass through the remote shell.
remote_psql() {
  ssh -o BatchMode=yes -o ControlMaster=auto -o ControlPath="$control" -o ControlPersist=120 \
    "$source_ssh" \
    "cd $source_path && docker compose$compose_args exec -T db sh -c 'psql -X -q -At -v ON_ERROR_STOP=1 -U \"\$POSTGRES_USER\" -d \"\$POSTGRES_DB\" -f -'"
}
local_psql() {
  docker compose exec -T db sh -c 'psql -X -q -At -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -f -'
}

latest_migration='select max(version) from _sqlx_migrations where success;'
printf 'Checking schema versions…\n'
remote_version="$(remote_psql <<<"$latest_migration")"
local_version="$(local_psql <<<"$latest_migration")"
[[ "$remote_version" == "$local_version" ]] || die \
  "Schema mismatch: remote is at migration $remote_version, local at $local_version. Update the older side first."

user_id="$(remote_psql <<SQL
select id from users where lower(email) = lower('$email');
SQL
)"
[[ "$user_id" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$ ]] \
  || die "No account for $email on $source_ssh."

sites="select site_id from site_memberships where user_id = '$user_id'"
event_filter="site_id in ($sites)"
[[ -n "$event_days" ]] && event_filter+=" and occurred_at >= now() - interval '$event_days days'"

# Tables in insert order (parents first). Each query selects every column, so the two
# databases must be on the same migration (checked above).
tables=(users sites site_memberships goals funnels annotations collection_health
  daily_site_rollups search_console_metrics report_subscriptions events goal_completions)
# A function rather than an associative array: macOS ships bash 3.2.
query_for() {
  case "$1" in
    users) echo "select * from users where id = '$user_id'" ;;
    sites) echo "select * from sites where id in ($sites)" ;;
    site_memberships) echo "select * from site_memberships where user_id = '$user_id'" ;;
    annotations) echo "select * from annotations where site_id in ($sites) and created_by = '$user_id'" ;;
    report_subscriptions)
      echo "select * from report_subscriptions where site_id in ($sites) and created_by = '$user_id'" ;;
    events) echo "select * from events where $event_filter" ;;
    goal_completions)
      echo "select * from goal_completions where event_id in (select id from events where $event_filter)" ;;
    *) echo "select * from $1 where site_id in ($sites)" ;;
  esac
}

printf 'Exporting %s from %s…\n' "$email" "$source_ssh"
# One read-only, repeatable-read transaction gives every table the same snapshot, so parents
# and children stay consistent while the remote keeps taking writes. The tables share a single
# output stream, separated by a random marker line that real CSV data won't contain.
marker="__slimlytics_sync_${RANDOM}${RANDOM}_$$__"
{
  echo 'begin isolation level repeatable read read only;'
  for table in "${tables[@]}"; do
    printf '\\echo %s %s\n' "$marker" "$table"
    echo "copy ($(query_for "$table")) to stdout with (format csv);"
  done
  echo 'commit;'
} | remote_psql > "$work/export.stream"
awk -v marker="$marker" -v dir="$work" '
  $1 == marker { file = dir "/" $2 ".csv"; printf "" > file; next }
  file { print > file }
' "$work/export.stream"
rm -f "$work/export.stream"
for table in "${tables[@]}"; do
  [[ -f "$work/$table.csv" ]] || die "Export is missing $table; nothing was changed locally."
  printf '  %-24s %8s rows\n' "$table" "$(wc -l < "$work/$table.csv" | tr -d ' ')"
done
site_count="$(wc -l < "$work/sites.csv" | tr -d ' ')"

if [[ "$assume_yes" != true ]]; then
  printf '\nThis replaces the local account %s and %s site(s) with the copies above.\n' "$email" "$site_count"
  [[ -t 0 ]] || die 'No terminal to confirm on; nothing was changed. Re-run with --yes to proceed.'
  read -r -p 'Continue? [y/N] ' answer
  [[ "$answer" =~ ^[Yy]$ ]] || { echo 'Cancelled.'; exit 1; }
fi

docker compose exec -T db mkdir -p "$container_dir"
for table in "${tables[@]}"; do
  docker compose cp "$work/$table.csv" "db:$container_dir/$table.csv" >/dev/null 2>&1 \
    || die "Could not copy $table.csv into the local db container."
done

{
  echo 'begin;'
  for table in "${tables[@]}"; do
    echo "create temp table sync_$table (like $table) on commit drop;"
    echo "\\copy sync_$table from '$container_dir/$table.csv' with (format csv)"
  done
  # Replace, don't merge: drop the local copy of this account and these sites first.
  # Cascades remove their local memberships, tokens, analytics, and settings.
  echo "delete from users where lower(email) = lower('$email') or id = '$user_id';"
  # Domains are unique, so a local site for the same domain (e.g. one created by hand while
  # testing) is replaced too, along with its local analytics.
  echo "delete from sites where id in (select id from sync_sites)"
  echo "  or lower(domain) in (select lower(domain) from sync_sites)"
  echo "  or write_key in (select write_key from sync_sites)"
  echo "  or server_write_key in (select server_write_key from sync_sites where server_write_key is not null);"
  for table in "${tables[@]}"; do
    echo "insert into $table select * from sync_$table;"
  done
  echo 'update report_subscriptions set enabled = false where id in (select id from sync_report_subscriptions);'
  echo 'commit;'
} | local_psql

printf '\nDone. Sign in at %s/login as %s with the same password.\n' "${local_base_url:-http://localhost:8080}" "$email"
