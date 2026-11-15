#!/bin/bash
# Dactile's Oracle machine, made once, as root: everything that needs sudo is
# here, so the owner runs it one time and never again. After it, Dactile is
# put up and kept running by the `dactile` user, who has no sudo.
#
#   sudo bash setup.sh <domain> <dactile.pub>
#
# Ubuntu 24.04 or later on an Ampere (ARM) machine. Safe to run again.

set -euo pipefail

domain=${1:?the domain of the site, e.g. ornek.duckdns.org}
key=${2:?the public key the dactile user is reached with}
[ "$(id -u)" = 0 ] || { echo "run it with sudo"; exit 1; }
grep -q '^ssh-' "$key" || { echo "$key is not a public key"; exit 1; }

export DEBIAN_FRONTEND=noninteractive
apt-get update -q

# The web server, the tools to build Dactile and back it up, and what its
# PDFs, imports and OCR call on.
apt-get install -y -q --no-install-recommends \
  caddy git rsync rclone curl ca-certificates build-essential pkg-config \
  texlive-xetex texlive-latex-recommended texlive-latex-extra \
  texlive-lang-european texlive-lang-cyrillic \
  poppler-utils tesseract-ocr tesseract-ocr-tur bubblewrap \
  fontconfig fonts-texgyre fonts-liberation fonts-dejavu-core fonts-dejavu-extra

# Fonts the page settings offer; one missing from this Ubuntu is no reason to
# stop.
for f in fonts-ebgaramond fonts-libertinus fonts-noto-core fonts-noto-mono fonts-jetbrains-mono; do
  apt-get install -y -q --no-install-recommends "$f" || echo "($f is not in this Ubuntu: skipped)"
done
fc-cache -f >/dev/null

# The user Dactile runs as: no sudo, reached only with the given key. Its
# folder is its own; lingering keeps its service up with no one signed in.
id dactile >/dev/null 2>&1 || useradd --create-home --shell /bin/bash dactile
chmod 700 /home/dactile
install -d -m 700 -o dactile -g dactile /home/dactile/.ssh
install -m 600 -o dactile -g dactile "$key" /home/dactile/.ssh/authorized_keys
loginctl enable-linger dactile

# Oracle's Ubuntu turns away everything but SSH in its own iptables rules,
# besides the Security List in the console: 80 and 443 open before the
# REJECT, and kept over restarts.
for port in 80 443; do
  if ! iptables -C INPUT -p tcp --dport "$port" -m state --state NEW -j ACCEPT 2>/dev/null; then
    at=$(iptables -L INPUT --line-numbers | awk '/REJECT/ { print $1; exit }')
    iptables -I INPUT "${at:-1}" -p tcp --dport "$port" -m state --state NEW -j ACCEPT
  fi
done
if command -v netfilter-persistent >/dev/null; then
  netfilter-persistent save
else
  echo "netfilter-persistent is missing: the ports close again on a restart"
fi

# XeLaTeX runs boxed in with bwrap, which needs a user namespace. Ubuntu
# 24.04 and later keep those from programs AppArmor has no profile for.
if [ "$(sysctl -n kernel.apparmor_restrict_unprivileged_userns 2>/dev/null)" = 1 ] &&
  [ ! -e /etc/apparmor.d/bwrap ] && [ ! -e /etc/apparmor.d/bwrap-userns-restrict ]; then
  cat > /etc/apparmor.d/bwrap <<'PROFILE'
abi <abi/4.0>,
include <tunables/global>

profile bwrap /usr/bin/bwrap flags=(unconfined) {
  userns,
  include if exists <local/bwrap>
}
PROFILE
  apparmor_parser -r /etc/apparmor.d/bwrap || echo "the bwrap profile didn't load: XeLaTeX will run unboxed"
fi

# Caddy: HTTPS with its own Let's Encrypt certificate, renewed by itself, in
# front of Dactile, which listens on this machine alone.
cat > /etc/caddy/Caddyfile <<EOF
$domain {
	encode zstd gzip
	request_body {
		max_size 64MB
	}
	header {
		Strict-Transport-Security "max-age=31536000"
		X-Content-Type-Options nosniff
		X-Frame-Options SAMEORIGIN
		Referrer-Policy no-referrer
		-Server
	}
	reverse_proxy 127.0.0.1:1430
}
EOF
systemctl enable caddy >/dev/null
systemctl reload caddy || systemctl restart caddy

echo
echo "Done. Dactile is put up from now on as the dactile user."
