# Design note: public read-only dashboards (live demo)

Status: **proposal**, not implemented. Written for launch prep (Oct 2026).

## Why

Launch prep needs a credible live demo: a public link where prospects can see a real Slimlytics
dashboard without signing up, the way Plausible shows its own site's stats. Today the
homepage hero uses hard-coded sample numbers ("3,421 visitors", "14 online now", `/blog/launch`),
and there is no way to share a dashboard publicly.

## What exists today

- **No share or public-link feature.** Every `/api/sites/{id}/…` read route calls
  `require_site(…)` with an authenticated `CurrentUser`. Nothing in the schema
  (`migrations/0001`–`0022`) marks a site as public.
- **`PUBLIC_DEMO_MODE=true`** (frontend env) makes the whole frontend serve fabricated demo sites
  (`demoSites`/`demoReport` in `frontend/src/lib/api.ts`) and shows an "Explore demo" button.
  It is a whole-deployment switch with fake data, not a share link.

## Option A: per-site public dashboard (recommended)

**Backend (Rust):**
1. Migration: `ALTER TABLE sites ADD COLUMN public_slug text UNIQUE` (NULL = private). Owners set
   it from site settings (`PUT /api/sites/{id}` or a dedicated `PUT /api/sites/{id}/public`).
2. Unauthenticated, aggregate-only routes that resolve `public_slug → site_id` and reuse the
   existing helpers (`bounds`, `counts`, the report queries):
   - `GET /api/public/{slug}/overview?from&to`: visitors, page views, bounce, duration, trend,
     and the current-online *count*.
   - `GET /api/public/{slug}/reports/{pages|referrers|countries|devices|campaigns}`, capped
     at the top 10 rows.
3. **Never exposed publicly:** visitors and visitor timelines, the Spy SSE stream (only the
   online count), custom event properties, CSV export, insights/briefs, goals config, write keys.
4. Rate-limit by IP and cache responses for about 60 s so a Hacker News spike can't load PostgreSQL.
5. Tests: a public slug returns aggregates; an unknown/NULL slug returns 404; private routes still
   require auth; a site owner's paths under excluded prefixes don't leak.

**Frontend (SvelteKit):**
- New SSR route `/share/[slug]` (or `/live/slimlytics.com`) that reuses `TrafficChart` and
  `ReportTable` in read-only mode, with a "Powered by Slimlytics · Start free" banner.
- Site settings: a "Public dashboard" toggle that shows the share URL.

**Effort:** about 1 to 2 focused days with tests (one migration, about 3 handlers, one settings
control, one page). It touches auth boundaries, so it needs Dustin's review.

### Privacy and product caveats
- **Paths can leak.** Public top-pages lists show every path. slimlytics.com's own stats are
  dominated by Dustin's `/app` and `/admin` usage (see issue #58), so the public view needs a
  path-exclusion list per public dashboard, or it should only show marketing paths.
- **The numbers are small today.** slimlytics.com had about 65 visitors in the last 90 days.
  A public dashboard of its own stats would currently be *anti*-social-proof. Ship the feature,
  but link it prominently only after launch traffic arrives, or demo a busier dogfooded site
  with its owner's consent.

## Option B: demo deployment (no code)

Run a second frontend container with `PUBLIC_DEMO_MODE=true` at e.g. `demo.slimlytics.com`.
It costs zero code and shows the full UI (including Spy and visitors), but the data is fabricated,
so it is weaker proof than real numbers. It's useful as a stopgap for the launch post.

## Hero numbers

Keep the hero's sample numbers for now, because there is no public data source to read
from, and real slimlytics.com numbers would be too small to help. Consider marking the preview as
"Sample data" and, once Option A ships, adding a "See our live dashboard →" link next to it
rather than wiring real numbers into the hero.
