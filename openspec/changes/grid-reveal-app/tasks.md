## 1. Repository and Tooling Scaffold

- [x] 1.1 Initialize the git repository and create a `.gitignore` covering `.env`, `target/`, `node_modules/`, and build output
- [x] 1.2 Create the monorepo layout with `apps/web/` and `apps/api/` directories
- [x] 1.3 Scaffold the Rust API crate with `axum`, `tokio`, `sqlx` (postgres, macros, chrono), `argon2`, `serde`, `ts-rs`, `image`, `aws-sdk-s3`, `rand`, `tracing`
- [x] 1.4 Scaffold the SvelteKit app with `@sveltejs/adapter-static` configured for SPA mode with a fallback page
- [x] 1.5 Create `.env.example` documenting `DATABASE_URL`, `ADMIN_USERNAME`, `ADMIN_PASSWORD`, `COOKIE_DOMAIN`, `R2_*` credentials, and `BIND_ADDR`
- [x] 1.6 Add a config loader in the API that reads environment variables at startup and fails fast with a clear message when a required variable is missing
- [x] 1.7 Wire `ts-rs` export so Rust request and response types emit TypeScript definitions into `apps/web/src/lib/api/types/`, and add a script to regenerate them
- [x] 1.8 Add a Docker Compose stack (PostgreSQL, API, built frontend) so the application can be run locally without installing a toolchain

## 2. Database Schema

- [x] 2.1 Write the migration for `users` (id, username with a case-insensitive unique index, nullable email, password_hash, is_admin, created_at)
- [x] 2.2 Write the migration for `sessions` (token primary key, user_id foreign key with cascade delete, expires_at) plus an index on user_id
- [x] 2.3 Write the migration for `pictures` (id, storage_key, width, height, uploaded_by, created_at, nullable deleted_at)
- [x] 2.4 Write the migration for `screens` (id, unique slug, title, picture_id, cols, rows, unit_minutes, permutation int[], nullable archived_at, created_by, created_at)
- [x] 2.5 Write the migration for `screen_members` (screen_id, user_id, composite primary key, cascade deletes)
- [x] 2.6 Write the migration for `entries` (id, screen_id, user_id, minutes, occurred_at date, nullable note, created_at, nullable edited_at, nullable deleted_at) plus an index on (screen_id, created_at)
- [x] 2.7 Write the migration for `goals` (id, screen_id, text, created_by, created_at, nullable deleted_at, position)
- [x] 2.8 Add a migration runner that applies pending migrations on API startup

## 3. Authentication

- [x] 3.1 Implement argon2id password hashing and verification helpers using the OWASP-recommended parameter profile
- [x] 3.2 Implement admin bootstrap on startup: create the env-var admin when no admin exists, make no change when one does, and fail startup when the variables are absent
- [x] 3.3 Implement `POST /auth/login` returning a session cookie with `HttpOnly`, `Secure`, `SameSite=Lax`, and the configured `Domain`, using a uniform error and constant-time comparison for unknown usernames and wrong passwords
- [x] 3.4 Implement `POST /auth/logout` deleting the session row and clearing the cookie
- [x] 3.5 Implement the session-resolving middleware that rejects missing, unknown, and expired tokens with 401 and attaches the resolved user to the request
- [x] 3.6 Implement an admin-only extractor that rejects non-administrators with 403
- [x] 3.7 Implement `POST /users` (admin only) creating accounts, validating username length 3–32, case-insensitive uniqueness, and password minimum length 8
- [x] 3.8 Implement `POST /auth/password` for self-service password change requiring the current password and revoking the user's other sessions
- [x] 3.9 Implement `POST /users/:id/password` (admin only) resetting a password and deleting all of that user's sessions
- [x] 3.10 Implement `GET /auth/me` returning the current user without any password material
- [x] 3.11 Verify no API response serializes a password hash, and add a test asserting it

## 4. Picture Library

- [x] 4.1 Implement the object storage client wrapping the S3-compatible API for put, get-URL, and delete
- [x] 4.2 Implement upload validation rejecting non-JPEG/PNG/WebP content and payloads over 10 MB
- [x] 4.3 Implement image processing: decode, downscale preserving aspect ratio so the larger dimension is at most 2560 pixels without upscaling smaller images, re-encode to WebP, and capture final dimensions
- [x] 4.4 Implement `POST /pictures` (admin only) storing the processed object and inserting the library row
- [x] 4.5 Implement `GET /pictures` (admin only) listing all non-soft-deleted pictures with their dimensions and URLs
- [x] 4.6 Implement `DELETE /pictures/:id?soft=true` marking `deleted_at` so the picture leaves the picker while existing screens keep rendering it
- [x] 4.7 Implement hard delete rejecting the request with a reference count when any screen references the picture, and removing both the object and the row when none does
- [x] 4.8 Implement picture serving or signed-URL generation so member clients can load a screen's background

