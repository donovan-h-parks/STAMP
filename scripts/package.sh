#!/usr/bin/env bash
# Package the release builds of STAMP for one platform.
#
#   scripts/package.sh linux   <target-dir> <label>   -> dist/stamp-<version>-<label>.tar.gz
#   scripts/package.sh macos   <target-dir> <label>   -> dist/stamp-<version>-<label>.zip  (STAMP.app + stamp-cli)
#   scripts/package.sh windows <target-dir> <label>   -> dist/stamp-<version>-<label>.zip
#
# <target-dir> is the directory holding the built binaries, for example
# target/release or target/aarch64-apple-darwin/release. Build first with
#   cargo build --release -p stamp-gui -p stamp-cli
# (stamp-gui builds both `stamp` and `stamp-mcp`).
set -euo pipefail

platform="${1:?platform: linux, macos or windows}"
bin_dir="${2:?directory containing the release binaries}"
label="${3:?label for the archive name, for example linux-x86_64}"

case "$platform" in
  linux | macos | windows) ;;
  *)
    echo "unknown platform '$platform'" >&2
    exit 1
    ;;
esac

root="$(cd "$(dirname "$0")/.." && pwd)"
bin_dir="$(cd "$bin_dir" && pwd)"
version="$(sed -n 's/^version = "\(.*\)"$/\1/p' "$root/Cargo.toml" | head -n 1)"
[ -n "$version" ] || { echo "could not read the version from Cargo.toml" >&2; exit 1; }

name="stamp-$version-$label"
out="$root/dist"
stage="$out/$name"
rm -rf "$stage"
mkdir -p "$stage"

# Documentation and a small set of example data go in every package.
cp "$root/README.md" "$root/LICENSE.txt" "$stage/"
cp "$root/docs/CHANGES_FROM_PYTHON.md" "$stage/"
# The fonts of plots and of the menu icons are compiled into the programs;
# their licenses go with them.
mkdir -p "$stage/fonts"
cp "$root/crates/stamp-plot/fonts/LICENSE-Liberation.txt" \
   "$root/crates/stamp-plot/fonts/LICENSE-DejaVu.txt" \
   "$root/crates/stamp-gui/assets/Phosphor-LICENSE.txt" "$stage/fonts/"
mkdir -p "$stage/examples"
cp -r "$root/examples/EnterotypesArumugam" "$root/examples/IronMineEdwards" "$stage/examples/"
assets="$root/crates/stamp-gui/assets"

case "$platform" in
  linux)
    # stamp-mcp is the bridge Claude Desktop starts; STAMP builds its
    # extension from the copy beside it.
    cp "$bin_dir/stamp" "$bin_dir/stamp-cli" "$bin_dir/stamp-mcp" "$stage/"
    cp "$assets/stamp.desktop" "$stage/"
    cp "$assets/icon.png" "$stage/stamp.png"
    (cd "$out" && tar -czf "$name.tar.gz" "$name")
    archive="$out/$name.tar.gz"
    ;;
  macos)
    app="$stage/STAMP.app/Contents"
    mkdir -p "$app/MacOS" "$app/Resources"
    cp "$bin_dir/stamp" "$bin_dir/stamp-mcp" "$app/MacOS/"
    cp "$assets/STAMP.icns" "$app/Resources/"
    sed "s/@VERSION@/$version/g" "$assets/Info.plist" > "$app/Info.plist"
    cp "$bin_dir/stamp-cli" "$stage/"
    (cd "$out" && zip -qry "$name.zip" "$name")
    archive="$out/$name.zip"
    ;;
  windows)
    cp "$bin_dir/stamp.exe" "$bin_dir/stamp-cli.exe" "$bin_dir/stamp-mcp.exe" "$stage/"
    if command -v zip >/dev/null 2>&1; then
      (cd "$out" && zip -qr "$name.zip" "$name")
    else
      # GitHub's Windows runners have 7-Zip but not zip.
      (cd "$out" && 7z a -bso0 "$name.zip" "$name")
    fi
    archive="$out/$name.zip"
    ;;
esac

rm -rf "$stage"
echo "$archive"
