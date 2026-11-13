## Why

Groups that work toward a shared time-based goal — a study group, a team sprint, a training plan — have no satisfying way to see collective effort accumulate. Spreadsheets and checklists record numbers but produce no sense of shared momentum.

Grid Reveal turns logged time into a visual reward. A background picture starts fully covered by a grid of opaque tiles; every unit of time anyone logs removes one tile at random, so the image emerges gradually as the group progresses. This change builds the entire application from an empty repository.

## What Changes

- **New monorepo** with a SvelteKit SPA frontend (Cloudflare Pages) and a Rust/axum API (Oracle Cloud VM), sharing generated TypeScript types via `ts-rs`.
- **Primitive username/password auth** with no email dependency: argon2id hashes, opaque DB-backed session cookies, admin-created accounts, admin-performed password resets, and a bootstrap admin seeded from environment variables.
- **Shared picture library**: admins upload JPEG/PNG/WebP images that are transcoded and downscaled server-side, stored in object storage, and reusable across screens. Soft-delete hides a picture from new screens; hard-delete is blocked while any screen references it.
- **Screens** — the core entity. Each has a title, a background picture, a grid geometry (`cols × rows`), a unit of time per tile, a random unguessable URL slug, and an explicit member list. Non-members receive 404, never 403, so private screens do not leak their existence.
- **Progress entries**: members log hours and minutes against a date they choose (defaulting to today), may edit or soft-delete their own entries, and see only their own history.
- **Derived reveal board**: the set of revealed tiles is computed from `SUM(entries.minutes)` against a per-screen fixed shuffle, rather than stored as removal events. This eliminates write contention entirely and makes edits and deletions rewind the board correctly for free.
- **Canvas rendering** of grids exceeding 1,000 tiles, with a reveal animation shown only to the member who submitted — other members' progress snaps in silently so the animation always represents the submitter's own contribution.
- **Screen goals**: free-text bullets any member can add, deletable by the author or an admin, shown in a collapsible panel overlaying the picture.
- **Screen lifecycle**: clone a screen (copying members, goals, and geometry; requiring a new title and new picture; resetting the board), archive it read-only, and delete it behind an explicit type-the-title confirmation.
- **Completion**: once every tile is revealed the screen celebrates for all members on every visit, while continuing to accept and total further entries.
- Credentials live in a gitignored `.env`; no secrets enter the repository.

## Capabilities

### New Capabilities

- `user-auth`: Username/password registration by admins, argon2id hashing, opaque session tokens in HttpOnly cookies, login/logout, self-service password change, admin-initiated password reset, and env-var bootstrap of the first admin account.
- `picture-library`: Admin upload with format/size validation, server-side downscale and WebP transcode, object-storage persistence, a library browsable across all admins, soft-delete, and reference-guarded hard-delete.
- `screens`: Screen creation with title, picture selection, grid geometry derived from picture aspect ratio and target time, random URL slug, membership management, member-scoped listing, and 404-for-non-members access control.
- `progress-entries`: Logging time as hours plus minutes against a chosen date, editing and soft-deleting one's own entries, per-user private history, and aggregate screen totals.
- `reveal-board`: The permutation-plus-sum derivation of revealed tiles, canvas rendering of the tile grid over the background picture, submitter-only reveal animation, geometry recomputation on screen edit, and the completion celebration.
- `screen-goals`: Member-authored text bullets on a screen, deletion by author or admin, admin-only authorship visibility, and the collapsible presentation panel.
- `screen-lifecycle`: Cloning a screen into a fresh board, archiving to read-only, and explicit-confirmation deletion.

### Modified Capabilities

A later round of changes, folded into this proposal rather than tracked separately
because nothing has shipped yet:

- **screen-lifecycle** — a screen gains an optional active period. Outside it the screen
  is read-only, keeping its board and celebration, and after a retention term it archives
  itself, so a finished period retires without an administrator remembering it. The phase
  is derived from the stored dates on every read rather than stored, for the same reason
  the board is: there is no scheduler in the service, and a stored state would need one.
- **progress-entries** — `occurred_at` is bounded by the period as well as by today, and
  entry writes are refused in every phase but the active one.
- **screens** — the period is set at creation, edit and clone. Membership visibility is
  reversed: the member list is now shown to every member rather than to administrators
  alone, while per-member logged time stays private.
- **screen-goals** — goals move from an overlay docked to the board into a tab, and freeze
  with everything else when a period closes.
- **reveal-board** — the board's chrome is reserved rather than floated over the picture.
  On a narrow viewport the letterbox becomes a title band and a tab bar; on a wide one the
  same content becomes a rail, a tab strip and a panel column. The picture is consequently
  smaller than it was, which is the deliberate trade for chrome that never lands on the
  photograph.

## Impact

**New code** — the entire repository:

- `apps/web/` — SvelteKit with `adapter-static` in SPA mode, deployed to Cloudflare Pages.
- `apps/api/` — Rust/axum service, deployed to an Oracle Cloud Always Free Ampere A1 VM behind Caddy for TLS, proxied through Cloudflare.
- `apps/api/migrations/` — PostgreSQL schema: `users`, `sessions`, `pictures`, `screens`, `screen_members`, `entries`, `goals`.

**External dependencies**:

- PostgreSQL (Neon free tier, or self-hosted on the same VM).
- S3-compatible object storage for pictures (Cloudflare R2).

**Operational prerequisites** — both block deployment and should be secured before implementation completes:

- A registered domain. The session cookie model depends on the frontend and API sharing a registrable domain (`Domain=.example.com; SameSite=Lax; Secure; HttpOnly`); splitting across unrelated origins would force `SameSite=None` and a weaker CSRF posture.
- An Oracle Always Free Ampere A1 instance. Capacity is frequently unavailable in popular regions and provisioning can take repeated attempts over several days.

**Explicitly out of scope**: real-time updates between members (no SSE or WebSocket), email delivery of any kind, an admin statistics dashboard (per-tile attribution is recorded and derivable but not surfaced), and public or anonymous screen viewing.