## 5. Screens and Membership

- [x] 5.1 Implement the geometry solver: given a target total, a unit time, and a picture aspect ratio, propose the `cols × rows` whose ratio is closest to the picture's and return the exact achievable total
- [x] 5.2 Implement `GET /screens/geometry-preview` so the creation form can recompute rows, tile count, and exact total as the admin adjusts columns
- [x] 5.3 Implement random slug generation with a uniqueness check against existing screens
- [x] 5.4 Implement permutation generation producing a random ordering containing every tile index exactly once
- [x] 5.5 Implement `POST /screens` (admin only) requiring a non-empty title and a picture, accepting either an uploaded file or a library picture id, and adding the creating admin as a member
- [x] 5.6 Implement the membership guard resolving a screen by slug, returning 404 for non-members and 401 for unauthenticated callers
- [x] 5.7 Implement `GET /screens` returning only the caller's screens, partitioned into active and archived
- [x] 5.8 Implement `GET /screens/:slug` returning title, picture URL, geometry, unit, participant count, total minutes, revealed count, and the revealed permutation prefix only
- [x] 5.9 Implement `POST /screens/:slug/members` and `GET /screens/:slug/members` (admin only), with member identities never returned to non-administrators
- [x] 5.10 Implement `GET /screens/:slug/members/:id/removal-impact` returning the minutes and revealed-tile count that deleting that member's entries would remove
- [x] 5.11 Implement `DELETE /screens/:slug/members/:id` taking an explicit retain-or-delete choice for that member's entries
- [x] 5.12 Implement `PATCH /screens/:slug` (admin only) for title edits, rejecting empty titles
- [x] 5.13 Implement geometry edits in `PATCH /screens/:slug` that regenerate the permutation, leave every entry untouched, and preserve the total logged time
- [x] 5.14 Implement `GET /admin/screens` (admin only) listing every screen with a flag for whether the caller participates in it

## 6. Board Derivation

- [x] 6.1 Implement the pure derivation function computing `revealed_count = LEAST(floor(total_minutes / unit_minutes), cols * rows)`
- [x] 6.2 Implement the attribution replay that walks entries in `created_at` order and maps each revealed tile index to the entry that crossed its threshold
- [x] 6.3 Implement the submit-time delta computation returning the tile indices attributable to a single new entry
- [x] 6.4 Write unit tests for the worked sequence at unit 5: +13 reveals tiles 0 and 1, +2 reveals tile 2, +4 reveals nothing, +1 reveals tile 3
- [x] 6.5 Write unit tests covering the cap at `cols * rows`, deletion rewinding the revealed count, and backdated entries not reshuffling attribution
- [x] 6.6 Verify the board response never includes unrevealed permutation entries, and add a test asserting it

## 7. Progress Entries

- [x] 7.1 Implement `POST /screens/:slug/entries` accepting hours 0–23 and minutes 0–59, rejecting a zero total, storing whole minutes, and returning the new revealed count with the submitter's tile indices
- [x] 7.2 Implement date handling storing `occurred_at` as a calendar date, defaulting to today, and rejecting future dates
- [x] 7.3 Implement `GET /screens/:slug/entries` returning only the caller's own entries, ordered by `occurred_at`
- [x] 7.4 Implement `PATCH /entries/:id` allowing a member to edit only their own entry's duration and date, setting `edited_at`
- [x] 7.5 Implement `DELETE /entries/:id` as a soft delete on the caller's own entry only, excluded from every total and from attribution
- [x] 7.6 Enforce that entries are accepted on fully revealed screens and rejected on archived screens
- [x] 7.7 Verify the entries API accepts no target-user parameter so no one can log on another member's behalf

## 8. Goals

