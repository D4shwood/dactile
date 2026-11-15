#!/bin/bash
# Dactile's plays and settings, off this machine: every hour into a private
# Git repository, once a day into a dated Google Drive folder of which the
# last five stay. Each goes only once it is set up:
#   git:   `git -C ~/data remote add origin git@github.com:...` and a deploy key
#   drive: an rclone remote named `drive`
# A restore is a `git clone` of that repository as ~/data.
#
# How it went is left in cache/backup for the server, which says so on the
# start screen when it went badly, is late, or never leaves this machine.

set -uo pipefail
data=${DACTILE_DATA:-$HOME/data}
keep=5
cd "$data" || exit 1
error=
away=0
said() {
  mkdir -p cache
  {
    [ -n "$error" ] || echo "ok=$(date +%s)"
    [ -z "$error" ] || grep -s '^ok=' cache/backup
    echo "away=$away"
    echo "error=$error"
  } > cache/backup.new && mv cache/backup.new cache/backup
}
trap said EXIT

if [ ! -d .git ]; then
  git init -q -b main
  git config user.name "Dactile"
  git config user.email "dactile@localhost"
fi
# Built again from the plays at will, or not the plays at all. The plays'
# earlier versions are left out too: this repository keeps them itself.
cat > .gitignore <<'EOF'
cache/
home/
uploads/
share/Trash/
share/dactile/history/
share/dactile/dict/
gate/secret/
*.aux
*.log
*.out
*.toc
*.pdf
*.synctex.gz
*.xdv
*.lock
*-lock
EOF

git add -A || error="git add"
git diff --cached --quiet || git commit -q -m "Yedek $(date '+%Y-%m-%d %H:%M')" || error="git commit"
if git remote get-url origin >/dev/null 2>&1; then
  if git push -q origin main; then away=1; else error="git push"; fi
fi

if rclone listremotes 2>/dev/null | grep -qx 'drive:'; then
  today=$(date +%F)
  if ! rclone lsf --dirs-only drive:Dactile-yedek 2>/dev/null | grep -qx "$today/"; then
    # The committed files only, as they are now.
    snap=$(mktemp -d /tmp/dactile-yedek.XXXXXX)
    git archive --format=tar HEAD | tar -x -C "$snap"
    if rclone copy "$snap" "drive:Dactile-yedek/$today"; then
      echo "drive: $today"
    else
      error="${error:+$error, }Drive"
    fi
    case $snap in /tmp/dactile-yedek.*) rm -r -- "$snap" ;; esac
    rclone lsf --dirs-only drive:Dactile-yedek | sort | head -n -"$keep" | while read -r old; do
      rclone purge "drive:Dactile-yedek/$old"
    done
  fi
  # Today's copy is there, made now or earlier today.
  [[ $error == *Drive* ]] || away=1
fi
