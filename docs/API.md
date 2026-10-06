# API overview

Interactive Scalar reference: `https://slimlytics.com/api/docs`

OpenAPI 3.1 JSON: `https://slimlytics.com/api/openapi.json`

The OpenAPI document is the machine-readable public contract and covers every authentication, account-token, site/settings, collection, reporting, goal, export, real-time, and first-party tracker route.

Account, token, site, goal, and collection request/response objects use camelCase fields. Reporting schemas retain their documented wire names where the backend returns fields such as `change_percent` or `occurred_at`. Authenticated requests send an `Authorization: Bearer <token>` header. Error responses use a stable machine-readable code and a human-readable message.

## System

- `GET /health` — process liveness
- `GET /ready` — PostgreSQL readiness

## Authentication

- `POST /api/auth/register`
- `POST /api/auth/login`
- `POST /api/auth/refresh` — exchange a refresh token for a new access token and a rotated refresh token
- `POST /api/auth/logout` — end the session named by the Bearer token and/or `{ "refreshToken": "..." }` (works after the access token expires)
- `POST /api/auth/passkey/start`, `POST /api/auth/passkey/finish` — usernameless passkey sign-in
- `POST /api/auth/mfa/start`, `POST /api/auth/mfa/finish` — verify a passkey from an existing session
- `GET /api/auth/me` — includes `isAdmin`, `passkeyCount`, and `mfaVerified`

Passwords are hashed with Argon2. Sign-in returns `{ token, refreshToken, expiresIn }`. The access token is a short-lived JWT signed with `JWT_SECRET` (`ACCESS_TOKEN_TTL_SECONDS`, one hour by default) and bound to a server-side session, so signing out, revoking a device, or disabling the account ends it immediately. Access tokens without a session (issued before sessions existed) are refused. The `slrt_...` refresh token is stored only as a SHA-256 digest and is single use: each refresh returns a new one. A session stays signed in for 30 days after its last refresh and at most 365 days. Every rotated-out refresh token is remembered for the session's lifetime; replaying any of them more than a minute after it was used is treated as theft and ends the session.

Passkeys use WebAuthn with the server's `SLIMLYTICS_BASE_URL` as the relying party, so they only work when the dashboard is opened at that origin. Signing in with a passkey, or verifying one from a password session, marks the session as MFA-verified for 12 hours.

## Account security

- `GET /api/account/sessions`, `DELETE /api/account/sessions/{sessionId}` — signed-in devices
- `GET /api/account/passkeys`, `DELETE /api/account/passkeys/{passkeyId}`
- `POST /api/account/passkeys/register/start`, `POST /api/account/passkeys/register/finish`

These accept browser sessions only. The first passkey requires `currentPassword`. Once any passkey exists, adding or removing one requires an MFA-verified session, so a stolen password alone can never replace someone's passkeys. Passkeys are created as discoverable credentials so they work for usernameless sign-in. Someone who loses every passkey signs in with their password and an operator runs `scripts/passkey-reset.sh EMAIL` after confirming their identity.

## Admin

- `GET /api/admin/overview`, `GET /api/admin/users?q=&limit=&offset=`, `GET /api/admin/users/{userId}`
- `POST /api/admin/users/{userId}/disable`, `/enable`, `/revoke-sessions`
- `DELETE /api/admin/users/{userId}` with `{ "confirmEmail": "..." }`
- `GET /api/admin/audit`

Admin routes need an admin account **and** a session that verified a passkey within 12 hours; otherwise they return `403` with error code `mfa_required`. API tokens are always refused. Admin status is granted only out of band with `scripts/admin-grant.sh EMAIL` (or `--revoke`), never through the API. Admins cannot change their own account or other admins from the portal. Disabling an account revokes its sessions, API tokens, MCP connections, and pending MCP authorization codes, and OAuth token exchange refuses disabled accounts. Deleting one also deletes sites it alone owns and is refused while a Stripe subscription is active. Every action is written to the admin audit log.

## Account API tokens

- `POST /api/account/tokens` — create a personal API token using a session JWT
- `GET /api/account/tokens` — list active token metadata
- `DELETE /api/account/tokens/{tokenId}` — immediately revoke a token
- `DELETE /api/account/tokens/current` — revoke the personal token authenticating this request

Creation accepts `{ "name": "slimlytics-cli", "expiresInDays": 365 }`. The response contains the `slyt_...` secret exactly once. Slimlytics stores only a SHA-256 digest of the 256-bit random secret, and list responses expose only a short prefix and timestamps. Tokens expire after 365 days by default; accepted bounds are 1–3650 days. A personal token can use account and site APIs but cannot mint another token—creating one requires a password-authenticated session JWT.