- [x] 8.1 Implement `POST /screens/:slug/goals` for any member, validating non-empty text of at most 200 characters and a maximum of 50 active goals per screen
- [x] 8.2 Implement `GET /screens/:slug/goals` ordered by creation, omitting author identity for non-administrators and including it for administrators
- [x] 8.3 Include a per-goal flag telling the caller whether they are its author, so the client can show the delete control
- [x] 8.4 Implement `DELETE /goals/:id` permitting the author or any administrator, and rejecting other members with 403
- [x] 8.5 Reject goal creation and deletion on archived screens while keeping existing goals readable

## 9. Screen Lifecycle

- [x] 9.1 Implement `POST /screens/:slug/clone` (admin only) requiring a new title and a new picture selection, copying `cols`, `rows`, `unit_minutes`, members, and active goals
- [x] 9.2 Ensure a clone starts with no entries, a freshly generated permutation, a revealed count of zero, and its own unique slug, leaving the source screen untouched
- [x] 9.3 Implement `POST /screens/:slug/archive` and `POST /screens/:slug/unarchive` (admin only)
- [x] 9.4 Enforce archived read-only behavior across the entries and goals endpoints
- [x] 9.5 Implement `DELETE /screens/:slug` (admin only) requiring an exact title match in the request body, cascading to entries, goals, and membership while retaining the picture in the library

## 10. Frontend Foundation

- [x] 10.1 Configure the API client with `credentials: 'include'`, a base URL from an environment variable, and shared error handling that redirects to login on 401
- [x] 10.2 Implement the auth store holding the current user, hydrated from `GET /auth/me` on load
- [x] 10.3 Implement the login page and the route guard redirecting unauthenticated users to it
- [x] 10.4 Implement the account page for self-service password change
- [x] 10.5 Implement the screen list page showing only the user's screens with active and archived sections
- [x] 10.6 Implement the application shell, navigation, and a consistent error and loading presentation

## 11. Canvas Board

- [x] 11.1 Implement the layout that letterboxes the background picture to fit entirely within the viewport and computes the picture box rectangle
- [x] 11.2 Implement the canvas overlay aligned exactly to the picture box, with cells of `picture_width / cols` by `picture_height / rows`
- [x] 11.3 Implement cover tile drawing as flat gray cells with the border drawn inside each cell so revealed tiles leave no trace
- [x] 11.4 Implement resize handling that rescales picture and canvas together and keeps them aligned
- [x] 11.5 Ensure no redraw occurs while the board is static
- [ ] 11.6 Benchmark rendering and animation with a grid of at least 12,000 tiles, and fall back to dirty-rectangle redraws if full-canvas redraws miss frame budget
- [x] 11.7 Implement the screen header showing the title and participant count, and the footer showing total logged time and progress

## 12. Entry Submission and Reveal Animation

- [x] 12.1 Implement the progress entry form with separate hours and minutes inputs and a date picker defaulting to today
- [x] 12.2 Implement the submit flow that snaps tiles revealed by other members into place without animation, then animates only the tile indices the response attributes to this submission
- [x] 12.3 Implement the staggered per-tile fade and scale animation over roughly 400ms driven by `requestAnimationFrame`
- [x] 12.4 Ensure a cold page load renders the current board state with no animation regardless of elapsed progress
- [x] 12.5 Implement the personal history view listing only the caller's entries with edit and delete controls
- [x] 12.6 Implement edit and delete flows that re-fetch and re-render the board at its recomputed state

## 13. Completion Celebration

- [x] 13.1 Implement the celebration presentation shown when the board is fully revealed, on the completing submission and on every subsequent visit by any member
- [x] 13.2 Implement per-visit dismissal that leaves the revealed picture and total time visible
- [x] 13.3 Verify that entries on a completed screen continue increasing the displayed total while the revealed count stays at its maximum

## 14. Goals Panel

- [x] 14.1 Implement the collapsed goal control showing the goal count, overlaying the board without introducing page scrolling
- [x] 14.2 Implement the expanded panel listing goals in creation order with the add-goal control
- [x] 14.3 Show the delete control only on goals the caller authored, and on all goals for administrators
- [x] 14.4 Enforce the 200-character and 50-goal limits in the UI with clear messaging
- [x] 14.5 Render the panel read-only on archived screens

## 15. Admin Interface

