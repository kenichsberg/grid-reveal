## Context

Grid Reveal is a greenfield application. Nothing exists in the repository, so every architectural decision below is being made for the first time rather than adapted to an existing codebase.

The shape of the problem is unusual in one respect that drives most of the design: the visible state of a screen — which tiles are gone and which remain — looks like it should be an append-only log of removal events, but it is better modelled as a pure function of two inputs. Recognising this early removes write contention, makes edits and deletions correct by construction, and shrinks the schema. Most other decisions follow from it.

The deployment target is a set of free tiers chosen before implementation began: Cloudflare Pages for static hosting, an Oracle Cloud Always Free Ampere A1 VM for the API, PostgreSQL (Neon or self-hosted), and Cloudflare R2 for images. These constrain the design more than the feature set does — in particular, the split between a Cloudflare-hosted frontend and a VM-hosted API dictates the entire session model.

Scale is small and private: tens of users, tens of screens, a handful of writes per user per day. There is no multi-tenancy, no public traffic, and no growth plan that would justify designing for horizontal scale.

## Goals / Non-Goals

**Goals:**

- Make the revealed state of a board derivable, not stored, so that concurrent submissions cannot race and corrections rewind cleanly.
- Keep the operational surface to what one person can run for free and maintain occasionally: one VM, one database, one bucket, one static host.
- Render grids of several thousand tiles at 60fps during animation without a WebGL dependency.
- Keep private screens genuinely private, including not disclosing that a given URL corresponds to a real screen.
- Eliminate every external vendor that is not load-bearing — specifically, send no email at all.

**Non-Goals:**

- Real-time propagation of one member's progress to another member's open page. Members see other progress on their next load.
- Public, anonymous, or read-only-link access to screens.
- An admin analytics dashboard. Per-tile attribution is derivable and the data to support one is retained, but no such view ships here.
- Horizontal scaling, multi-region deployment, or high availability. A single VM is the whole backend.
- Self-service account creation or password recovery.

## Decisions

### 1. The board is derived state, not an event log

Each screen stores a fixed random permutation of its tile indices, decided once at creation. The set of revealed tiles is then:

```
total_minutes   = SUM(entries.minutes) WHERE deleted_at IS NULL
revealed_count  = LEAST( floor(total_minutes / unit_minutes), cols × rows )
revealed tiles  = permutation[ 0 .. revealed_count )
```

An entry reveals `floor(total_after / unit) − floor(total_before / unit)` tiles. Worked example at `unit = 5`:

```
entry        total   floor(total/5)   newly revealed   attributed to
──────────────────────────────────────────────────────────────────────
A +13 min     13           2           tiles 0, 1            A
B  +2 min     15           3           tile 2                B
C  +4 min     19           3           —
A  +1 min     20           4           tile 3                A
```

**Alternative considered**: a `reveals` table holding one row per removed tile, written transactionally when an entry lands. Rejected because it requires picking a random *unrevealed* tile under concurrency, which needs either `SELECT … FOR UPDATE SKIP LOCKED` over the free tiles or a per-screen advisory lock, and because deleting or editing an entry then demands a compensating rewind of rows that may interleave with other users' reveals.

**Consequences in our favour**: two members submitting simultaneously is just two `INSERT`s — the count is a `SUM`, so there is no contention and no possibility of the same tile being chosen twice. Editing or deleting an entry rewinds the board automatically. The whole board is reproducible from the entries log at any time.

**Consequence against**: an admin cannot hand-reveal a specific tile, since tile order is fixed by the permutation. No requirement calls for this.

### 2. Permutation stored as an explicit array, and only its revealed prefix is sent

`screens.permutation` is an `INT[]` of tile indices. The alternative — storing a seed and reproducing the shuffle on demand — would require an identical seeded PRNG and shuffle implementation in both Rust and TypeScript, which is a silent-divergence hazard for no benefit at this size (a 12,000-tile array is roughly 48 KB).

The API sends the client only `permutation[0 .. revealed_count)`. Shipping the whole array would let any member read the future reveal order, which spoils the only surprise the application offers.

### 3. Attribution orders by `created_at`; display orders by `occurred_at`

Entries carry a user-chosen `occurred_at DATE`, so members can backdate. That creates a choice about which order defines "the entry that crossed a tile boundary":

