#!/bin/bash
# Accounts on the machine Dactile is put up on. The owner does the same
# from the site's settings (People); these are for the first one, and for
# when the site can't be reached:
#
#   bash deploy/people.sh add ada "Ada Yıldız" --owner --mail ada@example.com
#   bash deploy/people.sh reset ada         a link to set ada's password, good once, 7 days
#   bash deploy/people.sh invite-mail baran@example.com "Baran Deniz"
#                                           a link to make an account
#   bash deploy/people.sh people
#   bash deploy/people.sh remove ada        off the list; their plays stay
#
# A password link lets whoever opens it in as that person: send it to them
# alone.

set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
. "$here/site.env"
args=$(printf ' %q' "$@")
ssh -i "$HOME/.ssh/dactile_oracle" -o StrictHostKeyChecking=accept-new "dactile@$IP" \
  "DACTILE_HOME=/home/dactile/data DACTILE_SITE=https://$DOMAIN /home/dactile/bin/dactile-gate$args"
