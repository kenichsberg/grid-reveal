-- One link, one person, one screen.
--
-- There is deliberately no `expires_at`. An invite is open exactly while its
-- screen's period is upcoming or active, resolved per request from the
-- caller's own calendar day. A stored expiry would be a second deadline that
-- could disagree with the screen's: an invite still valid for a screen that
-- closed last week, or dead for one whose period was just extended. Extending
-- a screen's end date is how a closed screen is reopened, and it revives that
-- screen's unused invites for the same reason.
--
-- The token is stored as it is, like a session token, rather than hashed. A
-- leaked backup yields working sessions already; hashing one and not the other
-- would buy inconsistency rather than safety.
CREATE TABLE screen_invites (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    screen_id   UUID        NOT NULL REFERENCES screens (id) ON DELETE CASCADE,
    token       TEXT        NOT NULL UNIQUE,
    -- Who the administrator meant it for. Never shown to the invited person.
    label       TEXT,
    created_by  UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    accepted_at TIMESTAMPTZ,
    -- SET NULL rather than CASCADE: removing a member should not erase the
    -- record that this invite was spent.
    accepted_by UUID        REFERENCES users (id) ON DELETE SET NULL,
    revoked_at  TIMESTAMPTZ,
    CONSTRAINT screen_invites_label_len CHECK (char_length(label) <= 80),
    -- An accepting user implies a time, but not the reverse: ON DELETE SET
    -- NULL above can clear the user and leave the acceptance recorded.
    CONSTRAINT screen_invites_accepted_by_needs_a_time
        CHECK (accepted_by IS NULL OR accepted_at IS NOT NULL)
);

CREATE INDEX screen_invites_screen_idx ON screen_invites (screen_id, created_at DESC);
