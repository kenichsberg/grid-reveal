-- Free-text bullets any member may add. Authorship is disclosed only to
-- administrators; members see an unattributed list.
CREATE TABLE goals (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    screen_id  UUID        NOT NULL REFERENCES screens (id) ON DELETE CASCADE,
    text       TEXT        NOT NULL,
    created_by UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ,
    position   INTEGER     NOT NULL DEFAULT 0,
    CONSTRAINT goals_text_not_blank CHECK (btrim(text) <> ''),
    CONSTRAINT goals_text_len CHECK (char_length(text) <= 200)
);

CREATE INDEX goals_screen_idx
    ON goals (screen_id, position, created_at) WHERE deleted_at IS NULL;
