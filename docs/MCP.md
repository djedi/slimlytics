# Set up analytics with your agent

Slimlytics includes a remote MCP server in the normal backend. Connect it once, log in through your browser, and then ask your coding agent to install analytics in each website. No separate MCP process, password in agent configuration, or manually copied API token is required.

## Deploy

Deploy the normal Slimlytics stack and create an account. Set `SLIMLYTICS_BASE_URL` to the public HTTPS **origin**, for example `https://analytics.example.com`, and restart the backend. Migrations run at startup. The included Caddy configurations route OAuth discovery to the backend. If using another proxy, route `/api/*`, `/.well-known/oauth-authorization-server`, and `/.well-known/oauth-protected-resource*` to the backend; keep `/p/*` on the frontend.

The server URL is `https://analytics.example.com/api/mcp`. Replace the example origin in all commands with your deployment. These commands become usable after deploying this version.

## Connect Codex

From this checkout:

```sh
./scripts/connect-agent.sh https://analytics.example.com
```

Or run the commands directly:

```sh
codex mcp add slimlytics --url https://analytics.example.com/api/mcp
codex mcp login slimlytics
codex mcp list
```

The browser displays the agent name and requested permissions. Enter your Slimlytics login and select **Log in and authorize**. Restart the agent session to load its tools. Codex's CLI and IDE share MCP configuration; see the [official MCP guide](https://learn.chatgpt.com/docs/extend/mcp?surface=cli).

You can also use the IDE's MCP settings: add a Streamable HTTP server with this URL and select Authenticate. Agents that support remote MCP OAuth with dynamic client registration can use the same URL. A client that only supports static bearer tokens can use an existing scoped personal API token instead; see [AGENT_INTEGRATION.md](AGENT_INTEGRATION.md).

## Connect Claude Code

```sh
claude mcp add --transport http slimlytics https://analytics.example.com/api/mcp
```

Start `claude`, run `/mcp`, select **slimlytics**, and choose **Authenticate** to complete browser login. Verify with `claude mcp list` or `claude mcp get slimlytics`. Add `--scope user` to make the server available in every project, or `--scope project` to write a shareable `.mcp.json` (it holds only the URL; each teammate authenticates separately):

```json
{
  "mcpServers": {
    "slimlytics": { "type": "http", "url": "https://analytics.example.com/api/mcp" }
  }
}
```

Pre-approve read-only tools in `.claude/settings.json` and leave `setup_site` on manual approval:

```json
{
  "permissions": {
    "allow": [
      "mcp__slimlytics__list_sites",
      "mcp__slimlytics__analytics_summary",
      "mcp__slimlytics__dimension_report",
      "mcp__slimlytics__marketing_brief"
    ]
  }
}
```

Headless reports (authenticate interactively once first). Allow only the read-only tools so an unattended run cannot call `setup_site`:

```sh
claude -p "Summarize last week's traffic for shop.example.com with the Slimlytics MCP server." \
  --allowedTools "mcp__slimlytics__list_sites,mcp__slimlytics__analytics_summary,mcp__slimlytics__dimension_report,mcp__slimlytics__marketing_brief"
```

Static bearer token fallback: `claude mcp add --transport http slimlytics https://analytics.example.com/api/mcp --header "Authorization: Bearer $SLIMLYTICS_TOKEN"`.

## Connect Hermes Agent

Add the server to `~/.hermes/config.yaml`:

```yaml
mcp_servers:
  slimlytics:
    url: "https://analytics.example.com/api/mcp"
    auth: oauth
```

Then run `hermes mcp login slimlytics` and `hermes mcp test slimlytics`. Hermes prints an authorize URL, opens the browser, and waits for the callback on a loopback port. Use `/reload-mcp` inside a session after config changes. To expose only reporting tools, add:

```yaml
    tools:
      include: [list_sites, analytics_summary, dimension_report, marketing_brief]
```

## Ask for installation

Example:

> Set up Slimlytics analytics for this website at https://shop.example.com. Use first-party anti-adblock delivery. This app runs behind Nginx. Install the routes and tracker, preserve the site's consent policy, and verify collection.

For a predictable workflow, add this instruction to the website's `AGENTS.md`:

```text
When asked to add analytics, use the Slimlytics MCP server. Call setup_site
with the production domain, site name, timezone, and available proxy serverType.
Install the returned same-origin script and exact proxy routes. Preserve consent,
DNT, and GPC. Check both verification URLs and confirm a page view in analytics.
Reuse the existing site on retries. Keep account tokens and server ingestion keys
out of browser code. Report any deployment step that still needs operator access.
```

Claude Code reads `CLAUDE.md`; the same instruction works there.

More example prompts (any client):

