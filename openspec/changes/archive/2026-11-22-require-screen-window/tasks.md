## 1. Make the schema require a period

- [x] 1.1 Add `apps/api/migrations/0010_require_screen_window.sql`, leaving `0008` untouched so no already-applied migration changes checksum and breaks startup
- [x] 1.2 Backfill: `starts_on` from `created_at::date`, `ends_on` from the later of `created_at::date` and the screen's last `entries.occurred_at`, so a migrated screen reads as ended rather than open forever
- [x] 1.3 `ALTER COLUMN starts_on SET NOT NULL` and the same for `ends_on`, after the backfill
- [x] 1.4 Replace the `screens_window_ordered` CHECK, whose `IS NULL` arms are now dead, with a plain `ends_on >= starts_on`
- [x] 1.5 Comment the migration with what the backfill assumes and why `0008` was not amended
- [x] 1.6 Run the migration against the local container and confirm `\d screens` shows both columns `not null`

## 2. Drop the absent-bound cases from the model

- [x] 2.1 Change `Screen::starts_on` and `ends_on` to `NaiveDate` in `apps/api/src/models.rs`
- [x] 2.2 Simplify `phase` and `covers` — four `is_some_and` guards become four plain comparisons
- [x] 2.3 Simplify `auto_archive_on`: it keeps returning `Option<NaiveDate>`, since `auto_archive_days` is still optional, but loses the `ends_on?`
- [x] 2.4 Delete the tests that construct a screen without a window: `a_screen_without_a_window_is_active_forever`, `an_auto_archive_term_without_an_end_date_never_fires`, and `an_open_ended_window_only_bounds_the_end_it_has`
- [x] 2.5 Give the `screen()` test helper a real period so every remaining test still compiles, and fold `windowed()` into it
- [x] 2.6 Update the `screen(None, None, None)` helper calls in `apps/api/src/access.rs` tests; `an_open_screen_is_writable` becomes a screen inside its period

## 3. Require it at the edge

- [x] 3.1 `validate_window` in `apps/api/src/routes/screens.rs` takes `NaiveDate` for both bounds, and loses the "an auto-archive term needs an end date to count from" rule, which is now unrepresentable
- [x] 3.2 Delete `one_bound_alone_is_a_valid_window` and the `validate_window(None, None, Some(15))` case from its auto-archive test
- [x] 3.3 `CreateScreenRequest` and `CloneScreenRequest`: `starts_on` and `ends_on` become `NaiveDate`, so a request missing one is a 400 from serde before the handler runs
- [x] 3.4 `ScreenWindow`: both dates become `NaiveDate`, and the `Default` derive goes, since `NaiveDate` has none — check nothing calls `ScreenWindow::default()` first
- [x] 3.5 `ScreenDetail`: `starts_on` and `ends_on` become plain dates; `ScreenSummary` and `AdminScreenEntry` likewise for `ends_on`
- [x] 3.6 Add `starts_on` to `ScreenSummary` and `AdminScreenEntry`, so the list can say when an upcoming screen opens instead of only "Not started"
- [x] 3.7 Run `scripts/gen-types.sh` and confirm the regenerated TypeScript drops the `| null` on those fields

## 4. Make the client supply what is now required

- [x] 4.1 `apps/web/src/routes/admin/screens/new/+page.svelte`: pre-fill the start date with `today()` from `$lib/format`, mark both date fields required, and refuse to submit without them
- [x] 4.2 `apps/web/src/routes/admin/screens/[slug]/+page.svelte`: remove the control that cleared the window, and keep the edit form from submitting a blank date
- [x] 4.3 `apps/web/src/lib/screen.ts`: delete the "no date" arms of `timelineLabel` and the range label, now unreachable
- [x] 4.4 `apps/web/src/routes/+page.svelte`: drop the `starts_on: null` workaround and pass the summary's own start date
- [x] 4.5 Check every other reader of `starts_on` / `ends_on` for a null check that is now dead: `rg 'starts_on|ends_on' apps/web/src`

## 5. Prove it

- [x] 5.1 Model tests: the period is inclusive at both ends and the phase walks it, with no windowless case left to test
- [x] 5.2 Integration test in `apps/api/tests/screen_window.rs` (its own file: `date_boundaries.rs` is about the caller's day, not about what a screen must have): creating a screen without an end date is refused
- [x] 5.3 Integration test: a PATCH whose window omits a date is refused and the stored period is unchanged
- [x] 5.4 Integration test: a PATCH with no `window` at all leaves the period alone, so the two kinds of absence stay distinguishable
- [x] 5.5 Update `apps/api/tests/contract.rs`, which builds a `ScreenDetail` with `starts_on: None`
- [x] 5.6 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `npm run check`

## 6. Finish

- [x] 6.1 Update any README or deploy note that describes a screen's period as optional
- [x] 6.2 Record the deploy order and the one broken case during it — an old web client clearing a window gets a 400 — in the change's notes
- [x] 6.3 Create a screen through the running app, confirm the start date arrives pre-filled and the screen is refused without an end date
