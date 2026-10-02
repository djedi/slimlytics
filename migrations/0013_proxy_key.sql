-- Shared secret a site's first-party proxy sends with the visitor's IP. It only authorizes
-- forwarding the client IP (for location and visitor IDs); it grants no other access.
ALTER TABLE sites ADD COLUMN proxy_key uuid NOT NULL DEFAULT gen_random_uuid();
