-- Logged time. `occurred_at` is a calendar date chosen by the member, so no
-- timezone conversion happens anywhere. Attribution replays by `created_at`,
-- which means backdating never reshuffles who revealed which tile.
-- Deletion is soft so per-tile attribution stays auditable.
CREATE TABLE entries (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    screen_id   UUID        NOT NULL REFERENCES screens (id) ON DELETE CASCADE,
    user_id     UUID        NOT NULL REFERENCES users (id)   ON DELETE CASCADE,
    minutes     INTEGER     NOT NULL,
    occurred_at DATE        NOT NULL,
    note        TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    edited_at   TIMESTAMPTZ,
    deleted_at  TIMESTAMPTZ,
    CONSTRAINT entries_minutes_range CHECK (minutes BETWEEN 1 AND 1439)
);

-- The hot path: sum and replay a screen's live entries in attribution order.
CREATE INDEX entries_screen_created_idx
    ON entries (screen_id, created_at) WHERE deleted_at IS NULL;

CREATE INDEX entries_screen_user_idx
    ON entries (screen_id, user_id, occurred_at DESC) WHERE deleted_at IS NULL;
