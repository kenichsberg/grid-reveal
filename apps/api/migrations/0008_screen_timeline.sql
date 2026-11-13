-- A screen's active period.
--
-- All three columns are nullable: a screen without a window is open
-- indefinitely, which is exactly how every screen created before this
-- migration behaves. Nothing changes until an administrator sets a window.
--
-- Dates, not instants. The window is compared against `entries.occurred_at`,
-- which is itself a plain calendar date, so no timezone conversion enters
-- anywhere in the system.
ALTER TABLE screens
    ADD COLUMN starts_on         DATE,
    ADD COLUMN ends_on           DATE,
    -- Days after `ends_on` before the screen archives itself. NULL disables
    -- automatic archiving; 0 archives it the day after the period ends.
    ADD COLUMN auto_archive_days INTEGER;

ALTER TABLE screens
    ADD CONSTRAINT screens_window_ordered
        CHECK (ends_on IS NULL OR starts_on IS NULL OR ends_on >= starts_on),
    ADD CONSTRAINT screens_auto_archive_positive
        CHECK (auto_archive_days IS NULL OR auto_archive_days >= 0);
