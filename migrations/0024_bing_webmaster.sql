CREATE TABLE bing_webmaster_integrations (
  site_id uuid PRIMARY KEY REFERENCES sites(id) ON DELETE CASCADE,
  site_url text,
  api_key_encrypted text NOT NULL,
  connected_by uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  last_synced_at timestamptz,
  last_error text,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

-- Bing reports queries and pages separately, each in its own (roughly weekly) date buckets, and
-- site totals by day; `kind` keeps the three series apart so they are never summed together.
CREATE TABLE bing_webmaster_metrics (
  site_id uuid NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
  metric_date date NOT NULL,
  kind text NOT NULL CHECK (kind IN ('query','page','total')),
  value text NOT NULL DEFAULT '',
  clicks double precision NOT NULL,
  impressions double precision NOT NULL,
  position double precision,
  -- Impressions that `position` averages over; Bing marks unranked rows with no position.
  ranked_impressions double precision NOT NULL DEFAULT 0,
  synced_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY(site_id,kind,metric_date,value)
);
CREATE INDEX bing_webmaster_metrics_site_date_idx
  ON bing_webmaster_metrics(site_id,kind,metric_date DESC);
