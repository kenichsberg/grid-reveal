## Context

Three facts about the existing system shape this change.

**Membership is the access boundary.** `access::screen_for_viewer` answers 404,
not 403, for a screen the caller is not a member of, so a slug discloses
nothing. An invite has to create membership, and until it does, the invited
person must still be able to see *something* about what they are joining —
which means one endpoint that is deliberately outside that boundary.

**Sessions are already server-side and opaque.** `session::create` mints 256
bits of randomness and stores it; the cookie carries no claims. An invite token
is the same kind of object with a different lifetime, so it can follow the same
pattern rather than inventing a signed-token scheme.

**Nothing in this codebase is stored when it can be derived.** The board, the
screen's phase, the archived flag: all computed per request from a stored date
and the caller's day. An invite's validity is the same kind of question.

## Goals / Non-Goals

**Goals:**

- The administrator never handles the invited person's password.
- One link, one person, enforced by the database rather than by a read
  followed by a write.
- No new notion of time. An invite's life is the screen's life.
- The public surface stays exactly two endpoints, both scoped to a token
  someone already holds.

**Non-Goals:**

- Open signup, or any invite that is not tied to one screen.
- Email, SMS, or any delivery mechanism. The administrator copies a URL.
- Per-invite expiry dates, use counts, or role assignment. Every invited
  person joins as an ordinary member; administrators are still made by
  administrators.
- Rate limiting. A 120-bit token is not enumerable, and this application has no
  rate limiting anywhere to be consistent with.

## Decisions

### The invite's life is derived from the screen, not stored

An invite is open when all of these hold, evaluated per request:

```
accepted_at IS NULL           -- single use
revoked_at  IS NULL           -- not withdrawn
screen.phase(caller's day) ∈ { Upcoming, Active }
```

No `expires_at` column. A stored expiry would be a second deadline that could
disagree with the screen's own — an invite still valid for a screen that closed
last week, or dead for one whose period was extended. Extending the end date is
already how an administrator reopens a closed screen; it now also revives the
links for it, which is the behaviour someone extending a deadline wants.

`Upcoming` counts as open on purpose: invitations go out before the start date,
which is most of what a start date is for.

The caller's day comes from `ClientToday`, so an invite behaves at the boundary
exactly like every other day-granularity decision.

### The token is random, stored as it is, and shaped like a slug

24 characters from the existing slug alphabet — 32 confusable-free symbols, so
120 bits. Long enough that guessing is not a threat model, short enough to
paste into a chat message without wrapping.

Stored in plaintext, like session tokens, rather than hashed. The argument for
hashing is that a leaked backup yields working invites; the argument against is
that it yields working *sessions* already, so hashing one and not the other
buys nothing but inconsistency. If that changes, both should change together.

### Peek and accept are separate, and only accept consumes

`GET /invites/{token}` returns the screen's title and picture and whether the
invite is open. It is safe to call repeatedly — a browser prefetch, a link
preview in a chat client, or the person opening the page twice must not burn
the invite.

`POST /invites/{token}/accept` is the only thing that consumes it. This is why
the design is two endpoints rather than one page that does both.

Both are public. `GET` discloses one screen's title and picture to whoever
holds the token, which is the minimum needed for the person to know what they
are agreeing to join, and is exactly what the administrator intended to share.

### Single use is a conditional UPDATE, not a check

```sql
UPDATE screen_invites
   SET accepted_at = now(), accepted_by = $1
 WHERE id = $2 AND accepted_at IS NULL AND revoked_at IS NULL
RETURNING id
```

Zero rows returned means someone else accepted it first, and the whole
transaction rolls back — including the account that was about to be created.
Reading the invite, deciding it is open, and then writing would leave a window
in which two people both pass the check.

The account creation, the membership row, the consumption, and the session all
live in that one transaction. A taken username rolls the invite back to
unused, which is the behaviour that matters: a typo must not cost the person
their invitation.

### Accepting while already signed in joins that account

If the request carries a valid session, the invite adds *that* account to the
screen and consumes itself, taking no username or password.

The alternative — refusing, or silently creating a second account — is worse in
both directions. A member who is sent a link to a second screen would otherwise
end up with two accounts and no way to merge them, and that mistake is not
recoverable without an administrator. The cost is one branch in the handler.

### The URL is built by the client, not the API

The API returns the token. `client.ts` builds `${location.origin}/join/${token}`.

The API has no idea what origin serves the frontend — in production they are
different hosts entirely, and `CORS_ALLOWED_ORIGINS` may hold several. A
configured base URL would be a fourth place to keep that value correct.

### `PUBLIC_ROUTES` becomes a predicate

`+layout.svelte` holds `const PUBLIC_ROUTES = ['/login']` and tests it with
`includes(pathname)`. A token-bearing path cannot be matched by equality, so
the constant becomes a function:

```ts
const isPublicPath = (p: string) => p === '/login' || p.startsWith('/join/');
```

Used in both places it is tested today — the redirect effect and the
unauthorized handler — so an expired session on the join page does not bounce
the invited person to a login form they cannot use.

## Risks / Trade-offs

**A token in a URL path leaks through `Referer`** → Only to hosts the page
loads resources from, and the join page loads none. Noted rather than
mitigated.

**Whoever holds the link can use it, not just the intended person** → That is
what a link is. Single use bounds the damage to one account on one screen, and
the administrator sees who accepted. A forwarded link is the same risk as a
forwarded password, with a shorter life.

**The invite dies when the period ends, which may surprise** → An invite sent
on the last day of a screen is open for hours. This is intentional — the point
is that a link cannot outlive the thing it grants access to — but the admin UI
should show the date an invite stops working, so it is never a silent failure.

**An administrator could invite the same person twice** → Two invites, two
accounts. Nothing prevents it; the optional label on each invite is what makes
the list legible enough to avoid it. Not worth a constraint.

**Public endpoints are new attack surface** → Two of them, both requiring a
120-bit token, neither accepting anything but a username and password that are
already validated by the existing helpers. A bad token is a 404, the same
answer a non-member gets for a screen.

## Migration Plan

One migration, `0011_screen_invites.sql`: the table, a unique index on the
token, an index on `screen_id` for the admin listing, and `ON DELETE CASCADE`
from both `screens` and `users` so deleting either leaves nothing dangling.

No backfill — there is nothing to migrate. Deploy API first so the endpoints
exist before any link is generated; the web deploy adds the UI that generates
them, so there is no window in which a link exists and cannot be opened.

## Open Questions

- Should an accepted invite be deleted rather than kept with `accepted_at`?
  Kept, for now: "who joined through which link" is the only record of how
  someone got in, and the table is tiny.
- Should the admin UI warn when generating an invite for a screen whose period
  ends within a day or two? Probably, but it is wording rather than mechanism,
  and better decided once the list exists.
