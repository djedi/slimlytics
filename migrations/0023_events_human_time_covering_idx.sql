-- no-transaction
-- Lets the overview and all-sites summary count human visitors, sessions, and page views
-- with index-only scans. Built concurrently so collection keeps writing during the build.
CREATE INDEX CONCURRENTLY IF NOT EXISTS events_site_human_time_idx
  ON events(site_id, occurred_at) INCLUDE (visitor_id, session_id, event_name)
  WHERE traffic_class = 'human';
