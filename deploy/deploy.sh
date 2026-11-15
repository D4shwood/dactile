#!/bin/bash
# Dactile put up on the Oracle machine as the `dactile` user, with no sudo:
#
#   bash deploy/deploy.sh
#
# The interface is built here; the server is built there, as the machine is
# ARM; then the service restarts. What is put up is the committed tree: with
# changes not committed yet it stops. The server put up before stays beside
# the new one as ~/bin/dactile-server.before, to go back to.
# Reads deploy/site.env (IP, DOMAIN); the password to sign in with stays in
# ~/.config/dactile-web/password on this computer.

set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
repo=$(dirname "$here")
. "$here/site.env"
to="dactile@$IP"
ssh=(ssh -i "$HOME/.ssh/dactile_oracle" -o StrictHostKeyChecking=accept-new)
run() { "${ssh[@]}" "$to" "$@"; }
put() { rsync -a --delete -e "${ssh[*]}" "$@"; }

if ! git -C "$repo" diff --quiet HEAD; then
  echo "There are changes not committed: commit them first, or what is put up is not what git has."
  exit 1
fi

echo "== interface"
(cd "$repo/app" && npx vite build -c vite.web.config.ts --logLevel warn)

echo "== source"
src=$(mktemp -d /tmp/dactile-deploy.XXXXXX)
git -C "$repo" archive HEAD Cargo.lock core backend server tex examples | tar -x -C "$src"
# A workspace of the server and what it is made of: the desktop app and
# Dactile Light aren't built there.
{
  printf '[workspace]\nresolver = "3"\nmembers = ["core", "backend", "server"]\n\n'
  git -C "$repo" show HEAD:Cargo.toml | sed -n '/^\[workspace.package\]/,$p'
} > "$src/Cargo.toml"
put "$src/" "$to:src/"
case $src in /tmp/dactile-deploy.*) rm -r -- "$src" ;; esac

echo "== build (the first one fetches Rust and takes a while)"
run 'bash -s' <<'EOF'
set -euo pipefail
[ -x ~/.cargo/bin/cargo ] || curl -sSf https://sh.rustup.rs | sh -s -- -y -q --profile minimal
cd ~/src && ~/.cargo/bin/cargo build -q --release -p dactile-server
mkdir -p ~/bin && cp target/release/dactile-server ~/bin/.dactile-server.new
[ ! -f ~/bin/dactile-server ] || cp ~/bin/dactile-server ~/bin/dactile-server.before
mv ~/bin/.dactile-server.new ~/bin/dactile-server
EOF

echo "== interface, dictionary, services"
put "$repo/app/dist-web/" "$to:web/"
run 'mkdir -p ~/data/share/dactile/dict ~/bin ~/.config/systemd/user'
for f in /usr/share/hunspell/tr_TR.aff /usr/share/hunspell/tr_TR.dic; do
  [ -f "$f" ] && put "$f" "$to:data/share/dactile/dict/"
done
put "$here/backup.sh" "$to:bin/backup.sh"
put "$here/dactile.service" "$here/backup.service" "$here/backup.timer" "$to:.config/systemd/user/"

# The password, made here once and kept here; sent through stdin, never on a
# command line.
pass=$HOME/.config/dactile-web/password
if [ ! -s "$pass" ]; then
  mkdir -p "$(dirname "$pass")"
  (umask 077 && head -c 18 /dev/urandom | base64 | tr -d '/+=' > "$pass")
fi
# XeLaTeX boxed in, where the machine lets its users have the box.
box=
if run 'bwrap --unshare-all --unshare-user --ro-bind /usr /usr --ro-bind-try /lib /lib --ro-bind-try /lib64 /lib64 --ro-bind-try /bin /bin /usr/bin/true' 2>/dev/null; then
  box='DACTILE_SANDBOX=bwrap\n'
else
  echo "!! bwrap doesn't start for the dactile user: XeLaTeX will run UNBOXED. Run setup.sh again, or see docs/web.md."
fi
printf "$box"'DACTILE_DATA=/home/dactile/data\nDACTILE_WEB=/home/dactile/web\nDACTILE_ADDR=127.0.0.1:1430\nDACTILE_SECURE=1\nDACTILE_PASSWORD=%s\n' "$(cat "$pass")" |
  run 'umask 077 && mkdir -p ~/.config/dactile-server && cat > ~/.config/dactile-server/env'

run 'systemctl --user daemon-reload && systemctl --user enable --now -q dactile.service backup.timer && systemctl --user restart dactile.service'
sleep 2
echo "== check"
run 'systemctl --user is-active dactile.service'
code=$(curl -s -o /dev/null -w '%{http_code}' "https://$DOMAIN/api/version" || true)
echo "https://$DOMAIN/api/version: $code (401 is right: signed out)"
