# AmeOS overlay

Files here are installed on top of the FreeBSD base system by
`ameos/scripts/apply-overlay.sh`, which substitutes `@TOKEN@` placeholders
before writing each file.

## Available tokens

| Token | Value |
|---|---|
| `@AMEOS_VERSION@` | from the build profile (e.g. `0.2-b`) |
| `@SOURCE_COMMIT@` | HEAD commit SHA |
| `@FREEBSD_REVISION@` | from `sys/conf/newvers.sh` (e.g. `16.0`) |
| `@FREEBSD_BRANCH@` | from `sys/conf/newvers.sh` (e.g. `CURRENT`) |

## Current files

| Overlay path | Purpose |
|---|---|
| `etc/os-release` | Standard os-release identification (read by `lsb_release`, `hostnamectl`, etc.) |
| `etc/motd.template` | Login message rendered at boot |
| `etc/rc.d/ame-loader` | rc.d service script for interactive boot menu |
| `usr/local/sbin/ame-loader` | AmeOS interactive bootloader (archinstall-style CLI) |

## Applying the overlay

```sh
# Dry-run — see what would be installed
sh ameos/scripts/apply-overlay.sh \
    --profile v0.2-b-amd64 \
    --destdir /path/to/staged/root \
    --dry-run

# Real install
sh ameos/scripts/apply-overlay.sh \
    --profile v0.2-b-amd64 \
    --destdir /path/to/staged/root
```

The overlay is applied after `installworld` and before image creation so
overlay files take precedence over base system defaults. `apply-overlay.sh`
is safe to run repeatedly; each run atomically replaces the destination file
(write to temp, then `mv`).

## Rules for new files

- Use `@TOKEN@` for any value that varies by build, profile, or commit.
- Keep file paths identical to their intended location under `/`.
- Avoid overriding security-sensitive files (`/etc/master.passwd`, etc.).
- User-editable configuration files must be installed as defaults only —
  upgrading the overlay must not overwrite user changes.
