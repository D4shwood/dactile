#!/bin/bash
# The owner's one command, from their own computer, the one time:
#
#   bash deploy/first.sh
#
# Sends setup.sh to the new machine and runs it there with sudo, signed in
# with the owner's own Oracle key. Reads deploy/site.env (not in git):
#   IP=...          the machine's reserved public IP
#   DOMAIN=...      e.g. ornek.duckdns.org
#   OWNER_KEY=...   the private key Oracle was given for the machine
#   OWNER=ubuntu    the image's own user

set -euo pipefail
here=$(dirname "$0")
. "$here/site.env"
key=$HOME/.ssh/dactile_oracle
[ -f "$key.pub" ] || ssh-keygen -q -t ed25519 -N "" -C dactile-deploy -f "$key"

to="${OWNER:-ubuntu}@$IP"
opts=(-i "$OWNER_KEY" -o StrictHostKeyChecking=accept-new)
scp "${opts[@]}" "$here/setup.sh" "$key.pub" "$to:/tmp/"
ssh -t "${opts[@]}" "$to" "sudo bash /tmp/setup.sh '$DOMAIN' /tmp/dactile_oracle.pub && rm /tmp/setup.sh /tmp/dactile_oracle.pub"
