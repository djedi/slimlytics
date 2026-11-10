-- OAuth states are shared by Google and Bing sign-ins; each callback only consumes its own.
ALTER TABLE oauth_states ADD COLUMN provider text NOT NULL DEFAULT 'google';

-- A Bing connection authenticates with an API key or an OAuth refresh token, both stored
-- encrypted in api_key_encrypted. connection_id changes on every connect, so work started for one
-- connection (a sync, a verification) never lands on a later one, even after a token refresh.
ALTER TABLE bing_webmaster_integrations
  ADD COLUMN auth_kind text NOT NULL DEFAULT 'api_key' CHECK (auth_kind IN ('api_key','oauth')),
  ADD COLUMN connection_id uuid NOT NULL DEFAULT gen_random_uuid();
