-- A user's Porkbun API keys, encrypted with INTEGRATION_ENCRYPTION_KEY. Slimlytics uses them only
-- to create the DNS records that verify that user's sites with Google and Bing.
CREATE TABLE porkbun_credentials (
  user_id uuid PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  api_key_encrypted text NOT NULL,
  secret_key_encrypted text NOT NULL,
  api_key_hint text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);
