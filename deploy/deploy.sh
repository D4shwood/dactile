#!/bin/bash
# Dactile put up on the Oracle machine as the `dactile` user, with no sudo:
#
#   bash deploy/deploy.sh pbe     the trial site (PBE_DOMAIN), first
#   bash deploy/deploy.sh live    the real one (DOMAIN), with what the
#                                 trial site already runs, and nothing else
#
# The two are apart: each its own folder, accounts, plays and service. The
# trial site lives in ~/pbe and listens on 1431; it has no backups, and its
# page says it is a trial.
#
# The interface is built here; the server is built there, as the machine is
# ARM; then the service restarts. What is put up is the committed tree: with
# changes not committed yet it stops. The server put up before stays beside
# the new one as bin/dactile-server.before (and dactile-gate.before), to
# go back to.
# Reads deploy/site.env (IP, DOMAIN; GOOGLE_ID and GOOGLE_SECRET if people
# are to sign in with Google; MAIL_URL, MAIL_USER, MAIL_PASS, MAIL_FROM for
# the mail service invitations and new passwords go by). People have
# accounts: the owner invites them from the site's settings, or with
# deploy/people.sh.

set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
repo=$(dirname "$here")
. "$here/site.env"
case ${1:-} in
  live) home=/home/dactile; port=1430; domain=$DOMAIN; unit=dactile.service; conf=dactile-server; stage= ;;
  pbe)
    [ -n "${PBE_DOMAIN:-}" ] || { echo "site.env has no PBE_DOMAIN."; exit 1; }
    home=/home/dactile/pbe; port=1431; domain=$PBE_DOMAIN; unit=dactile-pbe.service; conf=dactile-pbe; stage=pbe ;;
  *) echo "Which site: bash deploy/deploy.sh pbe | live"; exit 1 ;;
esac
to="dactile@$IP"
ssh=(ssh -i "$HOME/.ssh/dactile_oracle" -o StrictHostKeyChecking=accept-new)
run() { "${ssh[@]}" "$to" "$@"; }
put() { rsync -a --delete -e "${ssh[*]}" "$@"; }

if ! git -C "$repo" diff --quiet HEAD; then
  echo "There are changes not committed: commit them first, or what is put up is not what git has."
  exit 1
fi
commit=$(git -C "$repo" rev-parse HEAD)
if [ "$1" = live ]; then
  tried=$(run 'cat ~/pbe/commit 2>/dev/null' || true)
  if [ "$tried" != "$commit" ]; then
    echo "The trial site doesn't run this commit: put it up there first (bash deploy/deploy.sh pbe), try it, then here."
    exit 1
  fi
fi

echo "== interface"
(cd "$repo/app" && npm run --silent build:web -- --logLevel warn)

echo "== source"
src=$(mktemp -d /tmp/dactile-deploy.XXXXXX)
git -C "$repo" archive HEAD Cargo.lock core backend server tex examples | tar -x -C "$src"
# A workspace of the server and what it is made of: the desktop app and
# Dactile Light aren't built there.
{
  printf '[workspace]\nresolver = "3"\nmembers = ["core", "backend", "server"]\n\n'
  git -C "$repo" show HEAD:Cargo.toml | sed -n '/^\[workspace.package\]/,$p'
} > "$src/Cargo.toml"
# What was built there before stays, or every deploy builds it all again.
put --exclude /target "$src/" "$to:src/"
case $src in /tmp/dactile-deploy.*) rm -r -- "$src" ;; esac

echo "== build (the first one fetches Rust and takes a while)"
run "bin=$home/bin bash -s" <<'EOF'
set -euo pipefail
[ -x ~/.cargo/bin/cargo ] || curl -sSf https://sh.rustup.rs | sh -s -- -y -q --profile minimal
cd ~/src && ~/.cargo/bin/cargo build -q --release -p dactile-server
mkdir -p "$bin"
for program in dactile-server dactile-gate; do
  cp target/release/$program "$bin/.$program.new"
  [ ! -f "$bin/$program" ] || cp "$bin/$program" "$bin/$program.before"
  mv "$bin/.$program.new" "$bin/$program"
done
EOF

echo "== interface, dictionary, services"
run "mkdir -p ~/dict ~/bin ~/.config/systemd/user $home/web"
put "$repo/app/dist-web/" "$to:$home/web/"
for f in /usr/share/hunspell/tr_TR.aff /usr/share/hunspell/tr_TR.dic; do
  [ -f "$f" ] && put "$f" "$to:dict/"
done
put "$here/backup.sh" "$to:bin/backup.sh"
put "$here/dactile.service" "$here/dactile-pbe.service" "$here/backup.service" "$here/backup.timer" "$to:.config/systemd/user/"

# Each person may keep two gigabytes; past that they can't upload more.
# XeLaTeX boxed in, where the machine lets its users have the box.
box=
if run 'bwrap --unshare-all --unshare-user --ro-bind /usr /usr --ro-bind-try /lib /lib --ro-bind-try /lib64 /lib64 --ro-bind-try /bin /bin /usr/bin/true' 2>/dev/null; then
  box='DACTILE_SANDBOX=bwrap\n'
else
  echo "!! bwrap doesn't start for the dactile user: XeLaTeX will run UNBOXED. Run setup.sh again, or see docs/web.md."
fi
# Signing in with Google, once site.env has the OAuth client's id and secret.
google=
if [ -n "${GOOGLE_ID:-}" ] && [ -n "${GOOGLE_SECRET:-}" ]; then
  google="DACTILE_GOOGLE_ID=$GOOGLE_ID\nDACTILE_GOOGLE_SECRET=$GOOGLE_SECRET\n"
fi
# Mail for invitations and new passwords, once site.env has the service's
# SMTP: MAIL_URL (smtps://host:465), MAIL_USER, MAIL_PASS, MAIL_FROM.
mail=
if [ -n "${MAIL_URL:-}" ]; then
  mail="DACTILE_MAIL_URL=$MAIL_URL\nDACTILE_MAIL_USER=${MAIL_USER:-}\nDACTILE_MAIL_PASS=${MAIL_PASS:-}\nDACTILE_MAIL_FROM=${MAIL_FROM:-Dactile <dactile@$domain>}\n"
fi
trial=
[ -z "$stage" ] || trial="DACTILE_STAGE=$stage\n"
printf '%b' "$box$google$mail$trial"'DACTILE_SITE=https://'"$domain"'\nDACTILE_HOME='"$home"'/data\nDACTILE_WEB='"$home"'/web\nDACTILE_ADDR=127.0.0.1:'"$port"'\nDACTILE_SECURE=1\nDACTILE_DICT=/home/dactile/dict\nDACTILE_BACKUP='"$home"'/data/cache/backup\nDACTILE_QUOTA_MB=2048\n' |
  run "umask 077 && mkdir -p ~/.config/$conf && cat > ~/.config/$conf/env"

# Backups are the real site's: the trial one's plays are for trying.
timer=
[ "$1" = pbe ] || timer=backup.timer
run "systemctl --user daemon-reload && systemctl --user enable --now -q $unit $timer && systemctl --user restart $unit"
sleep 2
echo "== check"
run "systemctl --user is-active $unit"
[ "$1" = live ] || run "echo $commit > ~/pbe/commit"
code=$(curl -s -o /dev/null -w '%{http_code}' "https://$domain/api/version" || true)
echo "https://$domain/api/version: $code (401 is right: signed out)"
echo "People and their links: bash deploy/people.sh $1 people | add <id> <name> [--owner] | reset <id> | remove <id>"
