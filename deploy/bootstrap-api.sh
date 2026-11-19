#!/usr/bin/env bash
# Configures a fresh API host from nothing to serving.
#
# Usage:
#   sudo ./bootstrap-api.sh grid-reveal-api.example.com [version]
#
# `version` is a release tag; the latest release is used when it is omitted.
#
# Expects /opt/grid-reveal/.env to exist already. Secrets are not arguments and
# are not fetched: you put that file there, this script configures around it.
#
# Idempotent. Re-running after a failure part-way through is the normal case,
# so every step checks whether it has already been done.
#
# What it cannot do, because neither lives on this host: the Oracle security
# list and the DNS record. It checks both at the end and says what is missing.
set -euo pipefail

REPO="${REPO:-kenichsberg/grid-reveal}"
INSTALL_DIR="${INSTALL_DIR:-/opt/grid-reveal}"
ENV_FILE="$INSTALL_DIR/.env"
SERVICE_USER="${SERVICE_USER:-gridreveal}"
SWAP_SIZE="${SWAP_SIZE:-2G}"

say()  { echo "bootstrap: $*"; }
skip() { echo "bootstrap: $* (already done)"; }
die()  { echo "bootstrap: $*" >&2; exit 1; }

[ "$(id -u)" -eq 0 ] || die "run with sudo"

domain="${1:-}"
[ -n "$domain" ] || die "usage: $0 <api-domain> [version]"
version="${2:-}"

[ -f "$ENV_FILE" ] || die "$ENV_FILE does not exist; create it from .env.example first"

if [ -z "$version" ]; then
  version="$(curl -fsSLI -o /dev/null -w '%{url_effective}' \
    "https://github.com/$REPO/releases/latest" | sed 's#.*/tag/##')"
  [ -n "$version" ] || die "could not determine the latest release"
fi
RAW="https://raw.githubusercontent.com/$REPO/$version"
say "configuring for $domain from $version"

# --- packages ---------------------------------------------------------------
# iptables-persistent asks two questions on install and would hang an
# unattended run; answering them in advance is the only way past it.
export DEBIAN_FRONTEND=noninteractive
echo 'iptables-persistent iptables-persistent/autosave_v4 boolean true' | debconf-set-selections
echo 'iptables-persistent iptables-persistent/autosave_v6 boolean true' | debconf-set-selections

say "installing packages"
apt-get update -qq
apt-get install -y -qq \
  curl ca-certificates gnupg debian-keyring debian-archive-keyring \
  apt-transport-https postgresql-client awscli iptables-persistent

# --- service user and directories -------------------------------------------
if id -u "$SERVICE_USER" >/dev/null 2>&1; then
  skip "user $SERVICE_USER"
else
  say "creating user $SERVICE_USER"
  useradd --system --home "$INSTALL_DIR" "$SERVICE_USER"
fi
mkdir -p "$INSTALL_DIR/data"

# --- swap -------------------------------------------------------------------
# 1 GB beside an image decode that may take 128 MB. The swap is a backstop
# against the OOM killer choosing for us, not storage we intend to use.
if swapon --show | grep -q '^/swapfile'; then
  skip "swap"
else
  say "creating ${SWAP_SIZE} swapfile"
  fallocate -l "$SWAP_SIZE" /swapfile 2>/dev/null \
    || dd if=/dev/zero of=/swapfile bs=1M count=2048 status=none
  # Swap can hold anything that was in memory, passwords included.
  chmod 600 /swapfile
  mkswap /swapfile >/dev/null
  swapon /swapfile
fi
grep -q '^/swapfile' /etc/fstab || echo '/swapfile none swap sw 0 0' >> /etc/fstab
echo 'vm.swappiness=10' > /etc/sysctl.d/99-swappiness.conf
sysctl -q -w vm.swappiness=10

# --- firewall ---------------------------------------------------------------
# -I, never -A: these images ship a REJECT rule near the end of INPUT, and an
# appended ACCEPT lands below it and does nothing at all.
for port in 80 443; do
  if iptables -C INPUT -p tcp --dport "$port" -j ACCEPT 2>/dev/null; then
    skip "iptables $port"
  else
    say "opening port $port"
    iptables -I INPUT -p tcp --dport "$port" -j ACCEPT
  fi
done
netfilter-persistent save >/dev/null

# --- Caddy ------------------------------------------------------------------
if command -v caddy >/dev/null 2>&1; then
  skip "caddy"
else
  say "installing caddy"
  curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' \
    | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
  curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' \
    > /etc/apt/sources.list.d/caddy-stable.list
  apt-get update -qq
  apt-get install -y -qq caddy
fi

say "writing Caddyfile for $domain"
curl -fLsS -o /etc/caddy/Caddyfile "$RAW/deploy/Caddyfile"
sed -i "s/grid-reveal-api\.example\.com/$domain/" /etc/caddy/Caddyfile
systemctl reload caddy || systemctl restart caddy

# --- scripts and units ------------------------------------------------------
say "installing scripts and units"
for f in update-api.sh backup.sh; do
  curl -fLsS -o "$INSTALL_DIR/$f" "$RAW/deploy/$f"
  chmod +x "$INSTALL_DIR/$f"
done
for u in grid-reveal-api.service grid-reveal-backup.service grid-reveal-backup.timer; do
  curl -fLsS -o "/etc/systemd/system/$u" "$RAW/deploy/$u"
done
chown -R "$SERVICE_USER:$SERVICE_USER" "$INSTALL_DIR"
chmod 600 "$ENV_FILE"
systemctl daemon-reload
systemctl enable -q grid-reveal-api
systemctl enable -q --now grid-reveal-backup.timer

# --- the binary -------------------------------------------------------------
# update-api.sh verifies the checksum, restarts, polls /health, and rolls back
# if the new binary is silent, so it is also the right way to install the first.
say "installing the API binary"
"$INSTALL_DIR/update-api.sh" "$version"

# --- what is not on this host -----------------------------------------------
echo
say "host is configured. Checking what is not on it:"

resolved="$(getent hosts "$domain" | awk '{print $1}' | head -1 || true)"
if [ -n "$resolved" ]; then
  echo "  DNS: $domain resolves to $resolved"
else
  echo "  DNS: $domain does not resolve — add the record before Caddy can get a certificate"
fi

if curl -fsS -m 10 -o /dev/null "https://$domain/health" 2>/dev/null; then
  echo "  HTTPS: https://$domain/health answers"
else
  cat <<'NOTE'
  HTTPS: not answering yet. In order, the usual causes:
    - the Oracle security list has no ingress for 80 and 443 (it drops silently,
      so this looks like a timeout rather than a refusal)
    - the DNS record is proxied, and Caddy cannot complete an HTTP-01 challenge
      through it; grey-cloud until the certificate exists, then re-proxy
    - watch it happen with: journalctl -u caddy -f
NOTE
fi