| Order by | Effect of backdating an entry |
|---|---|
| `occurred_at` | inserts into the middle of the log, rewriting ownership of every later tile |
| `created_at` | always appends, so backdating never reshuffles attribution |

We attribute by `created_at` and display by `occurred_at`. Ownership stays stable when someone records yesterday's work today, while history still reads in the order things actually happened.

Editing or deleting an entry still rewrites attribution for everything after it. This is inherent to deriving attribution rather than freezing it, and it is accepted: the alternative — freezing each tile's owner at first reveal — allows a tile to remain owned by minutes that no longer exist, which is worse.

### 4. `occurred_at` is a `DATE`, not a timestamp

The client sends `2026-11-13`; the server stores a date. No timezone conversion happens anywhere in the system. Storing an instant would require knowing each member's zone to answer "what did I log today", for no gain — the application has no sub-day resolution.

Duration is stored as a single `INT` of minutes. The hours-and-minutes pair is purely an input widget, bounded at 23h59m, so a single entry is 1–1439 minutes.

### 5. Entries are soft-deleted

Deleted entries keep their row with `deleted_at` set and are excluded from every `SUM`. Per-tile attribution is invisible in this release but is wanted later for an admin dashboard; hard deletion would destroy that history irrecoverably. One nullable column is a cheap price for an audit trail. Edits record `edited_at` without full versioning.

### 6. No real-time transport; the submitter alone sees the animation

Members do not receive other members' progress until they reload. This removes SSE/WebSocket infrastructure, connection management on the VM, and reconnect logic in the client.

It does create a visible seam. Consider a client that has drawn 10 tiles while another member pushes the server to 15:

```
client rendered count = 10
          meanwhile B submits → server count = 15
A submits 10 min (2 tiles) → server count = 17

naive:   animate 10 → 17   (A watches 7 tiles fall, 5 of them B's)
chosen:  snap 10 → 15 instantly, then animate 15 → 17
```

The submit response returns `revealed_count` together with the indices attributable to this submission. The client snaps everything else into place without animation, then animates only the submitter's own tiles. The animation therefore always means "this is what *you* just did", which is the entire point of the feature.

On a cold page load the board renders at its current state with no animation, regardless of how much changed while the member was away.

### 7. 2D canvas, not DOM nodes and not WebGL

Grids are expected to exceed 1,000 tiles and may reach roughly 12,000 (for example, 1,000 hours at 5 minutes per tile). Thousands of DOM elements with transitions is the wrong tool. WebGL is the other extreme and is unnecessary: a few thousand `fillRect` calls per frame is comfortably within a 2D context's budget, and the canvas is only redrawn during an animation — it is static the rest of the time.

```
layer 0   background picture, object-fit: contain into the viewport
layer 1   canvas of cover tiles, one fillRect per unrevealed cell
animate   requestAnimationFrame, per-tile fade/scale ≈400ms, staggered
idle      no redraw at all
```

Cover tiles are flat gray with a border drawn inside each tile, so a revealed tile leaves no residue — the boundary line remains only on the covered side. Hit-testing for a future attribution view is an inverse transform from pointer position to cell index, needing no retained scene graph.

### 8. Grid geometry is derived from the picture's aspect ratio

Square cells, full-window display, and an arbitrary picture aspect ratio cannot all hold simultaneously. The resolution:

```
browser window 16:9
┌───────────────────────────────────────────┐
│▓▓▓▓▓│                             │▓▓▓▓▓▓▓│
│▓▓▓▓▓│   picture box (contain)     │▓▓▓▓▓▓▓│
│▓▓▓▓▓│   grid overlays exactly     │▓▓▓▓▓▓▓│
│▓▓▓▓▓│   this box, 8 × 6           │▓▓▓▓▓▓▓│
└───────────────────────────────────────────┘
   ↑ letterbox            cell = pic_w/8 × pic_h/6
```

The picture is letterboxed into the viewport and the grid overlays exactly the picture box. Cells are then exactly square when `cols / rows == picture aspect ratio`, and near-square otherwise.

At creation the admin supplies a target total time and a unit; the server proposes a `cols × rows` whose ratio is closest to the uploaded picture's, then reports the exact achievable total (`cols × rows × unit`). The admin may adjust `cols` for coarser or finer tiles. The consequence is that **target total time is snapped, not free-form** — 7 hours at 5 minutes is 84 tiles, which does not factor into a rectangle matching a 3:2 photograph.

### 9. Editing geometry recomputes the board and preserves the total

