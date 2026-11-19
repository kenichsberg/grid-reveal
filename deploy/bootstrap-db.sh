#!/usr/bin/env bash
# Configures a fresh PostgreSQL host for the API.
#
# Usage:
#   sudo PGPASSWORD='...' ./bootstrap-db.sh <api-private-ip> [db-private-ip]
#
# The database VM's own private address is taken from the host when the second
# argument is omitted. The password is read from PGPASSWORD or prompted for;
# it is never an argument, because arguments are visible in ps.
#
# Idempotent: every step checks whether it has already been done.
set -euo pipefail

DB_NAME="${DB_NAME:-griddb}"
DB_USER="${DB_USER:-griduser}"
SWAP_SIZE="${SWAP_SIZE:-2G}"

say()  { echo "bootstrap-db: $*"; }
skip() { echo "bootstrap-db: $* (already done)"; }
die()  { echo "bootstrap-db: $*" >&2; exit 1; }

[ "$(id -u)" -eq 0 ] || die "run with sudo"

api_ip="${1:-}"
[ -n "$api_ip" ] || die "usage: $0 <api-private-ip> [db-private-ip]"
db_ip="${2:-$(hostname -I | tr ' ' '\n' | grep -E '^10\.' | head -1)}"
[ -n "$db_ip" ] || die "could not determine this host's private address; pass it as the second argument"

if [ -z "${PGPASSWORD:-}" ]; then
  read -rsp "Password for $DB_USER (must match DATABASE_URL on the API host): " PGPASSWORD
  echo
fi
[ -n "$PGPASSWORD" ] || die "a password is required"

say "database $db_ip, admitting the API at $api_ip"

# --- packages ---------------------------------------------------------------
export DEBIAN_FRONTEND=noninteractive
echo 'iptables-persistent iptables-persistent/autosave_v4 boolean true' | debconf-set-selections
echo 'iptables-persistent iptables-persistent/autosave_v6 boolean true' | debconf-set-selections

say "installing packages"
apt-get update -qq
apt-get install -y -qq postgresql iptables-persistent

conf_dir="$(dirname "$(find /etc/postgresql -name postgresql.conf | sort | tail -1)")"
[ -d "$conf_dir" ] || die "could not find a postgresql.conf under /etc/postgresql"

# --- role and database ------------------------------------------------------
if [ "$(sudo -u postgres psql -tAc "SELECT 1 FROM pg_roles WHERE rolname='$DB_USER'")" = "1" ]; then
  skip "role $DB_USER"
else
  say "creating role $DB_USER"
  # Piped, not passed as -c: an argument is visible in ps to every user on the
  # box for as long as the command runs. printf is a shell builtin, so the
  # password never becomes a process argument either.
  # sed rather than bash parameter substitution: quote handling in ${v//'/''}
  # differs between bash 3 and 5, and the password reaches sed on stdin, so it
  # never appears in any process's arguments.
  escaped="$(printf '%s' "$PGPASSWORD" | sed "s/'/''/g")"
  printf "CREATE ROLE %s LOGIN PASSWORD '%s';\n" "$DB_USER" "$escaped" \
    | sudo -u postgres psql -q -v ON_ERROR_STOP=1
  unset escaped
fi

if [ "$(sudo -u postgres psql -tAc "SELECT 1 FROM pg_database WHERE datname='$DB_NAME'")" = "1" ]; then
  skip "database $DB_NAME"
else
  say "creating database $DB_NAME"
  sudo -u postgres createdb --owner="$DB_USER" "$DB_NAME"
fi

# --- configuration ----------------------------------------------------------
# A drop-in rather than sed against postgresql.conf: Debian's packaging already
# includes conf.d, the file survives a package upgrade, and it keeps our
# settings legible as ours.
say "writing $conf_dir/conf.d/10-grid-reveal.conf"
mkdir -p "$conf_dir/conf.d"
cat > "$conf_dir/conf.d/10-grid-reveal.conf" <<EOF
# Managed by deploy/bootstrap-db.sh. Edits here are overwritten on a re-run.

# The private address only. Never 0.0.0.0 on a host with a public interface.
listen_addresses = '$db_ip'

# Sized for a 1 GB instance; the defaults assume a far larger machine.
shared_buffers = 192MB
effective_cache_size = 512MB
work_mem = 4MB

# Deliberately close to the API's pool of 10 (apps/api/src/db.rs). Each backend
# costs memory, and nothing else connects to this database.
max_connections = 20
EOF

hba="$conf_dir/pg_hba.conf"
hba_line="host    $DB_NAME    $DB_USER    $api_ip/32    scram-sha-256"
if grep -qF "$api_ip/32" "$hba"; then
  skip "pg_hba entry for $api_ip"
else
  say "admitting $api_ip in pg_hba.conf"
  echo "$hba_line" >> "$hba"
fi

# --- firewall ---------------------------------------------------------------
if iptables -C INPUT -p tcp -s "$api_ip" --dport 5432 -j ACCEPT 2>/dev/null; then
  skip "iptables 5432"
else
  say "opening 5432 to $api_ip"
  iptables -I INPUT -p tcp -s "$api_ip" --dport 5432 -j ACCEPT
fi
netfilter-persistent save >/dev/null

# --- swap -------------------------------------------------------------------
if swapon --show | grep -q '^/swapfile'; then
  skip "swap"
else
  say "creating ${SWAP_SIZE} swapfile"
  fallocate -l "$SWAP_SIZE" /swapfile 2>/dev/null \
    || dd if=/dev/zero of=/swapfile bs=1M count=2048 status=none
  chmod 600 /swapfile
  mkswap /swapfile >/dev/null
  swapon /swapfile
fi
grep -q '^/swapfile' /etc/fstab || echo '/swapfile none swap sw 0 0' >> /etc/fstab
echo 'vm.swappiness=10' > /etc/sysctl.d/99-swappiness.conf
sysctl -q -w vm.swappiness=10

# --- restart and verify -----------------------------------------------------
say "restarting postgresql"
systemctl restart postgresql

if ss -lntp 2>/dev/null | grep -q "$db_ip:5432"; then
  say "listening on $db_ip:5432"
else
  die "not listening on $db_ip:5432 — check journalctl -u postgresql"
fi

echo
say "done. On the API host, DATABASE_URL should be:"
echo "  postgres://$DB_USER:<password>@$db_ip:5432/$DB_NAME?sslmode=require"
echo
say "the Oracle security list is not on this host: it needs ingress for TCP 5432"
say "from $api_ip/32, or the connection times out rather than being refused."
