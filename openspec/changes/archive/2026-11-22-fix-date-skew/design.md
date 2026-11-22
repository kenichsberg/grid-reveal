## Context

The API resolves "what day is it" nine times in production code — four in
`entries.rs`, two in `goals.rs`, three in `screens.rs` — always as
`Utc::now().date_naive()`. The web client resolves the
same question from the browser's local clock. The two agree only for callers at
UTC+0, and the disagreement is a whole calendar day for the first hours of local
time east of UTC, or the last hours west of it.

`access.rs` is already built the right way: `ensure_writable`, `covers` and
`Screen::phase` all take `today: NaiveDate` as a parameter and hold no clock of
their own. Every unit test in `models.rs` passes a fixed date. The defect lives
entirely in the call sites that supply the argument, which is what keeps this
change small.

Entry dates are already client-supplied: `EntryForm.svelte` pre-fills from the
browser's local date and sends it as `occurred_at`. What is missing is the
ceiling that date is checked against, which is the server's own clock.

## Goals / Non-Goals

**Goals:**

- A calendar day decided by the caller, so that the day a member sees in the
  date picker is the day the server judges them against.
- One mechanism covering reads and writes alike, since a screen's phase is
  reported by GET responses and enforced by POST handlers.
- Graceful degradation: a caller that sends nothing behaves exactly as today.
- No new dependency and no stored instants.

**Non-Goals:**

- Per-user or per-screen timezone configuration. A screen does not gain a
  timezone, and members are not asked for one.
- A server-side timezone database. Resolving an IANA zone would require
  `chrono-tz`; the caller already knows its own date, so the server never needs
  to derive one.
- Sub-day resolution anywhere. Dates stay plain `DATE` columns.
- Correcting the skew for non-browser callers. A script calling the API without
  the header gets UTC semantics, which is the documented fallback.

## Decisions

### The caller's date travels as a request header, not a body field

`X-Client-Date: 2026-11-19`, parsed by a `ClientToday` extractor that sits
beside `AuthUser` and `AdminUser` in `auth/extract.rs`.

A body field was the first instinct, but it does not reach far enough: `GET
/screens`, `GET /screens/{slug}` and `GET /admin/screens` all derive a phase for
every screen they return, and have no body to carry a field. Covering them would
mean a query parameter in parallel with a body field — two spellings of one
concept, each needing its own validation, and a URL that varies by day.

A header applies uniformly to every route, needs no change to a single DTO, and
composes with the extractor pattern the codebase already uses for identity. The
cost is that it is invisible in the generated TypeScript types, so the contract
lives in `client.ts` and in tests rather than in `ts-rs` output.

### An unusable value falls back to UTC rather than failing the request

The header is ignored, with a `tracing::debug`, when it is absent, malformed, or
further than one day from the server's UTC date. It is not a 400.

A wrong device clock, a stale tab resumed after midnight, or a proxy that strips
unknown headers should degrade to the behaviour the API has today, not make the
application unusable for that caller. There is no legitimate client for which a
rejected date is more useful than a fallback, and a 400 on a header the user
never knowingly set is close to impossible to self-diagnose.

### The clamp is one day, and exists for exactly one endpoint

A caller may shift the date by at most ±1 day from UTC. The real skew never
exceeds that: UTC−12 to UTC+14 spans 26 hours, so a correct client is at most one
calendar day away in either direction.

The clamp matters at one place only: `ensure_writable`. Everywhere else a
supplied date can only inconvenience the caller who supplied it — defaulting
their own entry to the wrong day, or seeing their own screen labelled with the
wrong phase. But a screen's period gate is a deadline, and an unbounded date
would let a member log against a screen that closed last month by claiming it is
still open. Bounded at one day, the most a member can buy is the grace period
their timezone already justifies.

This is a deliberate, small loosening: today the window closes at 00:00 UTC on
the day after `ends_on`; afterwards it closes up to a day later for a caller who
says so. Members can already backdate freely inside the window, so the new
capability is narrow.

### `occurred_at` keeps its own meaning

`resolve_date` still defaults to today and still refuses the future. Both now
mean the caller's today. The function keeps its shape and takes the resolved date
as an argument instead of reading the clock, which also makes it unit-testable
for the first time.

### CORS must learn the header

`build_cors` in `routes/mod.rs` allows exactly `CONTENT_TYPE` and `ACCEPT`.
A browser will not send `X-Client-Date` cross-origin until it is listed in
`allow_headers`, and the preflight failure presents as every request failing —
not as the header quietly going missing. This is the one step that breaks loudly
if forgotten, and the one most easily left out of a local test, where the web
client and API share an origin through Vite.

## Risks / Trade-offs

**A header is less discoverable than a typed field** → The `ts-rs` DTOs will not
mention it. Mitigated by setting it in one place in `client.ts`, so no caller
site has to remember, and by contract tests that pin the behaviour.

**Two members in different zones see the same screen differently for a few
hours** → One sees `Active`, the other `Ended`. This is correct for each of
them: the screen ends at the end of *their* day. Accepted, and worth a sentence
in the spec so it is not later read as a bug.

**Auto-archiving becomes per-viewer at the boundary** → `is_archived_on` derives
from `ends_on + auto_archive_days`, so a screen can appear archived to one
viewer and active to another for up to a day. Same acceptance; nothing is
written, the derivation is pure.

**A malicious member extends their own deadline by a day** → Bounded by the
clamp, and a member who wanted another day could already log it a minute before
the window closed. Accepted for a tracker used by a group that knows each other.

**Silent fallback hides a broken client** → A client that sends a malformed
header gets UTC semantics with no error. Mitigated with a `debug` log; the
alternative of a 400 fails the whole request for a value that is advisory.

## Migration Plan

No schema change, no migration file, no data backfill.

Deploy order is API first, web second. The API accepts the header before anything
sends it, and falls back to UTC in the meantime — which is the behaviour it has
today, so the intermediate state is not a regression. The web deploy then starts
sending it.

Rollback is the reverse and equally safe: an older API ignores an unknown header,
so rolling back the API alone leaves the web client sending something harmless.

## Open Questions

- Should the read-only listing endpoints use the caller's date as well, or only
  the write gates? The design assumes yes, uniformly — a screen labelled "ended"
  while its owner can still write to it would be worse than the skew it fixes.
- If the app ever gains members spread across many zones in one screen, does a
  screen want a timezone of its own, so a deadline means one instant for the
  whole group? Out of scope here; revisit if a screen ever spans zones.
