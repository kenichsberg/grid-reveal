## Why

"Grid Reveal" is written into the client in fourteen places: the header brand,
the headings on the sign-in and invitation pages, and the `· Grid Reveal`
suffix on every page title. A group running this for themselves cannot call it
what they call it, and the name is the first thing anyone sees.

It cannot be an environment variable. Everything instance-wide so far is one —
read once at boot into `Config` — but the point of this setting is that an
administrator changes it from inside the application, and `bootstrap.rs`
already states the principle that the environment is a bootstrap rather than a
source of truth.

It also cannot ride on `GET /auth/me`. The sign-in page shows the name to
someone who has no session, which is the whole reason they are on that page.

So the name becomes the first configuration the database owns, read by anyone
and written only by an administrator.

## What Changes

- A `site_settings` table holding one row, enforced by a boolean primary key
  that may only be `TRUE`, seeded with `Grid Reveal` so every read finds a row.
- `GET /settings` is public — the second kind of public endpoint here, after
  the invite peek — and answers `no-store`, since a rename must show on the
  next load.
- `PATCH /settings` takes `AdminUser` and validates the name as every other
  string here is validated: trimmed, non-empty, 60 characters or fewer, with
  the database repeating the rule as a CHECK constraint.
- The client reads the name from one store, hydrated beside the session, and
  every page title goes through a `pageTitle()` helper. Nothing in the app says
  "Grid Reveal" any more unless that is the configured name.
- A new **Site settings** page under `/admin`, with the name on it.

## Non-goals

- Any other instance-wide setting. The table has room for a second column, and
  the page has room for a second field, when something earns one.
- Per-screen wording, which already exists as `progress_title` and
  `entry_noun`.
- A logo, favicon, or colour. The name is text; those are assets, and asset
  upload is a different problem.
