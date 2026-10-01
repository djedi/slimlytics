#!/usr/bin/env bash
# Download the free DB-IP "IP to City Lite" database (CC BY 4.0, monthly, no account) into
# data/geoip/ for the backend's local GeoIP lookups. Visitor IPs never leave the server.
#
# Idempotent: does nothing when this month's file is already installed. Early in a month,
# before DB-IP publishes the new release, it falls back to the previous month.
# Restart the backend afterwards to load a new file (make geoip does this).
#
# Attribution required by the licence: "IP Geolocation by DB-IP" linking to https://db-ip.com
set -Eeuo pipefail
umask 022

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dir="${GEOIP_DIR:-$repo_root/data/geoip}"
target="$dir/dbip-city-lite.mmdb"
version_file="$dir/VERSION"

die() { printf '%s\n' "$*" >&2; exit 1; }
command -v curl >/dev/null || die 'curl is required.'
command -v gzip >/dev/null || die 'gzip is required.'
mkdir -p "$dir"

month_offset() {
  # GNU date and BSD/macOS date spell relative months differently.
  date -u -d "$(date -u +%Y-%m-01) -$1 month" +%Y-%m 2>/dev/null || date -u -v-"$1"m +%Y-%m
}

installed="$(cat "$version_file" 2>/dev/null || true)"
for offset in 0 1; do
  month="$(month_offset "$offset")"
  if [[ "$installed" == "$month" && -s "$target" ]]; then
    printf 'GeoIP database is current (DB-IP City Lite %s).\n' "$month"
    exit 0
  fi
  url="https://download.db-ip.com/free/dbip-city-lite-$month.mmdb.gz"
  tmp="$(mktemp "$dir/.download.XXXXXX")"
  trap 'rm -f "$tmp" "$tmp.mmdb"' EXIT
  printf 'Downloading DB-IP City Lite %s…\n' "$month"
  if ! curl --fail --silent --show-error --location --retry 3 --max-time 600 -o "$tmp" "$url"; then
    rm -f "$tmp"
    printf 'Not available yet: %s\n' "$url" >&2
    continue
  fi
  gzip -dc "$tmp" > "$tmp.mmdb"
  # Every MaxMind-format database ends with this metadata marker; reject anything else.
  tail -c 131072 "$tmp.mmdb" | LC_ALL=C grep -aq $'\xab\xcd\xefMaxMind.com' \
    || die "Downloaded file is not a valid MMDB database: $url"
  chmod 644 "$tmp.mmdb"
  mv -f "$tmp.mmdb" "$target"
  printf '%s\n' "$month" > "$version_file"
  rm -f "$tmp"
  printf 'Installed %s (%s).\n' "$target" "$(du -h "$target" | cut -f1)"
  exit 0
done

[[ -s "$target" ]] || die 'Could not download a DB-IP City Lite database.'
printf 'Download failed; keeping the existing database (%s).\n' "${installed:-unknown version}" >&2
