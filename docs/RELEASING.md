# Building and releasing STAMP

## What is built

| Package | Contents |
|---|---|
| `stamp-<version>-linux-x86_64.tar.gz` | `stamp` (application), `stamp-cli`, a `.desktop` entry and icon, examples, documentation |
| `stamp-<version>-windows-x86_64.zip` | `stamp.exe`, `stamp-cli.exe`, examples, documentation |
| `stamp-<version>-macos-arm64.zip`, `-macos-x86_64.zip` | `STAMP.app`, `stamp-cli`, examples, documentation |
| `stamp-<version>-web.zip` | The browser version: static files for any web server |

Each desktop program is a single executable with nothing to install.

## Building by hand

```bash
cargo build --release -p stamp-gui -p stamp-cli
scripts/package.sh linux target/release linux-x86_64     # or: macos, windows
```

The archive is written to `dist/`. The browser version is built with
[trunk](https://trunkrs.dev):

```bash
cargo install trunk --locked
cd crates/stamp-gui && trunk build --release             # writes crates/stamp-gui/dist
```

The compiler version is pinned in `rust-toolchain.toml`; `rustup` installs it
on first use.

## Continuous integration

`.github/workflows/ci.yml` runs on every push to `master` or `rust-port` and
on pull requests. It checks formatting, runs the linter with warnings as
errors, runs the tests on Linux, Windows and macOS, and builds the browser
version. The interface tests need no display.

## Making a release

1. Set the version in the root `Cargo.toml` (`[workspace.package]`), and
   commit.
2. Tag that commit with the same version and push the tag:

   ```bash
   git tag v3.0.0
   git push origin v3.0.0
   ```

3. `.github/workflows/release.yml` tests and builds every package and
   attaches them to a **draft** release. Review it on GitHub, edit the notes,
   and publish it.

The workflow can also be started by hand from the Actions tab to try the
builds without creating a release; the packages are then available as
artifacts of the run.

## Publishing the browser version

`.github/workflows/pages.yml` publishes the browser version to GitHub Pages.
It runs only when started by hand, and needs Pages enabled for the repository
with "GitHub Actions" as the source (Settings > Pages).

## Known limits

* **macOS: the application is not signed or notarised.** That needs an Apple
  Developer ID. Until then, users must right-click `STAMP.app` and choose
  Open the first time, or clear the quarantine flag with
  `xattr -dr com.apple.quarantine STAMP.app`.
* **Windows: the executable is not signed**, so SmartScreen may warn on first
  run.
* **Linux: the build links against the C library of Ubuntu 22.04** (glibc
  2.35) and will not start on older distributions. Build from source there.
* The Linux application needs OpenGL, and an XDG desktop portal for its file
  dialogs, as provided by current desktop environments.
