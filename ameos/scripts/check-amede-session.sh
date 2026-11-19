#!/bin/sh
# SPDX-License-Identifier: BSD-2-Clause
# Validate the checked-in AmeDE Wayfire session files.
set -eu

fail() {
    printf 'check-amede-session: %s\n' "$*" >&2
    exit 1
}

scriptdir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
root=$(CDPATH= cd -- "$scriptdir/../.." && pwd -P)
config="$root/ameos/overlay/etc/xdg/wayfire/wayfire-amede.ini"
launcher="$root/ameos/overlay/usr/local/bin/amede-session"
session="$root/ameos/overlay/usr/local/share/wayland-sessions/amede.desktop"

[ -f "$config" ] || fail "missing Wayfire config: $config"
[ -f "$launcher" ] || fail "missing session launcher: $launcher"
[ -x "$launcher" ] || fail "session launcher is not executable: $launcher"
[ -f "$session" ] || fail "missing XDG session file: $session"

# Keep this validator deliberately small and format-aware: it checks the
# settings that make the session usable without pretending to parse all of
# Wayfire's INI dialect.
grep -q '^\[core\]$' "$config" || fail 'Wayfire config has no [core] section'
grep -q '^plugins = .*autostart' "$config" || fail 'autostart plugin is not enabled'
grep -q '^xwayland = true$' "$config" || fail 'XWayland is not enabled'
! grep -q '^outputs = ' "$config" || fail 'Wayfire config hard-codes an output name'
grep -q '^\[autostart\]$' "$config" || fail 'Wayfire config has no [autostart] section'
grep -q '^amede = amede$' "$config" || fail 'AmeDE autostart must preserve the saved theme'
grep -q '^Exec=amede-session$' "$session" || fail 'XDG session does not launch amede-session'
grep -q '^Type=Application$' "$session" || fail 'XDG session has no Application type'
grep -q '^exec wayfire -c ' "$launcher" || fail 'launcher does not exec Wayfire'
grep -q 'command -v wayfire' "$launcher" || fail 'launcher does not verify Wayfire availability'

printf 'check-amede-session: valid Wayfire config and XDG session\n'