- [x] 15.1 Implement the user management page for creating accounts, toggling the admin flag, and resetting passwords
- [x] 15.2 Implement the picture library page with upload, browsing, soft-delete, and hard-delete including the reference-count rejection message
- [x] 15.3 Implement the screen creation flow: title, picture upload or library selection, target total and unit entry, live geometry preview with the exact achievable total, and adjustable column count
- [x] 15.4 Implement the screen settings page for title edits and for geometry edits, showing the resulting tile and revealed counts with a warning that the picture may become less revealed
- [x] 15.5 Implement the membership management UI including the removal dialog with its retain-or-delete choice and the disclosed impact in minutes and tiles
- [x] 15.6 Implement the clone flow requiring a new title and picture, with goal pruning available on the resulting screen
- [x] 15.7 Implement archive and unarchive controls
- [x] 15.8 Implement screen deletion requiring the administrator to type the exact screen title
- [x] 15.9 Implement the administrator screen directory page listing every screen, with title filtering and an archived toggle

## 16. Deployment

- [ ] 16.1 Register the domain and point the web and API subdomains at Cloudflare
- [ ] 16.2 Provision the Oracle Ampere A1 instance, expecting repeated capacity failures, and document the fallback to an alternative VPS
- [ ] 16.3 Provision PostgreSQL, choosing between Neon and an instance on the VM, and record the decision
- [ ] 16.4 Provision the R2 bucket and credentials
- [x] 16.5 Install and configure Caddy for TLS termination in front of the axum service, and configure the Cloudflare proxy
- [x] 16.6 Create the systemd unit for the API with restart-on-failure and environment loaded from the deployed `.env`
- [x] 16.7 Configure the Cloudflare Pages project with `apps/web` as the build root and the API base URL as a build variable
- [ ] 16.8 Verify the end-to-end cookie flow across the web and API subdomains before considering deployment complete
- [x] 16.9 Set up a nightly `pg_dump` to R2 and verify a restore

## 17. Screen period and board tab shell

- [x] 17.1 Add migration `0008_screen_timeline.sql` with nullable `starts_on`, `ends_on`, and `auto_archive_days` on `screens`, plus CHECK constraints for ordering and non-negativity
- [x] 17.2 Add the window fields and a `ScreenPhase` with `phase`, `is_archived_on`, `auto_archive_on`, `accepts_writes`, and `covers` to `Screen`, deriving the phase from the stored window and today's date rather than storing it
- [x] 17.3 Widen `access::ensure_writable` to take today's date and reject the upcoming, ended, and archived phases with distinct messages, so entries and goals freeze together
- [x] 17.4 Add `access::ensure_in_window` and call it from entry creation and editing, so backdating cannot reach outside the period
- [x] 17.5 Add a wire `Phase`, a `ScreenWindow`, and the window fields to `ScreenDetail`, `ScreenSummary`, `AdminScreenEntry`, and the create, update, and clone requests; regenerate the TypeScript contract
- [x] 17.6 Accept and persist the window on create, update, and clone, validating it with a pure `validate_window`, and bucket the screen list on the derived archive state
- [x] 17.7 Clear `auto_archive_days` when unarchiving, so an automatically archived screen does not retire itself again
- [x] 17.8 Open `GET /screens/{slug}/members` to any member while keeping membership management administrator-only
- [x] 17.9 Unit-test the phase boundaries, the write gate per phase, the window bounds, and the window validator; extend the `ScreenDetail` contract test
- [x] 17.10 Add shell, safe-area, board-ink, and stacking tokens to `app.css` and document the 480px and 960px breakpoints
- [x] 17.11 Add `lib/screen.ts` holding the phase policy, and `formatHours` / `formatDayMonth` to `lib/format.ts`
- [x] 17.12 Rebuild `/s/[slug]` as a CSS grid shell: a band carrying the title, current and goal hours, meter, and period; a four-item tab bar; and a stage that gives `BoardCanvas` its box unchanged
- [x] 17.13 Drive the selected tab from a `?tab=` query parameter so one back press returns to the board, and guard the slug effect so switching tabs does not refetch the screen
- [x] 17.14 Convert `GoalsPanel` from a docked overlay to tab content and rename `HistoryPanel` to `RecordPanel`, replacing `archived` with `canEdit`
- [x] 17.15 Add `MembersPanel` listing every member by username with the viewer marked
- [x] 17.16 Rename the primary action to "Add Daimoku", centre and enlarge it, and bound the entry form's date picker by the period
- [x] 17.17 Lay the wide viewport out as a left rail, a top tab strip, the board, and a panel column, so a tab's content sits beside the picture rather than covering it
- [x] 17.18 Add a "Period" card to the screen creation and edit pages with start, end, and auto-archive term, and show the phase on the screen list cards