If an admin changes `cols`, `rows`, or `unit_minutes` after entries exist:

```
entries         untouched — SUM(minutes) preserved
permutation     regenerated (the tile set itself changed)
revealed_count  recomputed = LEAST(floor(SUM / new_unit), new N)
```

Logged time is never lost, which is the invariant that matters. The picture may visibly regress — raising `unit_minutes` or shrinking the grid re-covers part of it — and the reveal order reshuffles. This is accepted deliberately rather than blocking the edit.

### 10. PostgreSQL, not a document store

The data is relational in every direction: a many-to-many membership, uniqueness constraints on usernames and slugs, foreign keys guarding picture deletion, and an aggregate `SUM` that the entire board depends on. A document store would make the central read harder and buy nothing. Seven tables:

```
users           id, username CI-UNIQUE, email NULL, password_hash, is_admin, created_at
sessions        token PK, user_id, expires_at
pictures        id, storage_key, width, height, uploaded_by, created_at, deleted_at NULL
screens         id, slug UNIQUE, title, picture_id, cols, rows, unit_minutes,
                permutation INT[], archived_at NULL, created_by, created_at
screen_members  screen_id, user_id                        PK(screen_id, user_id)
entries         id, screen_id, user_id, minutes INT, occurred_at DATE, note NULL,
                created_at, edited_at NULL, deleted_at NULL
goals           id, screen_id, text, created_by, created_at, deleted_at NULL, position
```

`email` is nullable and unused at launch. It exists so that adding email later is a feature flag rather than a migration against live data.

### 11. One repository, two deployment targets

```
  apps/web   SvelteKit + adapter-static (SPA)  →  Cloudflare Pages    app.<domain>
  apps/api   Rust / axum                       →  Oracle Ampere A1    api.<domain>
                                                   Caddy TLS, CF-proxied
```

Cloudflare Pages accepts a build root directory, so a monorepo deploys to Pages and the VM independently. Splitting into two repositories would separate the API contract from its only consumer for no deployment benefit. `ts-rs` generates TypeScript types directly from the Rust request and response structs, making the contract a compile-time artifact rather than hand-maintained duplication.

SvelteKit uses `adapter-static` in SPA mode rather than the Cloudflare adapter. Server-side rendering would need the session cookie at the edge, pointed at a different origin — complexity with no payoff for an authenticated-only application with no SEO surface.

### 12. Opaque session tokens in a cookie, scoped to a shared parent domain

Sessions are random opaque tokens stored in the `sessions` table, carried in an `HttpOnly; Secure; SameSite=Lax; Domain=.<domain>` cookie. A signed JWT was considered and rejected: admin-initiated password reset requires immediate revocation, every authenticated request already touches the database, and stateless tokens would add a revocation list that reintroduces the state they were meant to avoid.

`SameSite=Lax` on a shared registrable domain gives CSRF resistance without additional tokens. **This depends on the frontend and API sharing a registrable domain.** Serving the frontend from `*.pages.dev` against a bare VM IP would force `SameSite=None`, which is both weaker and more fragile. Acquiring a domain is therefore a hard prerequisite, not a polish item.

### 13. No email, by design

Self-hosting SMTP from the VM is not viable — Oracle Cloud blocks outbound port 25 on free-tier instances, and mail from an unwarmed cloud IP without SPF/DKIM/DMARC is filtered as spam reliably enough to be useless. The real choice is therefore between adding a sending vendor and sending no mail at all.

