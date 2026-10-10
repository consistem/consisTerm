# Releasing consisTerm

Releases are built by `.github/workflows/release.yml`, which only calls the scripts under
`packaging/`. Every script also runs by hand on a machine of the right OS.

## Cutting a release

1. Bump `version` under `[package]` in `Cargo.toml` (e.g. `0.2.0`, or `0.2.0-rc.1` for a
   pre-release), let `Cargo.lock` follow (`cargo build`), and merge that to `main`.
2. Tag the merge commit and push the tag:

   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

   The `version` job fails the run if the tag is not exactly `v` + the Cargo.toml version.
3. The workflow builds Linux, macOS and Windows in parallel, then creates (or, on a rerun,
   refills) a **draft** release `v0.2.0` with all artifacts and `SHA256SUMS.txt`.
4. Review the draft on GitHub, write the highlights under **Novidades**, and press
   **Publish**. Nothing is public until then. A version with a `-` suffix is marked as a
   pre-release.

The release notes are written in **Brazilian Portuguese**, the language most people using the
app read. The draft opens with `packaging/release-notes.md` (a placeholder for the highlights
and the table of downloads, with `{version}` filled in), followed by GitHub's generated list of
changes under the Portuguese headings in `.github/release.yml`. A rerun against an existing
draft keeps the notes as they were edited.

A published release is never overwritten: rerunning against its tag fails. Bump the version.

To try the pipeline without releasing, run it from the Actions tab (**Run workflow**). It builds
everything and keeps the artifacts on the run, but drafts nothing.

## Artifacts

| file | what it is |
| --- | --- |
| `consisterm-<ver>-linux-x86_64.AppImage` | single-file Linux app; `chmod +x` and run. Needs no FUSE (static runtime). |
| `consisterm-<ver>-linux-x86_64.tar.gz` | the same binary as a plain tree: `bin/consisterm`, `share/applications`, `share/icons`. |
| `consisterm-<ver>-macos-universal.dmg` | `consisTerm.app` (Apple silicon + Intel), macOS 11 or later. |
| `consisterm-<ver>-windows-x64.exe` | the portable Windows executable. |
| `consisterm-<ver>-windows-x64.zip` | the same executable with the READMEs (Portuguese and English). |
| `SHA256SUMS.txt` | checksums of all of the above. |

**Linux glibc floor.** The Linux build runs on Ubuntu 22.04, so it needs glibc 2.35 or newer
(Ubuntu 22.04+, Debian 12+, Fedora 36+, RHEL 10). It also needs a desktop with X11 or Wayland,
and the Secret Service (GNOME Keyring, KWallet) for saved passwords.

**macOS signing.** Without secrets the app is signed ad-hoc and not notarized; Gatekeeper then
refuses it on first open, and the user must right-click > Open (or
`xattr -dr com.apple.quarantine /Applications/consisTerm.app`). Setting the repository secrets
`APPLE_CERTIFICATE` (base64 .p12 of a Developer ID Application certificate),
`APPLE_CERTIFICATE_PASSWORD`, and for notarization `APPLE_ID`, `APPLE_PASSWORD` (app-specific)
and `APPLE_TEAM_ID`, turns on real signing and notarization with no other change.

**Windows** builds with the MSVC toolchain in CI; the icon is embedded by `build.rs`.

## The in-app updater and asset names

The updater (`src/features/update.rs`) picks the **first** release asset whose name contains a
platform marker: `.exe` on Windows, `macos` on macOS, `linux` on Linux. So:

- exactly one asset may contain `.exe` (the release job checks this);
- the AppImage must be the first asset containing `linux`. The release job uploads it before
  the others, and GitHub lists assets in upload order. Do not add or re-upload Linux assets by
  hand ahead of it.

## Building locally

```bash
packaging/linux/package.sh            # on Linux; --formats tar to skip the AppImage
packaging/macos/package.sh            # on macOS; --arch aarch64 for a quicker single-arch build
pwsh packaging/windows/package.ps1    # on Windows
```

Output goes to `dist/release/`. The Linux script downloads `appimagetool` unless it is on
`PATH` or named by `$APPIMAGETOOL`.

## Icons

`assets/logo.png` (1024 px, transparent) is the only icon source. `assets/icon.ico` (Windows),
`assets/icon-window.png` (the window icon the taskbar shows, cut close to the ring),
`assets/icon-256.png` (Linux) and `assets/consisterm.icns` (macOS) are derived from it and
committed, so a release needs no image tools. After changing the logo, regenerate them with:

```sh
cargo run --manifest-path packaging/icons/Cargo.toml --release
```
