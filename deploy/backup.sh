#!/bin/bash
# Dactile's plays and settings, off this machine: every hour into a private
# Git repository, once a day into a dated Google Drive folder of which the
# last five stay. Each goes only once it is set up:
#   git:   `git -C ~/data remote add origin git@github.com:...` and a deploy key
#   drive: an rclone remote named `drive`
# A restore is a `git clone` of that repository as ~/data.

set -uo pipefail
data=${DACTILE_DATA:-$HOME/data}
keep=5
cd "$data" || exit 1

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

git add -A
git diff --cached --quiet || git commit -q -m "Yedek $(date '+%Y-%m-%d %H:%M')"
if git remote get-url origin >/dev/null 2>&1; then
  git push -q origin main || echo "git push failed"
fi

if rclone listremotes 2>/dev/null | grep -qx 'drive:'; then
  today=$(date +%F)
  if ! rclone lsf --dirs-only drive:Dactile-yedek 2>/dev/null | grep -qx "$today/"; then
    # The committed files only, as they are now.
    snap=$(mktemp -d /tmp/dactile-yedek.XXXXXX)
    git archive --format=tar HEAD | tar -x -C "$snap"
    rclone copy "$snap" "drive:Dactile-yedek/$today" && echo "drive: $today"
    case $snap in /tmp/dactile-yedek.*) rm -r -- "$snap" ;; esac
    rclone lsf --dirs-only drive:Dactile-yedek | sort | head -n -"$keep" | while read -r old; do
      rclone purge "drive:Dactile-yedek/$old"
    done
  fi
fi
