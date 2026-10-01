CREATE TABLE oauth_clients (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
 name text NOT NULL,
 redirect_uris text[] NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE oauth_codes (
 code_hash bytea PRIMARY KEY,
 client_id uuid NOT NULL REFERENCES oauth_clients(id) ON DELETE CASCADE,
 user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 redirect_uri text NOT NULL,
 challenge text NOT NULL,
 scopes text[] NOT NULL,
 expires_at timestamptz NOT NULL DEFAULT now() + interval '5 minutes'
);
ALTER TABLE api_tokens ADD COLUMN oauth_resource text;
