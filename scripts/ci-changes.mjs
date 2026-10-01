import { execFileSync } from 'node:child_process';
import { appendFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

// Unknown paths select every gate so new build inputs cannot silently bypass CI.
export function checksForPaths(paths) {
  const checks = { backend: false, web: false, cli: false, container: false };
  const full = () => ({ backend: true, web: true, cli: true, container: true });
  if (paths.length === 0) return full();
  for (const path of paths) {
    if (path === 'AGENTS.md') return full();
    if (/^(README|CONTRIBUTING|SECURITY)\.md$/.test(path)
      || /^(backend|frontend|tracker|cli)\/README\.md$/.test(path)
      || /^docs\/.*\.md$/.test(path)) continue;
    if (path === 'docs/openapi.json') {
      checks.backend = checks.web = checks.container = true;
    } else if (path.startsWith('backend/') || path.startsWith('migrations/') || path === 'docker/backend.Dockerfile') {
      checks.backend = checks.container = true;
    } else if (path.startsWith('frontend/') || path.startsWith('tracker/') || path === 'docker/frontend.Dockerfile') {
      checks.web = checks.container = true;
    } else if (path.startsWith('cli/') || path === 'scripts/install-cli.sh') {
      checks.cli = true;
    } else if (path.startsWith('docker/') || /^compose(?:\.[^/]+)?\.ya?ml$/.test(path)) {
      checks.container = true;
    } else {
      return full();
    }
  }
  return checks;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  let paths = [];
  if (process.env.GITHUB_EVENT_NAME === 'pull_request') {
    const base = process.env.CI_BASE_SHA;
    const head = process.env.CI_HEAD_SHA;
    if (![base, head].every((sha) => /^[a-f0-9]{40}$/.test(sha ?? ''))) {
      throw new Error('PR comparison requires complete base and head commit SHAs');
    }
    // --no-renames reports both removed and added paths when a file moves.
    paths = execFileSync('git', ['diff', '--no-renames', '--name-only', '-z', `${base}...${head}`], { encoding: 'utf8' }).split('\0').filter(Boolean);
  }
  const selected = checksForPaths(paths);
  const output = Object.entries(selected).map(([key, value]) => `${key}=${value}\n`).join('');
  if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, output);
  process.stdout.write(output);
}
