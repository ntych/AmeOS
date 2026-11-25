#!/bin/sh
# SPDX-License-Identifier: BSD-2-Clause
# Apply the AmeOS overlay onto a staged root (DESTDIR) or a live system.
# Substitutes @TOKEN@ placeholders and installs each file with the same
# relative path it has under ameos/overlay/.
#
# Usage:
#   sh ameos/scripts/apply-overlay.sh --profile NAME --destdir /mnt
#
# Options:
#   --profile NAME   Build profile name (must exist in ameos/profiles/).
#   --destdir DIR    Root to install into (must already exist).
#   --dry-run        Print what would happen without touching any files.
#   --help           Show this message.
#
# Token substitution:
#   @AMEOS_VERSION@     from the profile .conf
#   @SOURCE_COMMIT@     current HEAD commit SHA
#   @FREEBSD_REVISION@  from sys/conf/newvers.sh
#   @FREEBSD_BRANCH@    from sys/conf/newvers.sh
set -eu

fail() { printf 'apply-overlay: %s\n' "$*" >&2; exit 1; }

usage() {
    sed -n '3,/^set -eu/{ /^set -eu/d; s/^# \{0,1\}//; p; }' "$0"
}

profile=
destdir=
dry_run=no

while [ "$#" -gt 0 ]; do
    case "$1" in
    --profile)  [ "$#" -ge 2 ] || fail "missing value for $1"; profile=$2;  shift 2 ;;
    --destdir)  [ "$#" -ge 2 ] || fail "missing value for $1"; destdir=$2;  shift 2 ;;
    --dry-run)  dry_run=yes; shift ;;
    --help)     usage; exit 0 ;;
    *)          fail "unknown argument: $1" ;;
    esac
done

[ -n "$profile" ]  || fail '--profile is required'
[ -n "$destdir" ]  || fail '--destdir is required'
[ -d "$destdir" ]  || fail "destdir does not exist: $destdir"

scriptdir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
root=$(CDPATH= cd -- "$scriptdir/../.." && pwd -P)
overlay="$root/ameos/overlay"
profile_conf="$root/ameos/profiles/$profile.conf"

[ -f "$profile_conf" ] || fail "profile not found: $profile_conf"
[ -d "$overlay" ]      || fail "overlay directory not found: $overlay"

# Source the profile to get AMEOS_VERSION and optional desktop tokens.
# shellcheck source=/dev/null
. "$profile_conf"
[ -n "${AMEOS_VERSION:-}" ] || fail "AMEOS_VERSION not set by profile"
# Optional tokens — default to empty string when not set by the profile.
pkg_abi="${PKG_ABI:-}"
pkg_repo_branch="${PKG_REPO_BRANCH:-}"

# Read FreeBSD version tokens from the newvers.sh in the source tree.
version_file="$root/sys/conf/newvers.sh"
[ -f "$version_file" ] || fail "sys/conf/newvers.sh not found in source tree"
freebsd_revision=$(sed -n 's/^REVISION="\([^"]*\)"$/\1/p' "$version_file")
freebsd_branch=$(sed -n 's/^BRANCH="\([^"]*\)"$/\1/p' "$version_file")
[ -n "$freebsd_revision" ] || fail "could not parse REVISION from newvers.sh"
[ -n "$freebsd_branch" ]   || fail "could not parse BRANCH from newvers.sh"

# Use HEAD commit as the build ID; fall back to 'unknown' outside a repo.
source_commit=$(git -C "$root" rev-parse HEAD 2>/dev/null || printf 'unknown')

printf 'apply-overlay: profile=%s  AmeOS=%s  FreeBSD=%s-%s  commit=%s\n' \
    "$profile" "$AMEOS_VERSION" "$freebsd_revision" "$freebsd_branch" \
    "$source_commit"
[ "$dry_run" = no ] || printf 'apply-overlay: dry-run mode — no files will be written\n'

# Walk every file in the overlay tree.
# 'find' output is used to drive installation; file names are from a trusted
# source tree (not user input), so word splitting on the loop variable is safe.
# README.md at the overlay root documents the directory itself — skip it.
find "$overlay" -type f ! -name 'README.md' | while IFS= read -r src; do
    rel="${src#"$overlay/"}"
    dest="$destdir/$rel"

    if [ "$dry_run" = yes ]; then
        printf '  would install: %s\n' "/$rel"
        continue
    fi

    # Create parent directories if needed.
    dest_dir=$(dirname "$dest")
    mkdir -p "$dest_dir"

    # Substitute tokens.  Use a temp file so a failure mid-write doesn't
    # leave a partially-written destination.
    tmp="${dest}.apply-overlay.tmp"
    sed \
        -e "s|@AMEOS_VERSION@|$AMEOS_VERSION|g" \
        -e "s|@SOURCE_COMMIT@|$source_commit|g" \
        -e "s|@FREEBSD_REVISION@|$freebsd_revision|g" \
        -e "s|@FREEBSD_BRANCH@|$freebsd_branch|g" \
        -e "s|@PKG_ABI@|$pkg_abi|g" \
        -e "s|@PKG_REPO_BRANCH@|$pkg_repo_branch|g" \
        "$src" > "$tmp"
    mv "$tmp" "$dest"

    # Make certain files executable (binaries in sbin/bin, rc.d scripts).
    case "$rel" in
    usr/local/sbin/*|usr/local/bin/*|usr/sbin/*|usr/bin/*|etc/rc.d/*)
        chmod 755 "$dest"
        ;;
    esac
    printf '  installed: %s\n' "/$rel"
done

printf 'apply-overlay: done\n'
