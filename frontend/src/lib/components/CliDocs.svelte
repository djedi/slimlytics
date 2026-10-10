<script lang="ts">
  import { ExternalLink, KeyRound, Lock, ShieldCheck, Terminal } from '@lucide/svelte';
  import MarketingHeader from '$lib/components/marketing/MarketingHeader.svelte';
  import MarketingFooter from '$lib/components/marketing/MarketingFooter.svelte';
  import CodeBlock from '$lib/components/marketing/CodeBlock.svelte';
  import { cliCommands, installCommand } from '$lib/cli-docs';

  const toc = [
    ['install', 'Install'],
    ['authenticate', 'Authenticate'],
    ['commands', 'Command reference'],
    ['automation', 'Agents and JSON'],
    ['configuration', 'Configuration'],
    ['security', 'Security']
  ];

  const jsonExample = `{
  "schemaVersion": 1,
  "ok": true,
  "data": {
    "created": true,
    "site": { "id": "…", "domain": "example.com" },
    "tracking": {
      "serverConfig": "…",
      "snippet": "<script async src=\\"/…js\\"><\\/script>",
      "scriptTestUrl": "https://example.com/…js",
      "beaconTestUrl": "https://example.com/…"
    }
  }
}`;
</script>

<svelte:head>
  <title>CLI documentation · Slimlytics</title>
  <meta
    name="description"
    content="Install and use the Slimlytics command-line client for account, token, site, and first-party tracking management."
  />
</svelte:head>

