-- Preferred base language ("es", "pt") from the browser's Accept-Language header.
-- Nullable and without a default, so adding it is a metadata-only change on a large table.
-- Rows collected before this migration, and server-ingested events, stay NULL.
ALTER TABLE events ADD COLUMN IF NOT EXISTS language text;