API tokens and JWTs use the same Bearer header. Revoked or expired tokens return `401`. See `CLI.md` for the supported client and agent workflow.

## Sites

- `GET /api/sites`
- `POST /api/sites`
- `POST /api/sites/ensure` — atomically create or reuse a canonical domain
- `GET /api/sites/{siteId}`
- `PUT /api/sites/{siteId}`
- `PUT /api/sites/{siteId}/anti-adblock`
- `DELETE /api/sites/{siteId}`

A site has a display name, canonical URL, timezone, allowed origins, retention policy, status, independently rotatable collection write key, and a persisted anti-adblock server type, JavaScript path, and beacon path. New sites receive random neutral path defaults. Domains are canonicalized case-insensitively and globally unique; an account cannot claim a domain already managed by another account. `ensure` returns `{ "created": boolean, "site": {...} }` and is safe for retrying agents. The anti-adblock update body is `{ "serverType": "caddy|nginx|apache", "jsPath": "/...js", "beaconPath": "/..." }`.

## Collection

- `POST /api/collect/{writeKey}`
- `GET /api/collect/{writeKey}` — non-ingesting proxy diagnostic
- `POST /api/e/{writeKey}` — neutral anti-adblock alias with identical behavior
- `GET /api/e/{writeKey}` — non-ingesting legacy-alias diagnostic

The browser tracker sends page views and custom events. The collector accepts `sendBeacon` bodies, applies origin checks, normalizes and redacts URLs, classifies bots (by product name and by published Google, Meta, and Bing crawler networks whose ad-review and preview fetchers send stock browser user agents) and internal traffic, derives site-scoped anonymous identifiers, deduplicates event IDs, and persists accepted events.

A write key authorizes ingestion only. It never grants dashboard or reporting access.

## First-party tracker bootstrap

- `GET /p/{writeKey}/{beaconName}`

Returns the complete JavaScript tracker plus a site initializer targeting the exact same-origin `/{beaconName}` path. The bundle is embedded in the adapter-node build rather than loaded from the runtime working directory. Invalid keys or path names return `400`. See `FIRST_PARTY_PROXY.md` for the dashboard-generated server configurations.

## Reporting

- `GET /api/sites/{siteId}/overview?from=2026-07-01&to=2026-07-28`
- `GET /api/sites/{siteId}/reports/pages`
- `GET /api/sites/{siteId}/reports/referrers`
- `GET /api/sites/{siteId}/reports/countries`
- `GET /api/sites/{siteId}/reports/devices`
- `GET /api/sites/{siteId}/reports/campaigns`
- `GET /api/sites/{siteId}/visitors`
- `GET /api/sites/{siteId}/events`

Overview, reports, visitors, events, and export accept inclusive `from` and `to` dates. Reports also accept a bounded `limit`. The overview includes the prior equivalent period for percentage comparisons.

## Goals

- `GET /api/sites/{siteId}/goals`
- `POST /api/sites/{siteId}/goals`

Version 1 goals match an event name and an optional SQL-LIKE path pattern. This covers explicit custom-event goals and page-path goals without collecting additional identity data.

## Export

- `GET /api/sites/{siteId}/export.csv`

The export uses the caller’s site authorization and report filters. Content-Disposition provides a safe filename.

## Real time

- `GET /api/sites/{siteId}/stream?token={accessToken}`

The response is `text/event-stream`. Native browser `EventSource` supplies the short-lived access token as a query parameter because it cannot set an Authorization header. Events include monotonically useful event IDs, typed JSON payloads, and heartbeat comments. Browsers may reconnect with `Last-Event-ID`; clients reconcile against durable overview/report endpoints after reconnection.

## Tracker API

The global tracker provides:

- `init(options)`
- `page(properties?)`
- `event(name, properties?)`
- `consent(granted)`
- `flush()`

Initialization options include the collector endpoint, site write key, batching interval, automatic SPA/page tracking, outbound/download tracking, DNT/GPC behavior, and initial consent state.

### Ignore my visits

Open any page of the site with `#slimlytics-ignore` appended (for example `https://example.com/#slimlytics-ignore`) to stop counting that browser. The tracker stores `slimlytics_ignore=true` in that browser's `localStorage`, removes the fragment from the address bar, and sends nothing afterwards. Open `#slimlytics-ignore=off` to resume. The flag is only written when the site owner opens the link, so visitors are never affected, and fragments are never sent to the collector. The dashboard's site settings page has both links. `window.Slimlytics.isIgnored()` reports the current state.
