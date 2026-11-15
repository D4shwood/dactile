#!/bin/bash
# Who may come in, and their links, on the machine Dactile is put up on:
#
#   bash deploy/people.sh add ada "Ada Yıldız" --owner
#   bash deploy/people.sh invite ada        a link good once, for 7 days
#   bash deploy/people.sh people
#   bash deploy/people.sh remove ada        off the list; their plays stay
#
# A link lets whoever opens it in as that person: send it to them alone.

set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
. "$here/site.env"
args=$(printf ' %q' "$@")
ssh -i "$HOME/.ssh/dactile_oracle" -o StrictHostKeyChecking=accept-new "dactile@$IP" \
  "DACTILE_HOME=/home/dactile/data DACTILE_SITE=https://$DOMAIN /home/dactile/bin/dactile-gate$args"
