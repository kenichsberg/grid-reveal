## Why

A screen's active period is optional in three places at once: both date columns
are nullable, both DTO fields are `Option<NaiveDate>`, and `Screen::phase`
treats a missing bound as "no bound". Every one of those nulls has to be
handled, and almost every handling of it is a branch that says "this screen is
open forever".

That was the right shape when periods were added to a product that already had
screens without them. It is the wrong shape now:

- **The open-ended screen is not a feature anyone wants.** A screen exists to be
  uncovered by a group working against a deadline. A screen with no end never
  ends, never archives itself, and never becomes read-only — it just accumulates.
- **Nullability costs more than it earns.** `phase`, `covers`, `auto_archive_on`
  and `validate_window` each branch on absence. `validate_window` carries a rule
  that exists only because of it: an auto-archive term needs an end date to
  count from. The web client carries the same branching in `screen.ts`, where
  three of its labels have a "no date" fallback.
- **The next change depends on it.** `add-screen-invite-links` ties an invite's
  lifetime to its screen's period: an invite dies when the period ends. On a
  screen with no end date, that invite is immortal — a single-use URL that stays
  valid forever is exactly what a single-use URL must not be.

Nothing is live yet, so this is the last cheap moment to make the period part of
what a screen *is* rather than something it may have.

## What Changes

- **BREAKING**: `starts_on` and `ends_on` become NOT NULL on `screens`. A screen
  cannot be created, cloned, or edited into a state without both dates.
- Creating a screen requires both dates. The browser pre-fills the start date
  with the administrator's own day, so the common case is still one field to
  fill in.
- Editing a screen may move either date but may not remove one. The PATCH body's
  ability to clear a bound by sending `null` becomes a 400.
- `Screen::phase`, `covers` and `auto_archive_on` lose their absence branches.
  `validate_window` loses the rule about an auto-archive term needing an end
  date, which becomes unrepresentable.
- The wire types stop being nullable: `starts_on` and `ends_on` are plain dates
  in `ScreenDetail`, `CreateScreenRequest`, `CloneScreenRequest` and
  `ScreenWindow`, and `ends_on` likewise in `ScreenSummary`.
- `ScreenSummary` and `AdminScreenEntry` gain `starts_on`. The list already
  wants to say "Starts 3 Mar" and currently cannot, because the summary does not
  carry the date; the home page passes a literal `null` to work around it.
- The client's "no date" label fallbacks go, along with the control that cleared
  a screen's window.

Out of scope: changing what a period *means* once set, the auto-archive term
(which stays optional), and per-screen timezones. This change makes a period
mandatory; it does not change how one behaves.

## Capabilities

### Modified Capabilities

- `screen-lifecycle`: a screen's active period becomes mandatory rather than
  optional, and the phase machinery loses the windowless case.
- `screens`: creating, cloning and editing a screen all require a period.
- `progress-entries`: every screen now bounds the dates an entry may claim, so
  the entry form's date picker is always bounded.

## Impact

- `apps/api/migrations/` — a new migration adding NOT NULL, with a backfill for
  any rows that predate it.
- `apps/api/src/models.rs` — `Screen.starts_on` / `ends_on` become
  `NaiveDate`; `phase`, `covers` and `auto_archive_on` simplify; the tests that
  construct a windowless screen go.
- `apps/api/src/routes/screens.rs` — `validate_window`'s signature and one of
  its rules; create, update and clone.
- `apps/api/src/dto.rs` — five DTOs stop being nullable, two gain `starts_on`,
  and `ScreenWindow` loses its `Default` derive, which `NaiveDate` cannot
  satisfy.
- `apps/api/src/access.rs` — tests only; the production code already takes the
  dates as given.
- `apps/web/src/lib/screen.ts` — the label fallbacks for a missing date.
- `apps/web/src/routes/admin/screens/new/+page.svelte` — both dates required,
  the start date pre-filled with today.
- `apps/web/src/routes/admin/screens/[slug]/+page.svelte` — the edit form may no
  longer clear the window.
- `apps/web/src/routes/+page.svelte` — the `starts_on: null` workaround.
- `scripts/gen-types.sh` — regenerated TypeScript, since DTO shapes change.
