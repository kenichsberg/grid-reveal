## 1. Let the database hold it

- [x] 1.1 Add `apps/api/migrations/0012_site_settings.sql`: one row, `id BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (id)`, which makes a second row collide rather than merely discouraging it
- [x] 1.2 Seed `Grid Reveal` in the same migration, so no handler carries a "what if it is unset" branch
- [x] 1.3 Repeat the validator's rule as `site_settings_app_name_shape`, as `0009_screen_labels.sql` does for the per-screen labels

## 2. Serve it

- [x] 2.1 Add `SiteSettings` and `UpdateSettingsRequest` to `apps/api/src/dto.rs` under a new `Site` banner
- [x] 2.2 Add `apps/api/src/routes/settings.rs` with `GET`/`PATCH /settings`, and register it in `routes/mod.rs`
- [x] 2.3 `read` names no auth extractor, which is how a handler is public here; the module doc-comment says why, as `invites.rs` does
- [x] 2.4 Answer `Cache-Control: no-store` on the read — the first endpoint here a shared cache could hold
- [x] 2.5 `update` takes `_admin: AdminUser` and `validate_app_name`, modelled on `validate_title`, with the inline unit test every validator here has
- [x] 2.6 Run `scripts/gen-types.sh` and add `settings` / `updateSettings` to `apps/web/src/lib/api/client.ts`, the read `quiet` like the other public call

## 3. Read it everywhere

- [x] 3.1 Add `apps/web/src/lib/stores/site.svelte.ts`, shaped like the auth store, with a `FALLBACK_NAME` for an API that cannot answer
- [x] 3.2 Export `pageTitle(part?)` from it, so renaming the installation renames every browser tab with it
- [x] 3.3 Hydrate it beside `auth.hydrate()` in `+layout.svelte` and widen the boot gate to `auth.loading || site.loading`, so nobody watches the fallback turn into the real name
- [x] 3.4 Brand in the header, and the `<h1>` on the sign-in and invitation pages, become `{site.name}`
- [x] 3.5 Every `<svelte:head><title>… · Grid Reveal</title>` becomes `{pageTitle('…')}` — eleven pages

## 4. Let an administrator change it

- [x] 4.1 Add `apps/web/src/routes/admin/settings/+page.svelte`: one field, the same `$effect` admin guard and inline error handling the other admin pages use
- [x] 4.2 Seed the field from `site.name` rather than fetching: the layout already hydrated it
- [x] 4.3 Save through `site.rename`, so the header retitles itself with no reload, and show the trimmed name the server stored rather than what was typed
- [x] 4.4 Add the fifth card to the `/admin` menu

## 5. Prove it

- [x] 5.1 Add `apps/api/tests/site_settings.rs` on the `screen_invites.rs` harness, which is the one that already drives unauthenticated endpoints
- [x] 5.2 A stranger reads the name; an administrator renames it; the stranger reads the new one — asserting against the first read, since the row is shared with every other suite
- [x] 5.3 No session is 401, a non-administrator is 403, blank is 400, 61 characters is 400 — and none of the refused names becomes the name
- [x] 5.4 Restore the name afterwards: every other row these suites leave behind is invisible, and this one would rename the development installation
- [x] 5.5 `cargo test` (161) and `npm run check` both clean
