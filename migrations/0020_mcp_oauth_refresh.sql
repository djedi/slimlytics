-- An MCP OAuth connection is one api_tokens row. expires_at is the connection lifetime
-- (90 days, extended on every refresh) so idle connections stay listed and revocable;
-- access_expires_at bounds the current short-lived access token. Personal tokens leave it NULL.
ALTER TABLE api_tokens ADD COLUMN access_expires_at timestamptz;

-- Rotating refresh tokens. A used token is kept briefly so replaying it can be detected,
-- which revokes the whole connection (OAuth 2.1 section 4.3.1).
CREATE TABLE oauth_refresh_tokens (
  token_hash bytea PRIMARY KEY CHECK (octet_length(token_hash) = 32),
  api_token_id uuid NOT NULL REFERENCES api_tokens(id) ON DELETE CASCADE,
  client_id uuid NOT NULL REFERENCES oauth_clients(id) ON DELETE CASCADE,
  created_at timestamptz NOT NULL DEFAULT now(),
  used_at timestamptz
);
CREATE INDEX oauth_refresh_tokens_api_token_idx ON oauth_refresh_tokens(api_token_id);
CREATE INDEX oauth_refresh_tokens_client_idx ON oauth_refresh_tokens(client_id);

-- Registrations that never complete an authorization are pruned. Existing clients predate
-- this tracking and may hold live connections, so they count as used.
ALTER TABLE oauth_clients ADD COLUMN last_used_at timestamptz;
UPDATE oauth_clients SET last_used_at = now();

CREATE INDEX oauth_codes_expiry_idx ON oauth_codes(expires_at);
