# Deployment

The application is two deploy targets from one repository: a static SPA on
Cloudflare Pages and a Rust binary on a VM.

```
  apps/web   SvelteKit + adapter-static  ->  Cloudflare Pages      grid-reveal.<domain>
  apps/api   Rust / axum                 ->  Oracle AMD micro VM   grid-reveal-api.<domain>
                                                Caddy TLS, CF-proxied
  database   PostgreSQL                  ->  Oracle AMD micro VM   VCN-internal name only
```

## Prerequisites

Two things gate everything else. Secure both before you start.

**A registered domain.** The session model depends on it. The cookie is scoped
to a registrable domain shared by both hosts (`Domain=.example.com`), which
lets it be `SameSite=Lax` — CSRF-resistant without a separate token. Serving
the frontend from `*.pages.dev` against a bare VM address would force
`SameSite=None`, which is weaker and more fragile. About $10/year.

**Two Oracle Always Free AMD micro instances**, in the same VCN: one for the
API, one for PostgreSQL. Always Free includes exactly two, so this uses the
allowance rather than exceeding it. Each is 1/8 OCPU and 1 GB of RAM, which is
why they are separated — see section 2. Ampere A1 shapes are larger and often
unobtainable; nothing here is Oracle-specific, so any two small VPSs work.

## 1. DNS

Point `grid-reveal.<domain>` and `grid-reveal-api.<domain>` at Cloudflare.
Proxy both (orange cloud). Proxying the API host also hides the VM's address.

Both are deliberately **first-level** subdomains. Cloudflare's free Universal
SSL covers the apex and one level only, so a name like
`grid-reveal.api.<domain>` would need Total TLS or Advanced Certificate
Manager — a paid add-on to say the same thing a hyphen says for nothing.

The database VM gets no public DNS record. Give the instance the hostname
`grid-reveal-db` when you create it and Oracle's internal resolver answers for
`grid-reveal-db.<subnet>.<vcn>.oraclevcn.com` inside the VCN, which is the
readable name without the exposure. A public record would either publish the
database host's address or, pointing at an RFC 1918 address, be discarded by
resolvers that reject private addresses in public answers.

Set the Cloudflare SSL mode to **Full (strict)** so the hop from Cloudflare to
your origin is verified, not just encrypted.

## 2. Oracle networking

Both instances live in **one VCN, in the same public subnet**, so they can
reach each other over private addresses at no cost and with no hop through the
internet. Create it with the **VCN with Internet Connectivity** wizard, which
produces the subnet, internet gateway and route table together; launching the
instances into it is the only networking choice that matters.

The database VM keeps a public address so it can fetch packages and updates,
but Postgres never listens on it — `listen_addresses` binds the private address
only (section 3). A public address on the instance and a service exposed to the
internet are different things, and conflating them is what makes people reach
for a NAT gateway they do not need.

**Reserve the API VM's public IP**, before pointing any DNS at it. Oracle hands
out ephemeral addresses by default and they change when an instance is stopped
and started, which would break the `api` record with nobody having touched it.

The setting is on the address, not on the instance or the VNIC:
*Instance > Attached VNICs > the VNIC > IPv4 Addresses > the row's ⋮ > Edit*.

It takes two passes, because ephemeral cannot be converted in place: edit once
to **No public IP**, which releases the current address, then edit again to
**Reserved public IP** and create one. The instance comes back on a different
address than it had — harmless now, considerably less so once DNS names it.

Security list ingress rules for the subnet:

| Port | Source | Why |
|------|--------|-----|
| 22   | your own address, not `0.0.0.0/0` | SSH |
| 80   | `0.0.0.0/0` | Let's Encrypt HTTP-01, and Cloudflare |
| 443  | `0.0.0.0/0` | Cloudflare |
| 5432 | the API VM's private address, `/32` | the only client the database has |

