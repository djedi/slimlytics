# Slimlytics review instructions

Loaded from the trusted base branch by the local AI reviewer.

- Report concrete correctness, privacy, security, data loss, race, and deployment defects. Explain the input or sequence that triggers each issue. Avoid style suggestions.
- Check account and site authorization independently. Token scopes do not replace site membership checks. OAuth codes must be short-lived, bound to client/redirect/resource and S256 PKCE, and consumed once. Keep account/server credentials out of public trackers and generated HTML.
- All SQL values must be parameterized. Dynamic identifiers must come from a fixed allowlist. Database migrations are append-only after release and must keep older stored rows readable.
- Collection is cookieless by default. Never persist raw IPs, form values, or sensitive URL parameters. Preserve consent, DNT, GPC, hashing/truncation, origin validation, and public-versus-server ingestion boundaries. First-party delivery must not override privacy choices.
- Reports must use site-local date boundaries, handle DST, and distinguish humans, bots, and internal traffic. Totals, sessions, goals, attribution, and real-time updates should reconcile with durable events. Check retries, batching, duplicate events, idempotency, and reconnect behavior.
- API JSON is camelCase. The frontend defaults to `/api`; trackers use a configured endpoint/write key. Check Svelte 5 reactivity, SSR/browser boundaries, authentication expiry, and failure states.
- Reverse proxy examples must use fixed validated upstreams and exact routes, strip browser credentials, and preserve required collection headers. Check Docker build inputs, Compose health/dependency ordering, backup/deployment behavior, and configuration defaults.
- CI optimizations must run every affected gate and preserve full main checks. The persistent runner must not execute untrusted fork code; the review workflow may only read the PR as text using trusted base-branch scripts.
- Anchor comments to lines present on the right side of the diff. Provide a suggestion only when it is a complete, correct replacement. Mark a pre-existing issue explicitly when the change depends on it or worsens it.
