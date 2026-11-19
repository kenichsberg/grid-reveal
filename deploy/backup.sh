#!/usr/bin/env bash
# Nightly database backup to object storage.
#
# Scheduled by grid-reveal-backup.timer; see deploy/ for both units. Run it by
# hand with `systemctl start grid-reveal-backup`, or directly for a quick check.
#
# A backup you have never restored is a guess. Restore one into a scratch
# database before trusting this.
set -euo pipefail

# Normally systemd supplies the environment from this same file, via
# EnvironmentFile in grid-reveal-backup.service. Sourcing is the fallback for a
# run by hand, and deliberately not the usual path: the shell interprets $,
# backticks and quotes in a value where systemd takes it literally, so the two
# readings of one file can disagree. Keep these values plain either way.
ENV_FILE="${ENV_FILE:-/opt/grid-reveal/.env}"
if [ -z "${DATABASE_URL:-}" ] && [ -f "$ENV_FILE" ]; then
  set -a
  # shellcheck disable=SC1090
  . "$ENV_FILE"
  set +a
fi

: "${DATABASE_URL:?DATABASE_URL must be set}"
: "${R2_BUCKET:?R2_BUCKET must be set}"

# The AWS CLI reads credentials from the environment, so the R2 keys already in
# .env are enough. `aws configure` would write a second copy under /root/.aws:
# another place for them to live, drift, and be forgotten on a rebuild.
export AWS_ACCESS_KEY_ID="${R2_ACCESS_KEY_ID:?R2_ACCESS_KEY_ID must be set}"
export AWS_SECRET_ACCESS_KEY="${R2_SECRET_ACCESS_KEY:?R2_SECRET_ACCESS_KEY must be set}"
export AWS_DEFAULT_REGION="${AWS_DEFAULT_REGION:-auto}"

stamp="$(date -u +%Y%m%dT%H%M%SZ)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

dump="$tmp/grid-reveal-$stamp.sql.gz"

pg_dump --no-owner --no-privileges "$DATABASE_URL" | gzip -9 > "$dump"

size=$(wc -c < "$dump")
if [ "$size" -lt 1024 ]; then
  echo "refusing to upload a suspiciously small dump ($size bytes)" >&2
  exit 1
fi

# Requires the aws CLI configured against the R2 endpoint, or rclone.
aws s3 cp "$dump" "s3://$R2_BUCKET/backups/$(basename "$dump")" \
  --endpoint-url "${R2_ENDPOINT:?R2_ENDPOINT must be set}"

echo "uploaded $(basename "$dump") ($size bytes)"

# Keep 30 days; R2 lifecycle rules can do this instead if you prefer.
cutoff="$(date -u -d '30 days ago' +%Y%m%d 2>/dev/null || date -u -v-30d +%Y%m%d)"
aws s3 ls "s3://$R2_BUCKET/backups/" --endpoint-url "$R2_ENDPOINT" \
  | awk '{print $4}' \
  | while read -r name; do
      [ -z "$name" ] && continue
      file_date="$(echo "$name" | sed -n 's/.*-\([0-9]\{8\}\)T.*/\1/p')"
      [ -z "$file_date" ] && continue
      if [ "$file_date" -lt "$cutoff" ]; then
        aws s3 rm "s3://$R2_BUCKET/backups/$name" --endpoint-url "$R2_ENDPOINT"
      fi
    done