Restricting 80 and 443 to [Cloudflare's published ranges](https://www.cloudflare.com/ips/)
is worth doing once this is running: with the record proxied, nothing else has
any business reaching the origin directly.

Each instance's own `iptables` must be opened as well — see sections 3 and 6.
Oracle's Ubuntu images drop traffic the security list has already permitted,
and the resulting silence looks exactly like a security-list mistake.

## 3. Database

PostgreSQL runs on the second micro instance and is reached over the VCN's
private network. The application code is identical whatever you point it at;
the choice is only `DATABASE_URL`.

Keeping it off the API host matters more at this size than it would on a large
machine. A 1 GB instance has no room to hold a Postgres buffer pool *and* the
decode spike an image upload causes, which `apps/api/src/images.rs` caps at
128 MB. Separating them means neither can starve the other, and a runaway query
cannot take the API down with it.

On the database VM:

```bash
sudo apt install -y postgresql
sudo -u postgres createuser --pwprompt griduser
sudo -u postgres createdb --owner=griduser griddb
```

Then bind it to the private address only — never `0.0.0.0` — in
`/etc/postgresql/*/main/postgresql.conf`:

```
listen_addresses = '10.0.0.<db>'
```

and admit exactly one client, in `pg_hba.conf`:

```
host  griddb  griduser  10.0.0.<api>/32  scram-sha-256
```

Open 5432 in the Oracle security list **from the API VM's private address
only**, and remember the instance's own `iptables` as well — Ubuntu images ship
with rules that drop what the security list permits:

```bash
sudo iptables -I INPUT -p tcp -s 10.0.0.<api> --dport 5432 -j ACCEPT
sudo netfilter-persistent save
```

### Sizing for 1 GB

Defaults assume a far larger machine. In `postgresql.conf`:

```
shared_buffers = 192MB
effective_cache_size = 512MB
work_mem = 4MB
max_connections = 20
```

`max_connections` is deliberately close to the API's pool of 10
(`apps/api/src/db.rs`): each backend costs memory, and nothing else connects.
Add 2 GB of swap on both VMs as a backstop.

### Connecting

On the API host:

```
DATABASE_URL=postgres://griduser:<password>@10.0.0.<db>:5432/griddb?sslmode=require
```

Debian and Ubuntu's `postgresql` package enables TLS with a self-signed
certificate out of the box, so `sslmode=require` works with nothing further to
configure. It encrypts the hop without verifying the certificate, which is the
right trade inside a subnet you control and strictly better than plaintext.

Migrations run automatically at API startup, so there is no separate step.

### Backups

`deploy/backup.sh` runs on the API host and reaches the database over the
network, so install `postgresql-client` there. Its `pg_dump` must be at least
the server's major version. Backups are now yours, not a provider's — and a
backup you have never restored is a guess.

## 4. Object storage

Create a Cloudflare R2 bucket and an API token with read and write access.
Fill in `R2_ENDPOINT`, `R2_BUCKET`, `R2_ACCESS_KEY_ID`, and
`R2_SECRET_ACCESS_KEY`.

If you give the bucket a public domain, set `R2_PUBLIC_BASE_URL` and pictures
are served directly by Cloudflare. Leave it empty and the API streams them
itself, which is simpler and keeps pictures behind authentication.

Leaving `R2_ENDPOINT` empty falls back to local disk storage. That is for
development only — it ties your pictures to one free-tier instance's lifetime.

## 5. Build and install the API

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

## 6. Caddy

```bash
sudo cp deploy/Caddyfile /etc/caddy/Caddyfile
# edit grid-reveal-api.example.com to your domain
sudo systemctl reload caddy
```

### Getting the first certificate

Cloudflare must be **temporarily unproxied** for Caddy's first issuance. With
the record proxied and SSL mode Full (strict), Cloudflare reaches the origin
over HTTPS whatever scheme the client used — including Let's Encrypt's HTTP-01
challenge — and the origin has no certificate yet, so the challenge fails on
the very thing it was going to fix.

Grey-cloud the `api` record, let Caddy obtain the certificate, then turn the
orange cloud back on. Renewals are unaffected: by then the origin has a valid
certificate, so the proxied HTTPS hop succeeds.

The alternative, if that dance annoys you, is a Cloudflare **Origin CA**
certificate — valid 15 years, trusted by Cloudflare only, which is all a
proxied origin needs. Point Caddy at it with `tls <cert> <key>` and ACME never
runs.

### Firewall

Open ports 80 and 443 in both the Oracle security list **and** the instance's
own firewall. Oracle images ship with iptables rules that block traffic even
when the security list permits it — this catches people out:

```bash
sudo iptables -I INPUT -p tcp --dport 443 -j ACCEPT
sudo iptables -I INPUT -p tcp --dport 80 -j ACCEPT
sudo netfilter-persistent save
```

## 7. Cloudflare Pages

The frontend is built and deployed by `.github/workflows/deploy-web.yml`, not by
Cloudflare's Git integration: the build definition belongs in this repository
rather than in a dashboard, and Cloudflare never needs access to the source.
The project itself is created by the same workflow on its first run, so there is
nothing to click.

Give it its credentials once, from a terminal:

```bash
gh secret   set CLOUDFLARE_API_TOKEN     # Account > Cloudflare Pages > Edit
gh secret   set CLOUDFLARE_ACCOUNT_ID    # `wrangler whoami`, or the dashboard URL
gh variable set PUBLIC_API_BASE_URL      # https://grid-reveal-api.<domain>
```

The token is made at **My Profile > API Tokens > Create Token > Custom token**
with the single permission *Account - Cloudflare Pages - Edit*, scoped to this
account.

Then run the workflow (`gh workflow run deploy-web.yml`, or push to `main`).
Every later push that touches `apps/web/` redeploys on its own.

Two consequences worth knowing:

- `PUBLIC_API_BASE_URL` is read at build time by `$env/static/public`, so it is
  compiled into the bundle. Changing the API host means re-running the workflow,
  not restarting anything.
- `apps/web/static/_redirects` ships as `build/_redirects` and serves the shell
  for every path, so deep links like `/s/k3m9xq2p` resolve instead of 404ing.

Add the custom domain once the first deployment is live:

```bash
curl -X POST \
  "https://api.cloudflare.com/client/v4/accounts/$CLOUDFLARE_ACCOUNT_ID/pages/projects/grid-reveal/domains" \
  -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
  -H "Content-Type: application/json" \
  --data '{"name":"grid-reveal.<domain>"}'
```

Cloudflare creates the DNS record itself when the zone is in the same account.
If this is refused for permissions, either add *Zone - DNS - Edit* for that zone
to the token or add the `CNAME` to `grid-reveal.pages.dev` by hand.

## 8. Configure CORS and cookies

In the API's `.env`:

```
COOKIE_DOMAIN=.example.com
COOKIE_SECURE=true
CORS_ALLOWED_ORIGINS=https://grid-reveal.example.com
```

Credentialed CORS cannot use a wildcard origin, so this list must name the web
host exactly.

## 9. Verify the cookie flow

This is the integration most likely to be subtly wrong, so check it before
anything else:

1. Open `https://grid-reveal.<domain>` and sign in as the bootstrap administrator.
2. In devtools, confirm the `grid_session` cookie has `Domain=.<domain>`,
   `Secure`, `HttpOnly`, and `SameSite=Lax`.
3. Reload. You should stay signed in — that proves the cookie is being sent
   cross-subdomain.
4. Create a picture and a screen, then log progress and watch tiles reveal.

If step 3 fails, the usual cause is `COOKIE_DOMAIN` not matching both hosts.

## 10. Backups

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
