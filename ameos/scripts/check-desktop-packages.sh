#!/bin/sh
# SPDX-License-Identifier: BSD-2-Clause
# Validate an AmeOS pkg manifest without contacting a repository.
set -eu

fail() {
    printf 'check-desktop-packages: %s\n' "$*" >&2
    exit 1
}

scriptdir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
root=$(CDPATH= cd -- "$scriptdir/../.." && pwd -P)
profile=desktop-amd64
manifest="$root/ameos/packages/$profile.pkglist"
profile_conf="$root/ameos/profiles/$profile.conf"

[ -f "$profile_conf" ] || fail "profile not found: $profile_conf"
[ -f "$manifest" ] || fail "manifest not found: $manifest"

# shellcheck source=/dev/null
. "$profile_conf"
[ "${WITH_PACKAGES:-no}" = yes ] || fail "$profile is not a package profile"
[ -n "${PKG_ABI:-}" ] || fail "$profile has no PKG_ABI"
[ -n "${PKG_REPO_BRANCH:-}" ] || fail "$profile has no PKG_REPO_BRANCH"

count=0
while IFS= read -r line || [ -n "$line" ]; do
    case "$line" in
    ''|[[:space:]]*|'#'*) continue ;;
    esac
    case "$line" in
    *[!A-Za-z0-9+_.-]*) fail "invalid package name: $line" ;;
    esac
    count=$((count + 1))
done < "$manifest"

[ "$count" -gt 0 ] || fail 'manifest is empty'
printf 'check-desktop-packages: %s contains %s packages for %s\n' \
    "$manifest" "$count" "$PKG_ABI"
