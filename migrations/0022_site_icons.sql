-- Per-site avatar shown in the site picker and portfolio cards: initials on a colored tile,
-- or the website's own favicon. Colors are #rrggbb; NULL keeps the automatic per-domain hue.

ALTER TABLE sites
  ADD COLUMN icon_mode text NOT NULL DEFAULT 'initials' CHECK (icon_mode IN ('initials', 'favicon')),
  ADD COLUMN icon_background text CHECK (icon_background ~ '^#[0-9a-f]{6}$'),
  ADD COLUMN icon_background_end text CHECK (icon_background_end ~ '^#[0-9a-f]{6}$'),
  ADD COLUMN icon_foreground text CHECK (icon_foreground ~ '^#[0-9a-f]{6}$'),
  -- Bumped whenever a favicon is stored; the frontend uses it to bust the image cache.
  ADD COLUMN icon_updated_at timestamptz;

-- The fetched favicon, served back from /api/sites/{id}/icon so browsers never contact the
-- measured site from the dashboard.
CREATE TABLE site_icons (
  site_id uuid PRIMARY KEY REFERENCES sites(id) ON DELETE CASCADE,
  content_type text NOT NULL,
  body bytea NOT NULL,
  source_url text NOT NULL,
  fetched_at timestamptz NOT NULL DEFAULT now()
);
