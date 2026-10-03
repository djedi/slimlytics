-- Long-lived sign-in sessions, passkeys (WebAuthn), and a platform admin role.

ALTER TABLE users
  -- Granted only out of band (scripts/admin-grant.sh), never through the API.
  ADD COLUMN is_admin boolean NOT NULL DEFAULT false,
  ADD COLUMN disabled_at timestamptz,
  ADD COLUMN last_login_at timestamptz;

-- One row per signed-in browser/device. The refresh token is opaque and stored only as a
-- SHA-256 hash; it rotates on every use.
CREATE TABLE user_sessions (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  refresh_token_hash bytea NOT NULL UNIQUE,
  auth_method text NOT NULL CHECK (auth_method IN ('password', 'passkey')),
  -- Set when this session proved possession of a passkey; admin access requires it.
  mfa_verified_at timestamptz,
  user_agent text,
  created_at timestamptz NOT NULL DEFAULT now(),
  last_used_at timestamptz NOT NULL DEFAULT now(),
  expires_at timestamptz NOT NULL,
  revoked_at timestamptz
);
CREATE INDEX user_sessions_user_idx ON user_sessions(user_id) WHERE revoked_at IS NULL;

-- Every rotated-out refresh token, kept for the session's lifetime: replaying any of them
-- (outside a short grace window for racing tabs) revokes the session.
CREATE TABLE user_session_used_tokens (
  token_hash bytea PRIMARY KEY,
  session_id uuid NOT NULL REFERENCES user_sessions(id) ON DELETE CASCADE,
  used_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX user_session_used_tokens_session_idx ON user_session_used_tokens(session_id);

CREATE TABLE user_passkeys (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  name text NOT NULL CHECK (char_length(name) BETWEEN 1 AND 100),
  credential_id bytea NOT NULL UNIQUE,
  -- Serialized webauthn-rs Passkey: public key, algorithm, and signature counter.
  passkey jsonb NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  last_used_at timestamptz
);
CREATE INDEX user_passkeys_user_idx ON user_passkeys(user_id);

-- Short-lived, single-use WebAuthn ceremony state (challenge + expected parameters).
CREATE TABLE webauthn_challenges (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id uuid REFERENCES users(id) ON DELETE CASCADE,
  -- For step-up challenges: the session that will be marked as MFA-verified.
  session_id uuid REFERENCES user_sessions(id) ON DELETE CASCADE,
  kind text NOT NULL CHECK (kind IN ('register', 'login', 'step_up')),
  state jsonb NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  expires_at timestamptz NOT NULL
);
CREATE INDEX webauthn_challenges_expiry_idx ON webauthn_challenges(expires_at);

-- Every admin portal action. The target email is copied so entries survive user deletion.
CREATE TABLE admin_audit_log (
  id bigserial PRIMARY KEY,
  actor_user_id uuid REFERENCES users(id) ON DELETE SET NULL,
  actor_email text NOT NULL,
  action text NOT NULL,
  target_user_id uuid,
  target_email text,
  metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX admin_audit_log_created_idx ON admin_audit_log(created_at DESC);
