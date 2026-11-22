## Why

Every day-granularity decision the API makes — what date an entry defaults to,
whether a date lies in the future, which phase a screen is in — compares against
`Utc::now().date_naive()`. The browser, meanwhile, sends and renders plain local
dates: `format.ts:today()` is built from `getFullYear`/`getMonth`/`getDate`.

East of UTC the two disagree for the first hours of every local day. At UTC+2 the
server still believes it is yesterday from 00:00 to 02:00 local time, and until
03:00 while summer time is in force. Inside that window:

- logging a late-night session is refused with "progress cannot be logged for a
  future date", because the date the form pre-filled is tomorrow by the server's
  clock;
- a screen whose period starts that day reports that its period has not started
  yet, and refuses entries and goals.

West of UTC the skew lands at the other end of the day: the last evening of a
screen's period becomes read-only several hours early.

The bug exists today, but it is bounded: only screens with a window configured can
hit the lifecycle half of it. The next change, `require-screen-window`, makes a
window mandatory on every screen. Fixing the comparison first keeps a known-wrong
notion of "today" from being spread across every screen in the system.

## What Changes

- Requests whose outcome depends on the calendar day MAY carry the caller's own
  date. The caller's day becomes the authority for defaulting an entry's date,
  for rejecting future dates, and for resolving a screen's phase.
- A supplied date is clamped to within one day of the server's UTC date. That
  bounds how far a client can shift its own deadline to the skew that physically
  exists, rather than trusting an arbitrary claim. A screen's period gate is the
  only place the difference has consequences: without a bound, a member could
  keep logging against a screen whose period has closed.
- A request that omits the date falls back to the server's UTC date, which is
  exactly today's behaviour. The API stays correct for callers that do not send
  one, so this is additive rather than **BREAKING**.
- The web client sends its local date on those requests. It already computes one
  for the entry form; this makes the server agree with what the member sees.
- The specs say whose "today" is meant. `progress-entries` currently says an
  entry date defaults to "today" and that dates "later than today" are rejected,
  without stating whose clock decides, which is the ambiguity that allowed the
  defect.

Out of scope: per-screen or per-user timezone configuration, and any storage of
instants. Dates stay plain calendar dates with no time component, and the server
gains no timezone database.

## Capabilities

### New Capabilities

- `caller-date`: how the system decides what calendar day a request happens on
  — the date a caller may supply, the bound it is held to, and the fallback when
  it is absent or unusable. Cross-cutting, so it is its own capability rather
  than a clause inside the two that consume it.

### Modified Capabilities

- `progress-entries`: the date an entry defaults to, and the date against which a
  future date is rejected, are the caller's calendar day rather than the server's.
- `screen-lifecycle`: the phase of a screen, and therefore whether it accepts
  writes, resolves against the caller's calendar day.

## Impact

- `apps/api/src/auth/extract.rs` — the new `ClientToday` extractor and the
  header's name, beside `AuthUser` and `AdminUser`.
- `apps/api/src/routes/mod.rs` — the header added to the CORS allow-list,
  without which a browser never sends it at all.
- `apps/api/src/routes/entries.rs` — `resolve_date` and the four `Utc::now()`
  call sites that gate writes.
- `apps/api/src/routes/goals.rs` — two `ensure_writable` call sites.
- `apps/api/src/routes/screens.rs` — three call sites that derive phase for
  listing and detail responses, and the three handlers that call `detail`
  internally and so have to pass the day through to it.
- `apps/api/src/access.rs` — already takes `today` as a parameter and needs no
  change, which is why this is a small change rather than a refactor.
- No DTO change and no regenerated TypeScript. The contract is a header, so it
  is invisible to `ts-rs`; the design records why that cost was accepted.
- `apps/web/src/lib/api/client.ts` — sends the local date on every request and
  every upload, from the `today()` helper `$lib/format.ts` already provides.
  `format.ts` itself is unchanged.
- `apps/api/tests/client_date.rs` and `apps/api/tests/date_boundaries.rs`, both
  new — what the wire does with the header, and the day boundaries end to end
  against a real database.
- `.github/workflows/release-api.yml` — a PostgreSQL service. The boundary
  tests are the first here that need a database, and the workflow's comment
  previously said the suite needed none.
- `README.md` — the header, that it is optional, and what omitting it means.
- No migration. No schema change. No new dependency: resolving an IANA timezone
  on the server would have required one, since `chrono` is built here without a
  timezone database.
