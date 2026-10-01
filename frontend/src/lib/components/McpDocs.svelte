<script lang="ts">
  import MarketingHeader from '$lib/components/marketing/MarketingHeader.svelte';
  import MarketingFooter from '$lib/components/marketing/MarketingFooter.svelte';
  import CodeBlock from '$lib/components/marketing/CodeBlock.svelte';

  const mcpUrl = 'https://slimlytics.com/api/mcp';

  const claudeConnect = `claude mcp add --transport http slimlytics ${mcpUrl}`;
  const claudeUserScope = `claude mcp add --transport http --scope user slimlytics ${mcpUrl}`;
  const claudeProjectJson = `{
  "mcpServers": {
    "slimlytics": {
      "type": "http",
      "url": "${mcpUrl}"
    }
  }
}`;
  const claudeSettings = `{
  "permissions": {
    "allow": [
      "mcp__slimlytics__list_sites",
      "mcp__slimlytics__analytics_summary",
      "mcp__slimlytics__dimension_report",
      "mcp__slimlytics__marketing_brief"
    ]
  }
}`;
  const claudeHeadless = `claude -p "Summarize last week's traffic for shop.example.com \\
  with the Slimlytics MCP server. Compare it with the week before." \\
  --allowedTools "mcp__slimlytics__list_sites,mcp__slimlytics__analytics_summary,mcp__slimlytics__dimension_report,mcp__slimlytics__marketing_brief"`;
  const claudeToken = `claude mcp add --transport http slimlytics ${mcpUrl} \\
  --header "Authorization: Bearer $SLIMLYTICS_TOKEN"`;

  const connectCommand = `codex mcp add slimlytics --url ${mcpUrl}\ncodex mcp login slimlytics`;

  const hermesConfig = `mcp_servers:
  slimlytics:
    url: "${mcpUrl}"
    auth: oauth`;
  const hermesCommands = `hermes mcp login slimlytics
hermes mcp test slimlytics`;
  const hermesReadOnly = `mcp_servers:
  slimlytics:
    url: "${mcpUrl}"
    auth: oauth
    tools:
      include: [list_sites, analytics_summary, dimension_report, marketing_brief]`;

  const prompt =
    'Set up Slimlytics analytics for this website at https://shop.example.com. Use first-party anti-adblock delivery. This app runs behind Nginx. Install the routes and tracker, preserve the site’s consent policy, and verify collection.';

  const promptLibrary = [
    {
      title: 'Install on a framework or edge host',
      body: 'Add Slimlytics to this SvelteKit app deployed on Vercel. Call setup_site for https://blog.example.com, implement the two returned proxy paths as server routes that forward to the fixed Slimlytics origin, add the script tag to the root layout, and verify both test URLs after deploying a preview.'
    },
    {
      title: 'Weekly traffic summary',
      body: 'Using Slimlytics, summarize traffic for shop.example.com from last Monday through Sunday. Compare it with the previous week and call out the three pages and three referrers with the biggest change.'
    },
    {
      title: 'Campaign check',
      body: 'Pull the campaigns report for shop.example.com for the last 14 days. Which utm_campaign values brought visitors who reached the signup goal, and which ones only produced bounces?'
    },
    {
      title: 'Daily marketing brief',
      body: 'Get yesterday’s marketing_brief for shop.example.com. Turn it into three concrete actions for the content team, citing the supporting numbers.'
    },
    {
      title: 'Search Console gaps',
      body: 'Compare search_console_report queries with the pages report for the last 28 days. List queries with many impressions but a low click-through rate, and suggest title or description changes for the matching pages.'
    },
    {
      title: 'Fleet audit',
      body: 'List every Slimlytics site I can access. For each one, fetch tracking_setup and check the scriptTestUrl and beaconTestUrl. Report any site whose first-party routes are missing or failing.'
    }
  ];
</script>

<svelte:head>
  <title>MCP agent setup · Slimlytics</title>
  <meta
    name="description"
    content="Connect Claude Code, Codex, Hermes Agent, or any MCP client to Slimlytics with browser OAuth, then install privacy-minded analytics with first-party anti-adblock delivery."
  />
</svelte:head>

