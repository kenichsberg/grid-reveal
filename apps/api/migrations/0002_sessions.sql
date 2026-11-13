-- Opaque server-side sessions. The token is the cookie value; there is no JWT,
-- so an administrator password reset can revoke access immediately.
CREATE TABLE sessions (
    token      TEXT PRIMARY KEY,
    user_id    UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX sessions_user_id_idx ON sessions (user_id);
CREATE INDEX sessions_expires_at_idx ON sessions (expires_at);
