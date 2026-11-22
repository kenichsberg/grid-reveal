# Grid Reveal

A shared progress tracker. Members log time against a screen; every unit of
time logged removes one tile at random from a grid covering a background
picture, so the image is revealed gradually by the group's combined effort.

## Layout

```
apps/web/   SvelteKit SPA (adapter-static) -> Cloudflare Pages
apps/api/   Rust / axum API                -> Oracle Cloud VM behind Caddy
deploy/     Caddyfile, systemd unit, backup script
scripts/    gen-types.sh (Rust DTOs -> TypeScript)
```

## How the board works

The revealed state is derived, never stored:

```
total_minutes   = SUM(entries.minutes) WHERE deleted_at IS NULL
revealed_count  = min( floor(total_minutes / unit_minutes), cols * rows )
revealed tiles  = permutation[0 .. revealed_count)
```

Each screen holds a fixed random permutation of its tile indices. Because the
board is a function of a sum, concurrent submissions cannot race, and editing
or deleting an entry rewinds the board correctly for free.

## How someone joins

There is no signup form. An administrator generates an invite link for one
screen:

```
https://<web-host>/join/<24-character token>
```

Opening it shows the screen's title and picture. Accepting it creates the
account, joins it to that screen, and signs the person in, all in one
transaction — so a username that turns out to be taken costs them nothing and
the link still works. Each link admits one person, and the administrator never
sees the password.

An invite has no expiry date of its own. It is open while the screen is
upcoming or active, and dead once the period ends — which also means extending
a screen's end date revives its unused links.

## Every screen has a period

A screen is a stretch of time with a picture at the end of it, so `starts_on`
and `ends_on` are NOT NULL and have no "open indefinitely" case:

```
today < starts_on   upcoming  visible, read-only
today in [a, b]     active    the only phase that accepts writes
today > ends_on     ended     read-only, board and celebration intact
```

The start date is pre-filled in the browser with the administrator's own day,
never the server's — see below for why those differ. The retention term that
archives a screen after its period is still optional; the period itself is not.

## Whose day it is

Dates here are plain calendar dates, never instants, so something has to decide
which day a request happens on. The caller does, by sending its own date:

```
X-Client-Date: 2026-11-19
```

The server's clock is UTC, which is a different day from the member's for a few
hours either side of their local midnight. Without the header, a session logged
at 00:30 east of UTC is refused as a future entry, and a screen whose period
opens that day reports that it has not started.

The header is optional. A request without one is judged by the server's UTC
date, which is what the API did before the header existed. A date further than
one day from the server's own is ignored rather than rejected, so a wrong device
clock degrades to UTC instead of moving a screen's deadline.

## Running locally

### With Docker (nothing to install)

```bash
docker compose up --build
```

Then open <http://localhost:5173> and sign in as `admin` / `admin-dev-password`.

Brings up PostgreSQL, the API on :8080, and the built frontend on :5173.
Migrations run at API startup and the bootstrap administrator is seeded on
first boot. Pictures go to a local volume, since no R2 bucket is configured.

```bash
docker compose down        # stop
docker compose down -v     # stop and discard the database and pictures
docker compose logs -f api # follow the API log
```

To iterate on the frontend with hot reload, leave `db` and `api` running and
start Vite on the host — it uses the same port, so CORS already allows it:

```bash
docker compose up -d db api
cd apps/web && npm install && npm run dev
```

### Without Docker

```bash
cp .env.example .env          # fill in DATABASE_URL and the admin bootstrap

cd apps/api && cargo run      # migrations run at startup, on :8080
cd apps/web && npm install && npm run dev   # on :5173
```

`/dev/bench` renders a board at up to 20,000 tiles and reports frame times.

After changing `apps/api/src/dto.rs`, regenerate the TypeScript contract:

```bash
./scripts/gen-types.sh
```

## Testing

```bash
cd apps/api && cargo test     # 156 tests
cd apps/web && npm run check  # svelte-check
```

## Deploying

See [DEPLOYMENT.md](DEPLOYMENT.md). Two things gate everything else: a
registered domain (the session cookie model depends on it) and an Oracle
Always Free A1 instance (capacity is often unavailable).

Specs and design live in `openspec/changes/grid-reveal-app/`.