<div class="mkt-shell">
  <MarketingHeader />
  <main id="main" class="mkt-page">
    <header class="mkt-page-head">
      <p class="eyebrow">MCP agent setup</p>
      <h1>Set up analytics with your agent</h1>
      <p class="mkt-lead">
        Connect once, log in through your browser, and ask your coding agent to add analytics to each
        website. Slimlytics returns the site’s tracker and first-party proxy configuration for your
        agent to install.
      </p>
      <nav class="docs-nav" aria-label="Documentation navigation">
        <a href="/docs">Documentation overview</a>
        <a href="/docs/cli">CLI guide</a>
        <a href="/docs/api">API reference</a>
      </nav>
    </header>

    <section class="mkt-section docs-body" aria-labelledby="connect-title">
      <div class="mkt-prose">
        <h2 id="connect-title">1. Connect your agent</h2>
        <p>
          <a href="/register">Create a Slimlytics account</a>, then pick your agent. Every client uses
          the same Streamable HTTP endpoint with browser OAuth:
        </p>
        <CodeBlock code={mcpUrl} label="MCP server URL" lang="url" />
        <nav class="client-tabs" aria-label="Agent clients">
          <a href="#claude-code">Claude Code</a>
          <a href="#codex">Codex</a>
          <a href="#hermes">Hermes Agent</a>
          <a href="#other-clients">Other clients</a>
        </nav>

        <article id="claude-code" class="client">
          <h3>Claude Code</h3>
          <p>Add the server from your project directory:</p>
          <CodeBlock code={claudeConnect} label="Claude Code add command" />
          <p>
            Start <code>claude</code>, run <code>/mcp</code>, select <strong>slimlytics</strong>, and
            choose <strong>Authenticate</strong>. Your browser opens the Slimlytics consent screen; after
            you approve, the tools load in the current session. Check the connection any time with
            <code>claude mcp list</code> or <code>claude mcp get slimlytics</code>.
          </p>
          <p>
            By default the server is available only in the current project. Use
            <code>--scope user</code> to make it available everywhere you run Claude Code:
          </p>
          <CodeBlock code={claudeUserScope} label="Claude Code user-scope command" />
          <p>
            To share the connection with your team, commit a <code>.mcp.json</code> at the repository
            root (or run the add command with <code>--scope project</code>). It contains only the URL —
            each teammate authenticates with their own Slimlytics login.
          </p>
          <CodeBlock code={claudeProjectJson} label=".mcp.json" lang="json" />
          <p>
            Pre-approve the read-only reporting tools in <code>.claude/settings.json</code> so routine
            questions do not prompt for permission. Leave <code>setup_site</code> on manual approval.
          </p>
          <CodeBlock code={claudeSettings} label="Claude Code permissions" lang="json" />
          <p>
            Run reports non-interactively in scripts or CI-style jobs with print mode. Authenticate
            interactively once first; the stored OAuth token is reused. List only the read-only tools
            so an unattended run cannot call <code>setup_site</code>.
          </p>
          <CodeBlock code={claudeHeadless} label="Claude Code headless report" />
        </article>

        <article id="codex" class="client">
          <h3>Codex</h3>
          <p>Run these commands with the Codex CLI:</p>
          <CodeBlock code={connectCommand} label="connection commands" />
          <p>
            The browser shows the agent name and requested permissions. Enter your Slimlytics login and
            select <strong>Log in and authorize</strong>. Restart your agent session to load the tools.
            Your password stays in the browser login flow.
          </p>
          <p>
            In the Codex IDE, add a Streamable HTTP server in MCP settings and select Authenticate. See
            the <a href="https://learn.chatgpt.com/docs/extend/mcp?surface=cli">official MCP guide</a>
            for more connection details.
          </p>
        </article>

        <article id="hermes" class="client">
          <h3>Hermes Agent</h3>
          <p>Add Slimlytics to <code>~/.hermes/config.yaml</code>:</p>
          <CodeBlock code={hermesConfig} label="Hermes config" lang="yaml" />
          <p>
            Then authorize and test the connection. Hermes prints an authorize URL, opens your browser,
            and waits for the OAuth callback on a local loopback port. Inside a running session, use
            <code>/reload-mcp</code> to pick up config changes.
          </p>
          <CodeBlock code={hermesCommands} label="Hermes commands" />
          <p>
            For an analyst-style assistant that should never create or change sites, expose only the
            reporting tools. Pair this with read-only scopes (see
            <a href="#permissions">permissions</a>).
          </p>
          <CodeBlock code={hermesReadOnly} label="Hermes read-only config" lang="yaml" />
        </article>

        <article id="other-clients" class="client">
          <h3>Other clients</h3>
          <p>
            Any agent that supports remote MCP OAuth with dynamic client registration can use the same
            endpoint. Clients that support only static bearer tokens can send a scoped personal API
            token instead — for example, in Claude Code:
          </p>
          <CodeBlock code={claudeToken} label="bearer token command" />
          <p>
            For your own Slimlytics deployment, replace <code>https://slimlytics.com</code> with its
            public HTTPS origin.
          </p>
        </article>
      </div>
    </section>

    <section class="mkt-section mkt-section-alt" aria-labelledby="ask-title">
      <div class="mkt-prose">
        <h2 id="ask-title">2. Ask for analytics installation</h2>
        <p>Open your website project in the agent and provide its production domain and hosting setup:</p>
        <blockquote>{prompt}</blockquote>
        <p>
          The <code>setup_site</code> tool creates or reuses the site by domain. It returns a script tag,
          exact proxy routes, and verification URLs. The default proxy type is Caddy; your agent can
          choose Nginx or Apache. Repeating setup reuses the same site.
        </p>
        <p>
          Your agent applies the returned configuration to the website using its normal repository and
          deployment tools. It needs access to your hosting configuration to finish deployment.
          Slimlytics does not independently deploy an unrelated website.
        </p>
        <p>
          For framework or edge hosting, have your agent implement the equivalent two exact server
          routes. Forward to the fixed Slimlytics origin, preserve the method, body, content type,
          Origin, Referer, and User-Agent, strip Cookie and Authorization, and remove upstream
          Set-Cookie. Avoid caching collection responses or accepting arbitrary upstream URLs.
        </p>
        <h3 id="prompts">Prompt library</h3>
        <p>These work the same in Claude Code, Codex, and Hermes once the server is connected.</p>
      </div>
      <div class="prompt-grid">
        {#each promptLibrary as item}
          <article>
            <h4>{item.title}</h4>
            <p>{item.body}</p>
          </article>
        {/each}
      </div>
    </section>

    <section class="mkt-section" aria-labelledby="verify-title">
      <div class="mkt-prose">
        <h2 id="verify-title">3. Verify first-party delivery</h2>
        <ol class="docs-steps">
          <li>Install both proxy routes before adding the returned script tag. Validate the server configuration and reload it.</li>
          <li>Open <code>scriptTestUrl</code>; expect JavaScript with HTTP 200.</li>
          <li>Open <code>beaconTestUrl</code>; expect HTTP 200 with <code>{'{"status":"ok"}'}</code>. This check does not insert an event.</li>
          <li>Load a website page and confirm the tracker and collection requests use your website’s own origin.</li>
          <li>Confirm a page view in the dashboard or MCP reports, then check consent, SPA navigation, and unrelated application routes.</li>
        </ol>
        <p>
          First-party anti-adblock delivery improves reliability while preserving consent, DNT, and GPC.
          It remains cookieless and cannot guarantee that every blocker permits tracking. For
          consent-controlled websites, insert the script after consent using the site’s existing consent
          mechanism.
        </p>
        <h2>Available tools</h2>
        <div class="table-wrap">
          <table>
            <thead><tr><th scope="col">Tool</th><th scope="col">Use</th></tr></thead>
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
        <p>
          Add this instruction to your website’s <code>AGENTS.md</code> (read by Codex and Hermes) or
          <code>CLAUDE.md</code> (read by Claude Code):
        </p>
        <CodeBlock
          label="agent instructions"
          lang="md"
          code={`When asked to add analytics, use the Slimlytics MCP server.
Call setup_site with the production domain, name, timezone,
and available proxy serverType. Install both same-origin routes
and the returned script. Preserve consent, DNT, and GPC.
Verify both test URLs and a page view. Reuse the site on retries.
Keep account tokens and server ingestion keys out of browser code.
Report any deployment step that still needs operator access.`}
        />
      </div>
    </section>

    <section class="mkt-section mkt-section-alt" aria-labelledby="permissions">
      <div class="mkt-prose">
        <h2 id="permissions">Permissions and reconnecting</h2>
        <p>
          Default permissions are <code>sites:read sites:write analytics:read</code>. Site setup also
          requires owner or administrator access. Read-only agents can request
          <code>sites:read analytics:read</code>; they cannot create or configure sites.
        </p>
        <p>
          OAuth connections appear as <strong>MCP OAuth agent</strong> in your account’s API token
          settings. Revoke one there to disconnect it immediately. Access tokens expire after 30 days;
          reconnect with <code>/mcp</code> in Claude Code, <code>codex mcp login slimlytics</code>, or
          <code>hermes mcp login slimlytics</code>. Refresh tokens are not issued.
        </p>
        <p>
          Clients that support only static bearer tokens can use scoped personal API tokens. Keep
          account tokens, passwords, and server ingestion keys out of committed configuration and
          browser code. Collection keys in the proxy configuration grant ingestion access only.
        </p>
        <h2>Self-hosting and troubleshooting</h2>
        <p>
          Deploy the backend and frontend together, and set <code>SLIMLYTICS_BASE_URL</code> to the
          public HTTPS origin. Database migrations run at backend startup. The included Caddy
          configurations route OAuth discovery to the backend.
        </p>
        <p>
          With another proxy, route <code>/api/*</code>, <code>/.well-known/oauth-authorization-server</code>,
          and <code>/.well-known/oauth-protected-resource*</code> to the backend. Keep <code>/p/*</code>
          on the frontend for the tracker bootstrap. A 404 from OAuth discovery means the deployment or
          proxy routes need updating.
        </p>
        <p>
          From a source checkout, <code>./scripts/connect-agent.sh https://your-analytics-domain</code>
          runs the Codex connection commands. The server uses stateless Streamable HTTP; an
          unauthenticated POST advertises browser OAuth with HTTP 401. GET returns 405 because
          server-to-client SSE is not offered.
        </p>
      </div>
    </section>
  </main>
  <MarketingFooter />
</div>

<style>
  .docs-nav {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 22px;
    margin-top: 26px;
  }
  .docs-nav a {
    color: var(--m-accent);
    font-weight: 600;
    text-underline-offset: 3px;
  }
  .docs-body {
    padding-top: 8px;
  }
  .client-tabs {
    position: sticky;
    top: 70px;
    z-index: 5;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 8px 0 28px;
    padding: 6px;
    border: 1px solid var(--m-line);
    border-radius: 999px;
    background: color-mix(in srgb, var(--m-surface) 92%, transparent);
    backdrop-filter: blur(12px);
  }
  .client-tabs a {
    flex: 1 1 auto;
    padding: 9px 14px;
    border-radius: 999px;
    text-align: center;
    color: var(--m-text);
    font-weight: 600;
    font-size: 14px;
    text-decoration: none;
  }
  .client-tabs a:hover {
    background: var(--m-accent-soft);
    color: var(--m-accent);
  }
  .client {
    scroll-margin-top: 140px;
    padding: 28px 0 8px;
    border-top: 1px solid var(--m-line);
  }
  .client h3 {
    margin-top: 0;
    font-size: 22px;
  }
  blockquote {
    margin: 0 0 22px;
    padding: 20px 24px;
    border-left: 3px solid var(--m-accent);
    border-radius: 0 14px 14px 0;
    background: var(--m-surface);
    line-height: 1.7;
  }
  .prompt-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 320px), 1fr));
    gap: 14px;
    margin-top: 8px !important;
  }
  .prompt-grid article {
    padding: 22px;
    border: 1px solid var(--m-line);
    border-radius: 16px;
    background: var(--m-surface);
    box-shadow: var(--m-shadow);
  }
  .prompt-grid h4 {
    margin: 0 0 8px;
    font: 750 16px var(--m-display);
  }
  .prompt-grid p {
    margin: 0;
    color: var(--m-muted);
    font-size: 15px;
    line-height: 1.65;
  }
  .mkt-prose h2:not(:first-child) {
    margin-top: 40px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 15px;
    line-height: 1.6;
  }
  th,
  td {
    padding: 12px;
    text-align: left;
    vertical-align: top;
    border-top: 0;
    border-bottom: 1px solid var(--m-line);
    font-size: 15px;
  }
  thead th {
    font-size: 12px;
    color: var(--m-muted);
  }
  td:first-child {
    white-space: nowrap;
  }
</style>
