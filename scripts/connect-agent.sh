#!/bin/sh
# Connect Codex to a deployed Slimlytics server. Credentials stay in the OAuth browser flow.
set -eu
if [ "$#" -ne 1 ]; then
    echo "Usage: $0 https://analytics.example.com" >&2
    exit 2
fi
command -v codex >/dev/null 2>&1 || { echo 'Install the Codex CLI first.' >&2; exit 1; }
case "$1" in
    https://*|http://localhost:*|http://127.0.0.1:*) ;;
    *) echo 'Use an HTTPS origin, or HTTP localhost for development.' >&2; exit 2 ;;
esac
agent_origin=${1%/}
codex mcp add slimlytics --url "$agent_origin/api/mcp"
codex mcp login slimlytics
