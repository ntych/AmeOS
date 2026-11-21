# AmeOS package manifests

Package manifests are explicit install plans for profiles that use the
official FreeBSD pkg repository.

## `desktop-amd64.pkglist`

The desktop manifest contains Wayfire, Wayland, Mesa/XWayland, Foot and font
packages. It is deliberately **not** consumed by `build-release.sh` yet:
release builds remain offline and reproducible, while package resolution is
network-dependent and must be performed as a separate installation step.

Validate the manifest and profile metadata without contacting a repository:

```sh
sh ameos/scripts/check-desktop-packages.sh
```

On a prepared FreeBSD desktop installation, review the list before installing:

```sh
pkg install --dry-run $(awk '!/^($|#)/ { print }' \
  ameos/packages/desktop-amd64.pkglist)
pkg install $(awk '!/^($|#)/ { print }' \
  ameos/packages/desktop-amd64.pkglist)
```

The commands above are intentionally not executed by the build scripts.
For a read-only plan using the checked-in manifest:

```sh
sh ameos/scripts/install-desktop-packages.sh --dry-run
```

To actually install on a prepared native FreeBSD system, after reviewing the
plan and repository configuration:

```sh
sh ameos/scripts/install-desktop-packages.sh --apply
```

`--apply` is the only mode that calls `pkg install --yes`; it is rejected on
non-FreeBSD hosts. Console profiles do not use this manifest and remain
package-free.
