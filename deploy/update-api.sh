#!/usr/bin/env bash
# Installs a release of the API and makes sure it came up.
#
# Usage:
#   sudo /opt/grid-reveal/update-api.sh            # the latest release
#   sudo /opt/grid-reveal/update-api.sh v0.1.1     # a specific tag
#
# The binary is replaced, the service restarted, and /health polled. If the new
# binary does not answer, the previous one is put back and the service restarted
# again, so a bad release costs a few seconds rather than an outage you have to
# notice first. Exit status is 0 only when the running service answered.
set -euo pipefail

REPO="${REPO:-kenichsberg/grid-reveal}"
INSTALL_DIR="${INSTALL_DIR:-/opt/grid-reveal}"
SERVICE="${SERVICE:-grid-reveal-api}"
HEALTH_URL="${HEALTH_URL:-http://127.0.0.1:8080/health}"
OWNER="${OWNER:-gridreveal:gridreveal}"

die() { echo "update-api: $*" >&2; exit 1; }

[ "$(id -u)" -eq 0 ] || die "run with sudo"

version="${1:-}"
if [ -z "$version" ]; then
  # The redirect from /releases/latest names the tag without needing a token.
  version="$(curl -fsSLI -o /dev/null -w '%{url_effective}' \
    "https://github.com/$REPO/releases/latest" | sed 's#.*/tag/##')"
  [ -n "$version" ] || die "could not determine the latest release"
fi
echo "update-api: installing $version"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cd "$work"

base="https://github.com/$REPO/releases/download/$version"
curl -fLsS -o grid-reveal-api "$base/grid-reveal-api" \
  || die "no binary for $version"
curl -fLsS -o grid-reveal-api.sha256 "$base/grid-reveal-api.sha256" \
  || die "no checksum for $version"

# A truncated download produces a service that fails to start for reasons that
# look like anything but a truncated download.
sha256sum -c grid-reveal-api.sha256 >/dev/null \
  || die "checksum mismatch; nothing was installed"

current="$INSTALL_DIR/grid-reveal-api"
previous="$INSTALL_DIR/grid-reveal-api.prev"
[ -f "$current" ] && cp -p "$current" "$previous"

install -m 755 grid-reveal-api "$current"
chown "$OWNER" "$current"
systemctl restart "$SERVICE"

# Startup runs migrations, so allow more than a moment before judging it.
for _ in $(seq 1 30); do
  if curl -fsS -m 2 -o /dev/null "$HEALTH_URL"; then
    echo "update-api: $version is live"
    exit 0
  fi
  sleep 1
done

echo "update-api: $version did not answer $HEALTH_URL" >&2
if [ -f "$previous" ]; then
  echo "update-api: rolling back" >&2
  install -m 755 "$previous" "$current"
  chown "$OWNER" "$current"
  systemctl restart "$SERVICE"
  echo "update-api: rolled back; check journalctl -u $SERVICE" >&2
fi
exit 1
