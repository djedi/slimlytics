#!/usr/bin/env bash
# Builds the production release artifacts into OUT_DIR (default .prebuilt):
#   backend/   x86-64 Linux binary, migrations, Dockerfile
#   frontend/  SvelteKit adapter-node build, package files, Dockerfile
#   scalar-assets.json  expected /docs/api Scalar CSS digests for post-deploy verification
# The backend cross-compiles in Docker (docker/backend-cross.Dockerfile) while the tracker and
# frontend build on the host.
set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
out="${1:-.prebuilt}"
rm -rf "$out"
mkdir -p "$out/backend" "$out/frontend"

docker build --quiet -f docker/backend-cross.Dockerfile --output "type=local,dest=$out/backend" . >/dev/null &
backend_pid=$!

npm --prefix tracker ci --no-audit --no-fund
npm --prefix tracker run build
mkdir -p frontend/src/lib/server/generated
cp tracker/dist/slimlytics.js frontend/static/tracker.js
cp tracker/dist/slimlytics.js frontend/static/s.js
cp tracker/dist/slimlytics.js frontend/src/lib/server/generated/tracker.iife.txt
npm --prefix frontend ci --no-audit --no-fund
# Same build settings as docker/frontend.Dockerfile.
PUBLIC_API_BASE_URL=/api PUBLIC_DEMO_MODE=false npm --prefix frontend run build
node frontend/scripts/verify-api-docs-build.mjs --json > "$out/scalar-assets.json"
cp -R frontend/build "$out/frontend/build"
cp frontend/package.json frontend/package-lock.json docker/prebuilt/frontend.Dockerfile "$out/frontend/"
mv "$out/frontend/frontend.Dockerfile" "$out/frontend/Dockerfile"

wait "$backend_pid"
cp -R migrations "$out/backend/migrations"
cp docker/prebuilt/backend.Dockerfile "$out/backend/Dockerfile"
test -x "$out/backend/slimlytics-backend"
printf 'Built release artifacts in %s\n' "$out"
