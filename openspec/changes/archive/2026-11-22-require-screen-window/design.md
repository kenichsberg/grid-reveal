## Context

Migration `0008_screen_timeline.sql` added `starts_on`, `ends_on` and
`auto_archive_days`, all nullable, with a comment explaining why: screens
already existed, and a window had to be optional for them. That reasoning was
sound and is now spent — the screens it protected were development data, and the
product has not shipped.

The nullability is load-bearing in more places than the two columns suggest.
`Screen::phase` has two `is_some_and` guards, `covers` has the same two,
`auto_archive_on` is an `Option` chain over both `ends_on` and
`auto_archive_days`, and `validate_window` carries a cross-field rule that only
exists because `ends_on` can be absent. On the client, `screen.ts` has three
labels whose "no date" arm is reachable only for a windowless screen, and
`+page.svelte` passes a literal `starts_on: null` because `ScreenSummary` has no
such field.

`auto_archive_days` stays nullable. It is genuinely optional — a screen that
never archives itself is a real thing an administrator may want — and nothing in
this change touches it beyond the one validation rule it loses.

## Goals / Non-Goals

**Goals:**

- A screen always has both bounds, enforced at the database, not only above it.
- Fewer branches: every `Option` removed here is a case that can no longer be
  reached or tested.
- An administrator still creates a screen by filling in one date, not two.
- The next change can treat "the screen's period ends" as a date that always
  exists.

**Non-Goals:**

- Making `auto_archive_days` mandatory.
- Changing the inclusive semantics of either bound, or how phase is derived.
- A default end date. A start date has an obvious default — today — and an end
  date does not; guessing one would be worse than asking.
- Migrating real data. There is none. The backfill exists so the migration is
  well defined, not because it is expected to match a row.

## Decisions

### A new migration, not an edit to 0008

The instinct is to amend `0008` in place, since nothing depends on its current
form. That is unsafe here for a mechanical reason: `db.rs` runs
`sqlx::migrate!("./migrations")`, which records each migration's checksum in
`_sqlx_migrations` and refuses to start when an applied migration's content has
changed — `VersionMismatch`. The deployed API has run `0008`. Editing it would
turn the next deploy into a startup failure that has to be fixed by hand
against the production database, to save one file.

So: `0010_require_screen_window.sql`, additive and checksummed like the rest.
The cost is a migration whose whole content is a correction of an earlier one,
which is the ordinary cost of a migration history.

### The backfill picks dates that make a screen read-only, not immortal

`SET NOT NULL` fails on an existing NULL, so the migration must fill them first.
With no screens to migrate, any rule would do; that is exactly why the rule
should be the conservative one rather than the convenient one.

- `starts_on` → `created_at::date`. The screen was open from the day it was
  made, which is true by construction.
- `ends_on` → the later of `created_at::date` and the screen's last entry date.

A screen backfilled this way reads as *ended*: read-only, its board intact, its
history intact. The alternative — an end date some months out — would silently
reopen screens nobody has touched. If a row is ever backfilled in a real
database, the administrator can extend the end date, which already works and
already makes a closed screen writable again. Nothing is destroyed either way;
the dates are a bound, not data.

### Required in the request, defaulted in the browser

`CreateScreenRequest.starts_on` becomes a plain `NaiveDate`, so a request
without it is a 400 from serde before any handler sees it. The default comes
from the create form, pre-filled with `today()` from `$lib/format`.

Defaulting on the server was the alternative, and it is the wrong place for the
same reason the previous change moved the day to the caller: the server's idea
of today is UTC, and an administrator creating a screen at 00:30 would get a
start date a day behind the one shown in the field. The client already knows
its own day and already sends it.

### Clearing a bound becomes an error, not a silent no-op

`UpdateScreenRequest.window` is `Option<ScreenWindow>`: absent means "leave the
period alone", present means "replace it". That distinction survives. What goes
is the second level — `ScreenWindow.starts_on` being `Option`, where `null`
meant "clear this bound".

With the field non-optional, `{"window": {"ends_on": "2026-03-01"}}` fails
deserialization and returns 400, naming the missing field. Accepting it as a
partial update would be friendlier, but it would mean reintroducing the
`Option` this change exists to remove, and the client always has both dates in
hand because it loaded them to render the form.

`ScreenWindow` also loses `#[derive(Default)]`: `NaiveDate` has no `Default`, and
a default window was never meaningful.

### `auto_archive_on` stays an `Option`

It is `Option<NaiveDate>` today because either `ends_on` or `auto_archive_days`
may be absent. Only the second remains, so the function keeps its return type
and loses one `?`. The rule in `validate_window` — "an auto-archive term needs
an end date to count from" — becomes unreachable and is deleted, along with the
case in its test.

## Risks / Trade-offs

**A migration that cannot be reversed by another migration** → Making the
columns nullable again is trivial SQL, but the dates the backfill invented are
not recoverable. Accepted: there are no rows, and the values are derived from
data that stays put.

**A 400 where the client used to send `null`** → Any caller that cleared a
window breaks. The only such caller is the admin edit page, which this change
updates in the same breath. A third-party caller would get a clear
`missing field` message.

**The create form becomes two required date fields** → More friction than
before, when both were optional. Mitigated by pre-filling the start date, which
leaves the administrator one decision — when it ends — which is a decision they
have to make anyway for the screen to mean anything.

**Deploy order matters now** → Unlike the previous change, this one is not
additive: the API starts rejecting requests the current web client still sends.
See below.

## Migration Plan

One migration, `0010_require_screen_window.sql`, applied at startup by the
existing `db::migrate`. No manual step.

Deploy API first, then web, as always — but note that the window between them is
not benign this time. An old web client against a new API can still create a
screen (it sends both dates when the fields are filled) but gets a 400 if an
administrator leaves a date blank or uses the "clear the period" control. The
window is minutes and the affected actions are administrator-only, so this is
accepted rather than engineered around.

Rollback: the previous API against the new schema still works, because every
screen now has dates and the old code reads them as `Some`. Rolling the
migration back is not required, and should not be done, since a nullable column
with every row populated behaves identically.

## Open Questions

- Should `ends_on` get a soft default in the form — a month out, say — so the
  administrator is editing a date rather than entering one? Deliberately left
  open; a wrong default is a deadline nobody chose.
- Should an administrator be able to end a screen early by moving `ends_on` into
  the past? It already works, and it is the natural "stop this now" action, but
  it is untested as an intention rather than a correction.
