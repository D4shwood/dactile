#!/bin/bash
# Accounts on the machine Dactile is put up on. The owner does the same
# from the site's settings (People); these are for the first one, and for
# when the site can't be reached. The first word says which site: `live`,
# or `pbe` for the trial one, whose accounts are its own.
#
#   bash deploy/people.sh live add ada "Ada Yıldız" --owner --mail ada@example.com
#   bash deploy/people.sh live reset ada         a link to set ada's password, good once, 7 days
#   bash deploy/people.sh live invite-mail baran@example.com "Baran Deniz"
#                                           a link to make an account
#   bash deploy/people.sh live people
#   bash deploy/people.sh live remove ada        off the list; their plays stay
#
# A password link lets whoever opens it in as that person: send it to them
# alone.

set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
. "$here/site.env"
case ${1:-} in
  live) home=/home/dactile; domain=$DOMAIN ;;
  pbe) home=/home/dactile/pbe; domain=$PBE_DOMAIN ;;
  *) echo "Which site: bash deploy/people.sh live|pbe <command>"; exit 1 ;;
esac
shift
args=$(printf ' %q' "$@")
ssh -i "$HOME/.ssh/dactile_oracle" -o StrictHostKeyChecking=accept-new "dactile@$IP" \
  "DACTILE_HOME=$home/data DACTILE_SITE=https://$domain $home/bin/dactile-gate$args"
