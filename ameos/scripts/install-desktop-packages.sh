#!/bin/sh
# SPDX-License-Identifier: BSD-2-Clause
# Install the AmeOS desktop package manifest on a prepared FreeBSD system.
#
# Default mode is a read-only plan. Network/package mutation requires the
# explicit --apply flag and a native FreeBSD host.
set -eu

fail() {
    printf 'install-desktop-packages: %s\n' "$*" >&2
    exit 1
}

scriptdir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
root=$(CDPATH= cd -- "$scriptdir/../.." && pwd -P)
manifest="$root/ameos/packages/desktop-amd64.pkglist"
apply=no

while [ "$#" -gt 0 ]; do
    case "$1" in
    --apply) apply=yes; shift ;;
    --dry-run) apply=no; shift ;;
    --help)
        sed -n '3,/^set -eu/{ /^set -eu/d; s/^# \{0,1\}//; p; }' "$0"
        exit 0
        ;;
    *) fail "unknown argument: $1" ;;
    esac
done

[ -f "$manifest" ] || fail "manifest not found: $manifest"
packages=$(awk '!/^($|#)/ { print }' "$manifest")
[ -n "$packages" ] || fail 'manifest has no packages'

printf 'Manifest: %s\n' "$manifest"
printf '%s\n' 'Packages:'
printf '%s\n' "$packages" | sed 's/^/  /'

if [ "$apply" = no ]; then
    printf '%s\n' 'Plan only: no packages changed. Use --apply to install after reviewing.'
    # pkg dry-run still contacts the configured repository, so it is explicit
    # rather than part of the default offline plan.
    exit 0
fi

[ "$(uname -s)" = FreeBSD ] || fail 'desktop package installation requires native FreeBSD'
command -v pkg >/dev/null 2>&1 || fail 'pkg is required'

# Keep the package list as separate argv entries; do not pass it through eval.
# Package names are validated by check-desktop-packages.sh and come from the
# trusted repository manifest.
set -- $packages
# pkg itself resolves dependencies and verifies repository signatures.
pkg install --yes "$@"
printf '%s\n' 'Desktop packages installed.'
