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
cd apps/api && cargo test     # 83 tests
cd apps/web && npm run check  # svelte-check
```

## Deploying

See [DEPLOYMENT.md](DEPLOYMENT.md). Two things gate everything else: a
registered domain (the session cookie model depends on it) and an Oracle
Always Free A1 instance (capacity is often unavailable).

Specs and design live in `openspec/changes/grid-reveal-app/`.