- "Add Slimlytics to this SvelteKit app on Vercel. Call setup_site for https://blog.example.com, implement the two returned proxy paths as server routes, add the script to the root layout, and verify both test URLs on a preview deploy."
- "Summarize traffic for shop.example.com from last Monday through Sunday, compare with the previous week, and call out the biggest page and referrer changes."
- "Pull the campaigns report for the last 14 days. Which utm_campaign values reached the signup goal?"
- "Get yesterday's marketing_brief for shop.example.com and turn it into three actions with supporting numbers."
- "Compare search_console_report queries with the pages report for 28 days and flag high-impression, low-CTR queries."
- "List every site I can access, fetch tracking_setup for each, and check scriptTestUrl and beaconTestUrl."

`setup_site` returns whether the site was created and a `setup` object containing `siteId`, `serverType`, `serverConfig`, `snippet`, the JavaScript and beacon paths, and verification URLs. Repeating it for the same domain reuses the site. Existing timezone, retention, and origin settings are preserved; an explicit `serverType` changes only the proxy type, retaining its paths. Existing sites require administrator or owner access for setup.

The default proxy type is Caddy. Choose `serverType: "nginx"` or `"apache"` when applicable. `tracking_setup` retrieves the current installation for a known `siteId`; `list_sites` finds accessible sites. Reporting tools remain available after installation.

The MCP server configures Slimlytics and returns installation artifacts. Your coding agent applies them to the website repository and hosting configuration using its normal filesystem/deployment tools. The server cannot independently edit or deploy an unrelated website.

## First-party delivery

Install **both** routes in `serverConfig` on the measured website before adding `snippet`. One serves the initialized tracker from `/p/{writeKey}/{beacon}`; the other forwards collection to `/api/collect/{writeKey}`. Both browser requests stay on the measured website's origin. Caddy routes must precede a broad application fallback; Nginx exact locations belong inside the website's server block; Apache needs the listed modules. Validate the configuration before reloading.

For framework or edge hosting without these proxies, implement equivalent exact server routes for the returned paths. Forward to the fixed Slimlytics origin and the specified bootstrap/collection paths, preserve method/body/content type/Origin/Referer/User-Agent, strip Cookie and Authorization, and remove upstream Set-Cookie. Do not expose an arbitrary upstream URL or forward a browser-supplied X-Forwarded-For. Avoid caching collection responses. See [FIRST_PARTY_PROXY.md](FIRST_PARTY_PROXY.md) for routing and verification details.

Anti-adblock here means reliable first-party delivery. It remains cookieless and preserves the tracker's consent, DNT, and GPC behavior. It cannot guarantee that every blocker permits tracking. A consent-controlled website should insert the script only after consent, following its existing consent mechanism.

Verification:

1. Request `scriptTestUrl`; expect JavaScript with HTTP 200.
2. Request `beaconTestUrl`; expect HTTP 200 with `{"status":"ok"}` (no event inserted).
3. Load a page and check that script and collection use the website's origin.
4. Confirm a real page view in the dashboard or MCP reports.
5. Check consent and privacy signals, SPA navigation, and unrelated application routes.

## OAuth and operations

The native OAuth flow uses authorization codes, required S256 PKCE, exact registered redirect URIs, resource binding, CSRF protection on browser approval, and single-use five-minute codes. Public clients register automatically; HTTPS callbacks and HTTP loopback callbacks are accepted. No client secret is required.

Discovery:

- `GET /.well-known/oauth-protected-resource/api/mcp` (also at the root metadata path)
- `GET /.well-known/oauth-authorization-server`
- `POST /api/oauth/register`
- `GET|POST /api/oauth/authorize`
- `POST /api/oauth/token` (form-encoded)

Authorization and token requests must include `resource` equal to the configured MCP URL. The default scopes are `sites:read sites:write analytics:read`. Optional `integrations:read` enables Search Console reports. Scope checks and site membership checks apply independently. Read-only clients should request `sites:read analytics:read`; they cannot set up sites.

Access tokens are stored only as hashes, restricted to `/api/mcp`, and expire after 30 days. Refresh tokens are not issued: reconnect using browser login after expiry (`/mcp` in Claude Code, `codex mcp login slimlytics`, or `hermes mcp login slimlytics`). Revoke a connection immediately from the account's API token settings; OAuth connections appear as **MCP OAuth agent**. Agent calls use the existing audit log. Never put bearer tokens, passwords, or server ingestion keys in website code or committed MCP configuration. Collection keys in generated proxy configuration are public ingestion credentials, not account credentials.

The transport is stateless Streamable HTTP with JSON responses. It negotiates revisions `2025-03-26`, `2025-06-18`, and `2025-11-25`; initialization notifications return HTTP 202. GET returns 405 because this server does not offer a server-to-client SSE stream. An unauthenticated POST returns HTTP 401 with OAuth discovery in `WWW-Authenticate`.

To run the OAuth/database integration test against a disposable PostgreSQL database:

```sh
TEST_DATABASE_URL=postgres://... cargo test --manifest-path backend/Cargo.toml --test mcp_oauth -- --ignored
```
