<script lang="ts">
  import { Check, Copy } from '@lucide/svelte';
  import MarketingHeader from '$lib/components/marketing/MarketingHeader.svelte';
  import MarketingFooter from '$lib/components/marketing/MarketingFooter.svelte';

  const connectCommand = 'codex mcp add slimlytics --url https://slimlytics.com/api/mcp\ncodex mcp login slimlytics';
  const prompt = 'Set up Slimlytics analytics for this website at https://shop.example.com. Use first-party anti-adblock delivery. This app runs behind Nginx. Install the routes and tracker, preserve the site’s consent policy, and verify collection.';
  let copyStatus = $state('');
  async function copyConnection() {
    try {
      await navigator.clipboard.writeText(connectCommand);
      copyStatus = 'Connection commands copied';
    } catch {
      copyStatus = 'Copy failed; select the commands manually';
    }
  }
</script>

<svelte:head>
  <title>MCP agent setup · Slimlytics</title>
  <meta name="description" content="Connect your coding agent to Slimlytics with browser OAuth, then install privacy-minded analytics with first-party anti-adblock delivery." />
</svelte:head>

<div class="mkt-shell">
  <MarketingHeader />
  <main id="main" class="mkt-page">
    <header class="mkt-page-head">
      <p class="eyebrow">MCP agent setup</p>
      <h1>Set up analytics with your agent</h1>
      <p class="mkt-lead">Connect once, log in through your browser, and ask your coding agent to add analytics to each website. Slimlytics returns the site’s tracker and first-party proxy configuration for your agent to install.</p>
      <nav aria-label="Documentation navigation">
        <a href="/docs">Documentation overview</a>
        <a href="/docs/cli">CLI guide</a>
        <a href="/docs/api">API reference</a>
      </nav>
    </header>

    <section class="mkt-section">
      <div class="mkt-prose">
        <h2>1. Connect your agent</h2>
        <p><a href="/register">Create a Slimlytics account</a>, then run these commands with the Codex CLI:</p>
        <div class="command-block">
          <pre class="docs-snippet"><code>{connectCommand}</code></pre>
          <button class="secondary" onclick={copyConnection} aria-label="Copy connection commands">
            {#if copyStatus === 'Connection commands copied'}<Check size={16} />{:else}<Copy size={16} />{/if}
            Copy commands
          </button>
        </div>
        <p class="copy-status" aria-live="polite">{copyStatus}</p>
        <p>The browser shows the agent name and requested permissions. Enter your Slimlytics login and select <strong>Log in and authorize</strong>. Restart your agent session to load the tools. Your password stays in the browser login flow.</p>
        <p>In the Codex IDE, add a Streamable HTTP server in MCP settings and select Authenticate. Other agents that support remote MCP OAuth with dynamic client registration can use the same endpoint:</p>
        <pre class="docs-snippet"><code>https://slimlytics.com/api/mcp</code></pre>
        <p>For your own Slimlytics deployment, replace <code>https://slimlytics.com</code> with its public HTTPS origin. Codex connection details are also covered in the <a href="https://learn.chatgpt.com/docs/extend/mcp?surface=cli">official MCP guide</a>.</p>
      </div>
    </section>

    <section class="mkt-section mkt-section-alt">
      <div class="mkt-prose">
        <h2>2. Ask for analytics installation</h2>
        <p>Open your website project in the agent and provide its production domain and hosting setup:</p>
        <blockquote>{prompt}</blockquote>
        <p>The <code>setup_site</code> tool creates or reuses the site by domain. It returns a script tag, exact proxy routes, and verification URLs. The default proxy type is Caddy; your agent can choose Nginx or Apache. Repeating setup reuses the same site.</p>
        <p>Your agent applies the returned configuration to the website using its normal repository and deployment tools. It needs access to your hosting configuration to finish deployment. Slimlytics does not independently deploy an unrelated website.</p>
        <p>For framework or edge hosting, have your agent implement the equivalent two exact server routes. Forward to the fixed Slimlytics origin, preserve the method, body, content type, Origin, Referer, and User-Agent, strip Cookie and Authorization, and remove upstream Set-Cookie. Avoid caching collection responses or accepting arbitrary upstream URLs.</p>
      </div>
    </section>

    <section class="mkt-section">
      <div class="mkt-prose">
        <h2>3. Verify first-party delivery</h2>
        <ol class="docs-steps">
          <li>Install both proxy routes before adding the returned script tag. Validate the server configuration and reload it.</li>
          <li>Open <code>scriptTestUrl</code>; expect JavaScript with HTTP 200.</li>
          <li>Open <code>beaconTestUrl</code>; expect HTTP 200 with <code>{'{"status":"ok"}'}</code>. This check does not insert an event.</li>
          <li>Load a website page and confirm the tracker and collection requests use your website’s own origin.</li>
          <li>Confirm a page view in the dashboard or MCP reports, then check consent, SPA navigation, and unrelated application routes.</li>
        </ol>
        <p>First-party anti-adblock delivery improves reliability while preserving consent, DNT, and GPC. It remains cookieless and cannot guarantee that every blocker permits tracking. For consent-controlled websites, insert the script after consent using the site’s existing consent mechanism.</p>
        <h2>Available tools</h2>
        <div class="table-wrap">
          <table>
            <thead><tr><th>Tool</th><th>Use</th></tr></thead>
            <tbody>
              <tr><td><code>setup_site</code></td><td>Create or reuse a site and get installation artifacts. Accepts name, domain, timezone, allowedOrigins, retentionDays, and serverType.</td></tr>
              <tr><td><code>tracking_setup</code></td><td>Get the current installation configuration for an existing siteId.</td></tr>
              <tr><td><code>list_sites</code></td><td>Find the sites your account can access.</td></tr>
              <tr><td><code>analytics_summary</code></td><td>Inspect metrics and comparisons with an explicit siteId and inclusive from/to dates.</td></tr>
              <tr><td><code>dimension_report</code></td><td>Inspect pages, referrers, countries, devices, and campaigns.</td></tr>
              <tr><td><code>marketing_brief</code></td><td>Get a completed-day marketing summary and supporting evidence.</td></tr>
              <tr><td><code>search_console_report</code></td><td>Read connected Search Console data with integrations:read permission.</td></tr>
            </tbody>
          </table>
        </div>
        <h2>Keep setup predictable</h2>
        <p>Add this instruction to your website’s <code>AGENTS.md</code>:</p>
        <pre class="docs-snippet"><code>When asked to add analytics, use the Slimlytics MCP server.
Call setup_site with the production domain, name, timezone,
and available proxy serverType. Install both same-origin routes
and the returned script. Preserve consent, DNT, and GPC.
Verify both test URLs and a page view. Reuse the site on retries.
Keep account tokens and server ingestion keys out of browser code.
Report any deployment step that still needs operator access.</code></pre>
      </div>
    </section>

    <section class="mkt-section mkt-section-alt">
      <div class="mkt-prose">
        <h2>Permissions and reconnecting</h2>
        <p>Default permissions are <code>sites:read sites:write analytics:read</code>. Site setup also requires owner or administrator access. Read-only agents can request <code>sites:read analytics:read</code>; they cannot create or configure sites.</p>
        <p>OAuth connections appear as <strong>MCP OAuth agent</strong> in your account’s API token settings. Revoke one there to disconnect it immediately. Access tokens expire after 30 days; run <code>codex mcp login slimlytics</code> to reconnect. Refresh tokens are not issued.</p>
        <p>Clients that support only static bearer tokens can use scoped personal API tokens. Keep account tokens, passwords, and server ingestion keys out of committed configuration and browser code. Collection keys in the proxy configuration grant ingestion access only.</p>
        <h2>Self-hosting and troubleshooting</h2>
        <p>Deploy the backend and frontend together, and set <code>SLIMLYTICS_BASE_URL</code> to the public HTTPS origin. Database migrations run at backend startup. The included Caddy configurations route OAuth discovery to the backend.</p>
        <p>With another proxy, route <code>/api/*</code>, <code>/.well-known/oauth-authorization-server</code>, and <code>/.well-known/oauth-protected-resource*</code> to the backend. Keep <code>/p/*</code> on the frontend for the tracker bootstrap. A 404 from OAuth discovery means the deployment or proxy routes need updating.</p>
        <p>From a source checkout, <code>./scripts/connect-agent.sh https://your-analytics-domain</code> runs the Codex connection commands. The server uses stateless Streamable HTTP; an unauthenticated POST advertises browser OAuth with HTTP 401. GET returns 405 because server-to-client SSE is not offered.</p>
      </div>
    </section>
  </main>
  <MarketingFooter />
</div>

<style>
  nav { display: flex; flex-wrap: wrap; gap: 18px; margin-top: 22px; }
  nav a, .mkt-prose a { color: var(--accent); }
  .command-block { display: grid; gap: 10px; }
  .command-block button { justify-self: start; display: inline-flex; align-items: center; gap: 8px; }
  .copy-status { min-height: 1.5em; font-size: 13px; }
  .mkt-prose h2:not(:first-child) { margin-top: 32px; }
  blockquote { margin: 0 0 22px; padding: 18px 22px; border-left: 3px solid var(--accent); background: var(--surface); line-height: 1.7; }
  .table-wrap { overflow-x: auto; }
  table { width: 100%; border-collapse: collapse; font-size: 14px; line-height: 1.6; }
  th, td { padding: 12px; text-align: left; vertical-align: top; border-bottom: 1px solid var(--line); }
  td:first-child { white-space: nowrap; }
</style>
