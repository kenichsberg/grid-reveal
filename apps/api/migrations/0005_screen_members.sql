-- Explicit membership. A screen is invisible to everyone else: requests from
-- non-members answer 404 rather than 403 so the slug's existence stays private.
CREATE TABLE screen_members (
    screen_id UUID        NOT NULL REFERENCES screens (id) ON DELETE CASCADE,
    user_id   UUID        NOT NULL REFERENCES users (id)   ON DELETE CASCADE,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (screen_id, user_id)
);

CREATE INDEX screen_members_user_id_idx ON screen_members (user_id);
