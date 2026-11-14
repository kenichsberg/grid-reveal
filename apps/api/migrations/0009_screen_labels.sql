-- Per-screen wording.
--
-- The board's progress heading and the name of one logged session were fixed
-- strings in the client, which tied the whole app to a single practice. Both
-- are nullable: NULL means "use the client's neutral default", which is exactly
-- how every screen created before this migration reads.
--
-- Blank is not a value. The API trims an empty string to NULL rather than
-- storing whitespace, so there is one representation of "unset" and the
-- fallback cannot be defeated by a stray space.
ALTER TABLE screens
    ADD COLUMN progress_title TEXT,
    ADD COLUMN entry_noun     TEXT;

ALTER TABLE screens
    ADD CONSTRAINT screens_progress_title_shape
        CHECK (progress_title IS NULL
               OR (btrim(progress_title) <> '' AND char_length(progress_title) <= 60)),
    ADD CONSTRAINT screens_entry_noun_shape
        CHECK (entry_noun IS NULL
               OR (btrim(entry_noun) <> '' AND char_length(entry_noun) <= 40));
