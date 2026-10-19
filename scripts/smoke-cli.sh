#!/bin/sh
# Installs Monita as a systemd service on a throwaway Linux host, signs in, and removes it.
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

monita service uninstall
test ! -e /etc/systemd/system/monita.service
if curl -fs "$url/healthz" >/dev/null; then echo "monita still answers after uninstall" >&2; exit 1; fi
echo "smoke-cli: ok"
