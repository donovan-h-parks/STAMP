#!/usr/bin/env bash
# Cut the Phosphor icon font down to the icons STAMP uses, which are the
# characters named in crates/stamp-gui/src/icons.rs. The full font
# (scripts/icons/Phosphor.ttf) stays out of the build; the subset is
# compiled in (crates/stamp-gui/assets/Phosphor.ttf). Run from the
# repository root after adding an icon:
#
#     scripts/icons/subset.sh
#
# Needs pixi (https://pixi.sh), which fetches fontTools from conda-forge.
set -euo pipefail
cd "$(dirname "$0")/../.."
codepoints=$(grep -o '\\u{[0-9A-Fa-f]*}' crates/stamp-gui/src/icons.rs \
    | tr -d '\\u{}' | sort -u | sed 's/^/U+/' | paste -sd, -)
pixi exec --spec fonttools pyftsubset scripts/icons/Phosphor.ttf \
    --unicodes="$codepoints" \
    --layout-features='' --no-hinting --desubroutinize \
    --name-IDs='*' --name-languages='*' \
    --output-file=crates/stamp-gui/assets/Phosphor.ttf
ls -l scripts/icons/Phosphor.ttf crates/stamp-gui/assets/Phosphor.ttf
