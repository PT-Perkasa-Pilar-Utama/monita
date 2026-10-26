#!/bin/sh
# Installs Monita and a location's probe as systemd services on a throwaway Linux host, signs in,
# and removes both.
# CI runs it on a GitHub runner: sudo scripts/smoke-cli.sh target/debug/monita
set -eu

install -m 0755 "$1" /usr/local/bin/monita
url=http://127.0.0.1:18090
export MONITA_BASE_URL=https://monita.example.com MONITA_BIND=127.0.0.1:18090
export MONITA_ADMIN_EMAIL=smoke@example.com MONITA_ADMIN_PASSWORD=smoke-password-1234

# Signs in as the sign-in page does; prints the HTTP status (204 when it works).
sign_in() {
  curl -s -o /dev/null -w '%{http_code}' -X POST "$url/api/ui/login" \
    -H "Origin: $MONITA_BASE_URL" -H 'Content-Type: application/json' \
    -d "{\"email\": \"$MONITA_ADMIN_EMAIL\", \"password\": \"$1\"}"
}

monita setup --yes
monita setup --yes | grep 'nothing changed' >/dev/null
test "$(stat -c %a /etc/monita/monita.env)" = 600

monita service install
test "$(stat -c %U /var/lib/monita/monita.db)" = monita
curl -fsS "$url/healthz"
test "$(sign_in "$MONITA_ADMIN_PASSWORD")" = 204

# Root resets the password offline; the service must still open the files root touched.
systemctl stop monita
printf 'a-new-smoke-password\n' | monita admin reset-password --email "$MONITA_ADMIN_EMAIL"
systemctl start monita
for _ in 1 2 3 4 5 6 7 8 9 10; do curl -fs "$url/healthz" >/dev/null && break; sleep 1; done
test "$(sign_in a-new-smoke-password)" = 204
test "$(monita whereis service)" = /etc/systemd/system/monita.service

# A location's probe as its own service: it reaches this Monita, and Admin, Locations sees it.
# The session cookie is Secure (the base URL is https), so it goes back by hand over plain HTTP.
api() {
  curl -fsS -H "Origin: $MONITA_BASE_URL" -H 'Content-Type: application/json' -H "Cookie: $cookie" "$@"
}
cookie=$(curl -s -D - -o /dev/null -X POST "$url/api/ui/login" \
  -H "Origin: $MONITA_BASE_URL" -H 'Content-Type: application/json' \
  -d "{\"email\": \"$MONITA_ADMIN_EMAIL\", \"password\": \"a-new-smoke-password\"}" |
  sed -n 's/^[Ss]et-[Cc]ookie: \(monita_session=[^;]*\).*/\1/p')
test -n "$cookie"
token=$(api -X POST "$url/api/ui/admin/locations" -d '{"name": "Smoke"}' |
  sed -n 's/.*"token":"\(ml_[^"]*\)".*/\1/p')
test -n "$token"
MONITA_PROBE_TOKEN=$token monita probe install --server "$url"
test "$(stat -c %a /etc/monita/probe.env)" = 600
seen=
for _ in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15; do
  if api "$url/api/ui/admin/locations" | grep -q '"last_seen_at":[0-9]'; then seen=1; break; fi
  sleep 1
done
if [ -z "$seen" ]; then journalctl -u monita-probe --no-pager | tail -20; exit 1; fi
monita probe uninstall
test ! -e /etc/systemd/system/monita-probe.service
test ! -e /etc/monita/probe.env

monita service uninstall
test ! -e /etc/systemd/system/monita.service
if curl -fs "$url/healthz" >/dev/null; then echo "monita still answers after uninstall" >&2; exit 1; fi
echo "smoke-cli: ok"