We send none. Admins create accounts and reset passwords directly, which fits a model where admins already control membership. Adding a vendor later (Resend's free tier, for instance) is roughly twenty lines against the already-present `email` column.

### 14. Cloudflare R2 for pictures

R2 is S3-compatible (so `aws-sdk-s3` works from Rust), has a 10 GB free tier, charges nothing for egress, and lives in the same ecosystem as Pages. Supabase Storage was the original candidate but introduces a third vendor for a bucket we can already obtain. Storing images on the VM's 200 GB block volume is simpler still but ties image durability to a single free-tier instance.

Uploads are validated as JPEG/PNG/WebP, capped at 10 MB, downscaled to a 2560px maximum dimension, and re-encoded to WebP server-side. This bounds both storage consumption and the client's decode cost on a full-viewport image.

### 15. Non-members receive 404; administrators are exempt

Requesting a screen the caller does not belong to returns 404, not 403. A 403 confirms that the slug names a real screen, which is exactly the fact a private screen should withhold. Slugs are random short identifiers (for example `/s/k3m9xq2p`) rather than admin-typed readable strings — collision-free, unguessable, and not subject to name squatting. Screens are private regardless, so readability buys only convenience.

Administrators are exempt from the membership check and may open any screen. Without the exemption there is a deadlock: an administrator cannot see a screen in order to add themselves to it, and a screen whose last administrator member was removed becomes permanently unmanageable.

The exemption is paired with an administrator-only directory of every screen, because an access right with no way to enumerate what it covers is inert — an administrator would otherwise need to already know a slug to use it. The personal screen list stays member-scoped for everyone, administrators included: "the screens I am part of" and "every screen I can administer" are different questions, and collapsing them would bury an administrator's own screens among all the rest.

## Risks / Trade-offs

**[Oracle Always Free A1 capacity is frequently unavailable]** → Provisioning in popular regions commonly fails for days. Secure the instance before implementation completes rather than discovering this at deployment. Fallback: any small VPS, since nothing in the design is Oracle-specific.

**[Oracle may reclaim idle Always Free instances]** → Keep a nightly `pg_dump` to R2 and treat the VM as reproducible from configuration. This matters more if PostgreSQL is self-hosted on the same instance.

**[The session model depends on owning a domain]** → Without one, cookies require `SameSite=None` and a weaker CSRF posture. Register the domain early; it is roughly $10/year and gates the auth implementation.

**[Editing an entry rewrites attribution for every later tile]** → Inherent to deriving attribution rather than freezing it. Accepted because attribution is invisible in this release and the alternative produces tiles owned by minutes that no longer exist.

**[Geometry edits can visibly un-reveal part of the picture]** → Confirmed as acceptable. Mitigate in the UI by warning the admin and showing the resulting tile count and revealed count before they confirm.

**[Canvas performance is unverified above ~12,000 tiles]** → The approach should hold comfortably, but this is an estimate. Benchmark with a deliberately large grid before considering the renderer done; the fallback is to redraw only dirty rectangles instead of the full canvas per frame.

**[No password recovery without an admin]** → A locked-out sole admin has no path back except direct database access. The env-var bootstrap admin partially mitigates this, since redeploying with known credentials recreates access.

**[Neon free tier suspends after idle and adds cold-start latency]** → Roughly 500ms on the first request after idle, which is acceptable for this traffic. Self-hosting PostgreSQL on the VM removes it entirely at the cost of owning backups.

**[R2's 10 GB free tier bounds the picture library]** → At 2560px WebP, images run a few hundred kilobytes each, so the practical ceiling is in the thousands. Soft-delete does not reclaim space; hard-delete of unreferenced pictures does.

**[A single VM is a single point of failure]** → Accepted. There is no availability requirement, and the application is unusable-but-harmless while down.

## Migration Plan

There is no existing system, so this is a first deployment rather than a migration.

1. Register the domain; point `app.` and `api.` at Cloudflare.
2. Provision the Oracle Ampere A1 instance, expect retries, and install Caddy for TLS termination in front of axum.
3. Provision PostgreSQL (Neon, or on the VM) and the R2 bucket; populate `.env` from them. `.env` is gitignored from the first commit.
4. Run migrations. On first boot the API seeds an admin from `ADMIN_USERNAME` and `ADMIN_PASSWORD` if no admin exists.
5. Deploy `apps/web` to Cloudflare Pages with the build root set to that directory.
6. Verify the end-to-end cookie flow across `app.` and `api.` before building out further — this is the integration most likely to be subtly wrong.

Rollback is redeployment of a prior commit. The only destructive operations are migrations; since there is no production data until launch, forward-fix is the expected path.

## Open Questions

- **Neon or PostgreSQL on the VM?** The VM has 24 GB of RAM and would host it comfortably, removing a vendor and the cold start, at the cost of owning backups. Deferred: the application code is identical either way, so this is decided at deployment by setting `DATABASE_URL`.
- **Which domain?** Not yet registered. Blocks nothing until the cookie flow is wired up.
- **Confetti implementation** for the completion celebration — a small library versus a few dozen lines on the existing canvas. Decide during implementation; the latter avoids a dependency given the canvas is already present.
- **Exact argon2id parameters.** Default to the OWASP-recommended profile and revisit only if login latency on the A1's ARM cores proves noticeable.
