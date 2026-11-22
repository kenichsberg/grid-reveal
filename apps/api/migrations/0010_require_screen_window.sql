-- Every screen has an active period.
--
-- 0008 made the timeline columns nullable, correctly: screens already existed
-- without a period, and a column that had to be filled would have had nothing
-- to fill it with. That reason is spent. An open-ended screen never ends,
-- never archives itself, and never becomes read-only, which is not a screen
-- anyone wants; and `auto_archive_days` can only count from an end date.
--
-- 0008 is deliberately left untouched rather than amended. `db::migrate` runs
-- `sqlx::migrate!`, which records a checksum for every applied migration and
-- refuses to start when one of them has changed. A database that has already
-- run 0008 would fail to boot with VersionMismatch.

-- Backfill before the constraint, because SET NOT NULL rejects an existing
-- NULL. No row is expected here: nothing is in production yet. The rule is
-- written to be defensible rather than convenient, so that a row it does meet
-- comes out read-only rather than open forever.
--
-- A screen was open from the day it was made, which is true by construction.
UPDATE screens SET starts_on = created_at::date WHERE starts_on IS NULL;

-- An end date it never had becomes its last day of activity, so the screen
-- reads as ended: read-only, with its board and history intact. Inventing a
-- date in the future would silently reopen a screen nobody has touched. An
-- administrator can extend the end date, which already makes a closed screen
-- writable again.
UPDATE screens s
SET ends_on = GREATEST(
        s.starts_on,
        COALESCE(
            (SELECT MAX(e.occurred_at)
             FROM entries e
             WHERE e.screen_id = s.id AND e.deleted_at IS NULL),
            s.starts_on))
WHERE s.ends_on IS NULL;

ALTER TABLE screens
    ALTER COLUMN starts_on SET NOT NULL,
    ALTER COLUMN ends_on SET NOT NULL;

-- The old constraint's two IS NULL arms are now dead weight.
ALTER TABLE screens
    DROP CONSTRAINT screens_window_ordered,
    ADD CONSTRAINT screens_window_ordered CHECK (ends_on >= starts_on);
