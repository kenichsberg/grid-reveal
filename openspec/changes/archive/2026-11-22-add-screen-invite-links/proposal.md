## Why

Every account in this system is typed in by an administrator. To add someone to
a screen, the administrator invents a username, invents a password, saves it,
and then sends both to the person over whatever channel is at hand — which
means the password exists in a chat log, and the person's first act is to use a
credential someone else chose for them.

That is the only onboarding path there is. There is no signup, deliberately:
the application is for small groups who know each other, and an open signup
form would invite everyone else.

An invite link keeps the closed door and moves the credential: the
administrator generates a URL for one person, that person opens it, chooses
their own username and password, and lands on the screen they were invited to,
already a member. The administrator never learns the password, and nothing
secret travels except a token that is useless once used.

## What Changes

- An administrator can generate an invite link for a specific screen. The link
  carries a random, unguessable token and nothing else.
- Opening the link shows what is being joined — the screen's title and picture
  — and a form for a username and password. No account is needed to see it.
- Accepting the invite creates the account, joins it to that screen, signs the
  person in, and consumes the invite in one transaction. Any failure — a taken
  username, a short password — leaves the invite unused.
- An invite is **single-use**: the first acceptance wins, enforced by a
  conditional update rather than by a check-then-write.
- An invite **dies with its screen's period**. There is no stored expiry: an
  invite is open exactly while the screen is `Upcoming` or `Active` for the
  person holding it. The previous change made that a guarantee by requiring
  every screen to have an end date.
- An administrator can revoke a pending invite, and can see which invites are
  outstanding, which were accepted, and by whom.
- Cloning a screen does not copy its invites. A clone is a new run with a new
  membership question; a link generated for the old one must not open the new.
- A new public route, `/join/{token}`, joins `/login` as a page reachable
  without a session.

Out of scope: account-only invites that join no screen, invites that are
reusable by many people, email delivery of any kind, and expiry a human sets by
hand. An administrator who wants a different deadline moves the screen's end
date, which is the date that already governs everything else about the screen.

## Capabilities

### New Capabilities

- `screen-invites`: generating, inspecting, accepting, and revoking an invite
  link, and the rules that decide whether one is still open.

### Modified Capabilities

- `user-auth`: an account may now be created by the person who will own it,
  through an invite, rather than only by an administrator. The closed-signup
  property is unchanged — without a token there is still no way in.
- `screens`: membership may be acquired by accepting an invite, not only by an
  administrator adding a user.
- `screen-lifecycle`: cloning a screen leaves its invites behind.

## Impact

- `apps/api/migrations/0011_screen_invites.sql` — the new table.
- `apps/api/src/models.rs` — a `ScreenInvite` row and the predicate that decides
  whether it is open.
- `apps/api/src/routes/invites.rs` — new: two public endpoints (peek, accept)
  and two administrator endpoints (create, revoke).
- `apps/api/src/routes/screens.rs` — the admin detail response gains its
  invites; the clone handler gains a comment and a test, not code.
- `apps/api/src/routes/mod.rs` — the new router.
- `apps/api/src/dto.rs` — the invite DTOs, and their generated TypeScript.
- `apps/web/src/routes/join/[token]/+page.svelte` — new: the public page.
- `apps/web/src/routes/+layout.svelte` — `PUBLIC_ROUTES` becomes a predicate,
  since `/join/{token}` cannot be matched by equality.
- `apps/web/src/routes/admin/screens/[slug]/+page.svelte` — generate, copy,
  revoke, and the list of outstanding invites.
- `apps/web/src/lib/api/client.ts` — the four calls.
- `README.md` — how someone joins.
