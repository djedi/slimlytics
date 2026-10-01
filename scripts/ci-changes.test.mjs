import assert from 'node:assert/strict';
import { test } from 'node:test';
import { checksForPaths } from './ci-changes.mjs';

const none = { backend: false, web: false, cli: false, container: false };
const all = { backend: true, web: true, cli: true, container: true };
test('prose documentation requires no application builds', () => {
  assert.deepEqual(checksForPaths(['README.md', 'docs/MCP.md', 'backend/README.md']), none);
});
test('frontend pages still get web and container verification', () => {
  assert.deepEqual(checksForPaths(['frontend/src/routes/docs/mcp/+page.svelte']), { ...none, web: true, container: true });
});
test('backend and migrations require database tests and container verification', () => {
  for (const path of ['backend/src/app.rs', 'migrations/0012_mcp_oauth.sql']) {
    assert.deepEqual(checksForPaths([path]), { ...none, backend: true, container: true });
  }
});
test('OpenAPI is a compile input for both backend and frontend', () => {
  assert.deepEqual(checksForPaths(['docs/openapi.json']), { ...none, backend: true, web: true, container: true });
});
test('tracker changes require web and container checks', () => {
  assert.deepEqual(checksForPaths(['tracker/src/index.ts']), { ...none, web: true, container: true });
});
test('CLI-only changes avoid unrelated builds', () => {
  assert.deepEqual(checksForPaths(['cli/src/main.rs', 'scripts/install-cli.sh']), { ...none, cli: true });
});
test('Docker changes require their matching build checks', () => {
  assert.deepEqual(checksForPaths(['docker/frontend.Dockerfile']), { ...none, web: true, container: true });
  assert.deepEqual(checksForPaths(['docker/backend.Dockerfile']), { ...none, backend: true, container: true });
  assert.deepEqual(checksForPaths(['docker/Caddyfile']), { ...none, container: true });
});
test('CI, tooling, unknown paths and absent diffs conservatively run all checks', () => {
  for (const paths of [[], ['.github/workflows/ci.yml'], ['scripts/ci-changes.mjs'], ['rust-toolchain.toml'], ['Makefile'], ['scripts/server-log-forwarder.mjs'], ['.dockerignore'], ['new-area/code.ts'], ['AGENTS.md']]) {
    assert.deepEqual(checksForPaths(paths), all);
  }
});
test('change unions include both sides of moves and deletions', () => {
  assert.deepEqual(checksForPaths(['frontend/src/old.ts', 'backend/src/new.rs', 'cli/src/old.rs']), all);
});