<div class="mkt-shell">
  <MarketingHeader />
  <main id="main" class="mkt-page">
    <header class="mkt-page-head">
      <p class="eyebrow">Command-line reference</p>
      <h1>Slimlytics CLI</h1>
      <p class="mkt-lead">
        Manage your account, personal API tokens, sites, and complete first-party tracking setup from a
        terminal or AI agent.
      </p>
      <nav class="docs-nav" aria-label="Documentation navigation">
        <a href="/docs">Documentation overview</a>
        <a href="/docs/mcp">MCP agent setup</a>
        <a href="/api/docs">Interactive API reference <ExternalLink size={14} aria-hidden="true" /></a>
      </nav>
    </header>

    <div class="docs-layout">
      <aside class="docs-toc" aria-label="On this page">
        <p>On this page</p>
        {#each toc as [id, label]}
          <a href={`#${id}`}>{label}</a>
        {/each}
      </aside>

      <article class="mkt-prose docs-article">
        <div class="docs-notice">
          <Terminal size={20} aria-hidden="true" />
          <p>
            <strong>Why the command is <code>slimlytics</code>, not <code>slim</code>:</strong>
            SlimToolkit already owns the popular <code>slim</code> command and has more than 23,000 GitHub
            stars. Avoiding that collision keeps installs predictable.
          </p>
        </div>

        <section id="install">
          <h2>Install</h2>
          <p>
            Cargo builds the pinned <code>cli-v0.2.0</code> release tag straight from GitHub with a locked
            release build. Requirements: a recent stable Rust toolchain with Cargo.
          </p>
          <CodeBlock code={installCommand} label="install command" />
          <CodeBlock code={'slimlytics --version\nslimlytics --help'} label="version check" />
          <p>Install from a local checkout instead:</p>
          <CodeBlock code="cargo install --locked --path cli" label="local install command" />
          <p>
            Cargo places the executable in <code>{'${CARGO_HOME:-$HOME/.cargo}'}/bin</code>. Ensure that
            directory is on <code>PATH</code>.
          </p>
        </section>

        <section id="authenticate">
          <h2>Authenticate</h2>
          <p>
            Interactive login exchanges your password for a short-lived session and then creates a
            durable, revocable personal API token:
          </p>
          <CodeBlock code={'slimlytics auth login --email you@example.com\nslimlytics auth status'} label="login commands" />
          <p>For automation, keep passwords and tokens out of process arguments:</p>
          <CodeBlock
            label="automation login commands"
            code={`printf '%s\\n' "$SLIMLYTICS_PASSWORD" | \\
  slimlytics auth login --email you@example.com --password-stdin

printf '%s\\n' "$SLIMLYTICS_TOKEN" | \\
  slimlytics auth use-token --token-stdin`}
          />
          <p>Revoke the active personal token while signing out:</p>
          <CodeBlock code="slimlytics auth logout --revoke" label="logout command" />
        </section>

        <section id="commands">
          <h2>Complete command reference</h2>
          <p>
            Global options can appear before or after the command: <code>--json</code> emits
            machine-readable output and <code>--api-url URL</code> selects a self-hosted API.
          </p>
          <div class="command-list">
            {#each cliCommands as command}
              <section class="command-card">
                <code class="command-usage">{command.usage}</code>
                <h3>{command.summary}</h3>
                <p>{command.details}</p>
                {#if command.options}
                  <ul>
                    {#each command.options as option}<li>{option}</li>{/each}
                  </ul>
                {/if}
              </section>
            {/each}
          </div>
        </section>

        <section id="automation">
          <h2>AI agents and JSON output</h2>
          <p>
            <code>site ensure</code> is the primary agent workflow. The server performs the ensure
            transaction atomically, so retries cannot create duplicate domains.
          </p>
          <CodeBlock code="slimlytics --json site ensure example.com --server caddy" label="site ensure command" />
          <p>
            The versioned response envelope includes <code>schemaVersion</code>, <code>ok</code>, whether
            the site was created, all site settings, reverse-proxy configuration, the script snippet,
            verification URLs, and ordered steps.
          </p>
          <CodeBlock code={jsonExample} label="JSON response" lang="json" />
          <p>
            Successful JSON goes to stdout. Errors go to stderr and return a nonzero exit status. Login,
            status, and listings never print password or token secrets.
          </p>
        </section>

        <section id="configuration">
          <h2>Configuration and environment</h2>
          <div class="table-wrap">
            <table>
              <thead><tr><th scope="col">Name</th><th scope="col">Purpose</th></tr></thead>
              <tbody>
                <tr><td><code>SLIMLYTICS_API_URL</code></td><td>API base URL. Defaults to <code>https://slimlytics.com</code>. Remote URLs must use HTTPS; HTTP is accepted only for loopback development.</td></tr>
                <tr><td><code>SLIMLYTICS_TOKEN</code></td><td>Use a personal API token directly instead of the saved credential file.</td></tr>
                <tr><td><code>SLIMLYTICS_CLI_REF</code></td><td>Installer source tag override. The standard installer pins <code>cli-v0.2.0</code>.</td></tr>
                <tr><td><code>CARGO_HOME</code></td><td>Controls Cargo's install directory.</td></tr>
              </tbody>
            </table>
          </div>
          <p>
            The credential file is stored in the platform configuration directory—for example,
            <code>~/Library/Application Support/slimlytics/auth.json</code> on macOS or
            <code>~/.config/slimlytics/auth.json</code> on Linux.
          </p>
        </section>

        <section id="security">
          <h2>Security guarantees</h2>
          <div class="security-grid">
            <article class="feature-card">
              <span class="feature-icon"><Lock size={20} aria-hidden="true" /></span>
              <h3>Protected transport</h3>
              <p>Remote APIs require HTTPS and the HTTP client refuses all redirects, preventing credentials from following downgrade or cross-origin responses.</p>
            </article>
            <article class="feature-card">
              <span class="feature-icon"><ShieldCheck size={20} aria-hidden="true" /></span>
              <h3>Private local storage</h3>
              <p>Credential directories use mode 0700 and files mode 0600 on Unix. Writes are atomic and refuse symbolic-link targets.</p>
            </article>
            <article class="feature-card">
              <span class="feature-icon"><KeyRound size={20} aria-hidden="true" /></span>
              <h3>Revocable tokens</h3>
              <p>Personal tokens are random, hashed at rest, expire, and can be revoked individually. Plaintext is returned only at creation.</p>
            </article>
          </div>
          <p>
            The CLI generates configuration but deliberately does not SSH into unrelated servers, reload
            web servers, or modify website templates. The calling human or agent retains those privileges
            and rollback responsibilities.
          </p>
        </section>

        <p class="docs-callout">
          Need raw HTTP details? Open the <a href="/api/docs">interactive API reference</a> or download
          <a href="/api/openapi.json">OpenAPI JSON</a>.
        </p>
      </article>
    </div>
  </main>
  <MarketingFooter />
</div>
