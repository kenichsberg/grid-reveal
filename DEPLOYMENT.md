# Deployment

The application is two deploy targets from one repository: a static SPA on
Cloudflare Pages and a Rust binary on a VM.

```
  apps/web   SvelteKit + adapter-static  ->  Cloudflare Pages   app.<domain>
  apps/api   Rust / axum                 ->  Oracle Ampere A1   api.<domain>
                                              Caddy TLS, CF-proxied
```

## Prerequisites

Two things gate everything else. Secure both before you start.

**A registered domain.** The session model depends on it. The cookie is scoped
to a registrable domain shared by both hosts (`Domain=.example.com`), which
lets it be `SameSite=Lax` — CSRF-resistant without a separate token. Serving
the frontend from `*.pages.dev` against a bare VM address would force
`SameSite=None`, which is weaker and more fragile. About $10/year.

**An Oracle Always Free Ampere A1 instance.** Capacity is frequently
unavailable in popular regions; provisioning can take repeated attempts over
several days. Nothing in the design is Oracle-specific, so any small VPS works
as a fallback.

## 1. DNS

Point `app.<domain>` and `api.<domain>` at Cloudflare. Proxy both (orange
cloud). Proxying `api` also hides the VM's address.

Set the Cloudflare SSL mode to **Full (strict)** so the hop from Cloudflare to
your origin is verified, not just encrypted.

## 2. Database

Either option works; the application code is identical and the choice is just
`DATABASE_URL`.

- **Neon free tier** — managed, with backups handled for you. Suspends when
  idle, adding roughly 500ms to the first request after a quiet period.
- **PostgreSQL on the VM** — no cold start and one less vendor, on a machine
  with 24 GB of RAM. Backups become yours: use `deploy/backup.sh`.

Migrations run automatically at API startup, so there is no separate step.

## 3. Object storage

Create a Cloudflare R2 bucket and an API token with read and write access.
Fill in `R2_ENDPOINT`, `R2_BUCKET`, `R2_ACCESS_KEY_ID`, and
`R2_SECRET_ACCESS_KEY`.

If you give the bucket a public domain, set `R2_PUBLIC_BASE_URL` and pictures
are served directly by Cloudflare. Leave it empty and the API streams them
itself, which is simpler and keeps pictures behind authentication.

Leaving `R2_ENDPOINT` empty falls back to local disk storage. That is for
development only — it ties your pictures to one free-tier instance's lifetime.

## 4. Build and install the API

On the VM (or cross-compile for `aarch64-unknown-linux-gnu`):

```bash
cd apps/api
cargo build --release
```

The `webp` crate compiles vendored libwebp, so the build host needs a C
compiler (`build-essential` on Debian/Ubuntu).

Then:

```bash
sudo useradd --system --home /opt/grid-reveal gridreveal
sudo mkdir -p /opt/grid-reveal/data
sudo cp target/release/grid-reveal-api /opt/grid-reveal/
sudo cp ../../deploy/backup.sh /opt/grid-reveal/
sudo cp /path/to/your/.env /opt/grid-reveal/.env
sudo chown -R gridreveal:gridreveal /opt/grid-reveal
sudo chmod 600 /opt/grid-reveal/.env

sudo cp ../../deploy/grid-reveal-api.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now grid-reveal-api
```

On first boot the API creates an administrator from `ADMIN_USERNAME` and
`ADMIN_PASSWORD` if no administrator exists. Once one exists, changing those
variables does nothing — they are a bootstrap, not a source of truth.

Set a real `ADMIN_PASSWORD` before the first boot. Change it from the account
page afterwards.

## 5. Caddy

```bash
sudo cp deploy/Caddyfile /etc/caddy/Caddyfile
# edit api.example.com to your domain
sudo systemctl reload caddy
```

Open ports 80 and 443 in both the Oracle security list **and** the instance's
own firewall. Oracle images ship with iptables rules that block traffic even
when the security list permits it — this catches people out:

```bash
sudo iptables -I INPUT -p tcp --dport 443 -j ACCEPT
sudo iptables -I INPUT -p tcp --dport 80 -j ACCEPT
sudo netfilter-persistent save
```

## 6. Cloudflare Pages

Create a Pages project from the repository:

- **Build command:** `npm run build`
- **Build output directory:** `build`
- **Root directory:** `apps/web`
- **Environment variable:** `PUBLIC_API_BASE_URL=https://api.<domain>`

The SPA falls back to `index.html`, so deep links like `/s/k3m9xq2p` resolve.

## 7. Configure CORS and cookies

In the API's `.env`:

```
COOKIE_DOMAIN=.example.com
COOKIE_SECURE=true
CORS_ALLOWED_ORIGINS=https://app.example.com
```

Credentialed CORS cannot use a wildcard origin, so this list must name the web
host exactly.

## 8. Verify the cookie flow

This is the integration most likely to be subtly wrong, so check it before
anything else:

1. Open `https://app.<domain>` and sign in as the bootstrap administrator.
2. In devtools, confirm the `grid_session` cookie has `Domain=.<domain>`,
   `Secure`, `HttpOnly`, and `SameSite=Lax`.
3. Reload. You should stay signed in — that proves the cookie is being sent
   cross-subdomain.
4. Create a picture and a screen, then log progress and watch tiles reveal.

If step 3 fails, the usual cause is `COOKIE_DOMAIN` not matching both hosts.

## 9. Backups

If you self-host PostgreSQL:

```bash
sudo crontab -e
# 15 3 * * * /opt/grid-reveal/backup.sh >> /var/log/grid-reveal-backup.log 2>&1
```

Restore one into a scratch database before trusting it. A backup you have
never restored is a guess.

## Rollback

Redeploy a prior commit. Migrations are the only destructive step; since there
is no production data before launch, forward-fix is the expected path.
