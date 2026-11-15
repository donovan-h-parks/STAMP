# Menu icons

The menu icons come from [Phosphor Icons](https://phosphoricons.com/) (MIT
licence, `crates/stamp-gui/assets/Phosphor-LICENSE.txt`). `Phosphor.ttf`
here is the whole regular-weight font, about 480 KB. STAMP compiles in only
the icons it uses (`crates/stamp-gui/assets/Phosphor.ttf`, about 6 KB), so
that the browser version downloads less.

After adding an icon to `crates/stamp-gui/src/icons.rs` (and to its `ALL`
list), regenerate the subset from the repository root:

```bash
scripts/icons/subset.sh
```

The script collects every `\u{...}` character named in `icons.rs` and runs
fontTools' `pyftsubset` (fetched by `pixi exec`). The test in `icons.rs`
checks that the font has every icon.
