#!/usr/bin/env bash
# Build and package consisTerm for Linux x86_64:
#
#   $DIST/consisterm-<version>-linux-x86_64.AppImage  any distro with glibc >= the build host's
#   $DIST/consisterm-<version>-linux-x86_64.tar.gz    plain tree (bin/, share/)
#
# Usage: packaging/linux/package.sh [--skip-build] [--formats "appimage tar"]
#
# Needs: cargo, and the -dev packages eframe and keyring link against (see release.yml).
# appimagetool is taken from $APPIMAGETOOL or PATH, else downloaded into $CARGO_TARGET_DIR.
# Build on an old distro (CI: Ubuntu 22.04, glibc 2.35) so the binary runs on newer ones.
set -euo pipefail
# shellcheck source=../env.sh
. "$(dirname "${BASH_SOURCE[0]}")/../env.sh"
HERE="$ROOT/packaging/linux"

SKIP_BUILD=0
FORMATS="appimage tar"
while [ $# -gt 0 ]; do
  case "$1" in
    --skip-build) SKIP_BUILD=1; shift ;;
    --formats) FORMATS="$2"; shift 2 ;;
    -h | --help) sed -n '2,11p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

ARCH="$(uname -m)"
case "$ARCH" in
  x86_64) ;;
  aarch64 | arm64) ARCH=aarch64 ;;
  *) echo "unsupported architecture $ARCH" >&2; exit 2 ;;
esac
BASENAME="$BIN_NAME-$VERSION-linux-$ARCH"
has() { case " $FORMATS " in *" $1 "*) return 0 ;; *) return 1 ;; esac; }

echo "==> $APP_NAME $VERSION for Linux $ARCH ($FORMATS)"

if [ "$SKIP_BUILD" = 0 ]; then
  (cd "$ROOT" && cargo build --release --locked)
fi
BIN="$CARGO_TARGET_DIR/release/$BIN_NAME"
WORK="$CARGO_TARGET_DIR/linux-package"
STAGE="$WORK/root"
rm -rf "$WORK"

# ---- stage an FHS tree, shared by both formats -------------------------------------------------
install -Dm755 "$BIN" "$STAGE/usr/bin/$BIN_NAME"
strip "$STAGE/usr/bin/$BIN_NAME" 2>/dev/null || true
install -Dm644 "$HERE/$APP_ID.desktop" "$STAGE/usr/share/applications/$APP_ID.desktop"
install -Dm644 "$ROOT/assets/icon-256.png" "$STAGE/usr/share/icons/hicolor/256x256/apps/$APP_ID.png"
mkdir -p "$STAGE/usr/share/doc/$BIN_NAME"
copy_docs "$STAGE/usr/share/doc/$BIN_NAME"

if command -v desktop-file-validate >/dev/null; then
  desktop-file-validate "$STAGE/usr/share/applications/$APP_ID.desktop"
fi

# ---- .tar.gz -----------------------------------------------------------------------------------
if has tar; then
  mkdir -p "$WORK/tar"
  cp -R "$STAGE/usr" "$WORK/tar/$BASENAME"
  tar -C "$WORK/tar" -czf "$DIST/$BASENAME.tar.gz" "$BASENAME"
  echo "wrote $DIST/$BASENAME.tar.gz"
fi

# ---- AppImage ----------------------------------------------------------------------------------
if has appimage; then
  APPDIR="$WORK/$APP_NAME.AppDir"
  cp -R "$STAGE" "$APPDIR"
  rm -rf "$APPDIR/usr/share/doc"
  ln -s "usr/bin/$BIN_NAME" "$APPDIR/AppRun"
  cp "$HERE/$APP_ID.desktop" "$APPDIR/$APP_ID.desktop"
  cp "$ROOT/assets/icon-256.png" "$APPDIR/$APP_ID.png"
  ln -s "$APP_ID.png" "$APPDIR/.DirIcon"

  TOOL="${APPIMAGETOOL:-$(command -v appimagetool || true)}"
  if [ -z "$TOOL" ]; then
    TOOL="$CARGO_TARGET_DIR/appimagetool-$ARCH.AppImage"
    if [ ! -x "$TOOL" ]; then
      curl -fsSL -o "$TOOL" \
        "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$ARCH.AppImage"
      chmod +x "$TOOL"
    fi
  fi
  OUT="$(cd "$DIST" && pwd)/$BASENAME.AppImage"
  # Extract-and-run: appimagetool is itself an AppImage, and this works without FUSE (containers,
  # CI). The output embeds the static runtime, so users do not need libfuse2 either.
  ARCH="$ARCH" APPIMAGE_EXTRACT_AND_RUN=1 "$TOOL" --no-appstream "$APPDIR" "$OUT"
  chmod +x "$OUT"
  echo "wrote $OUT"
fi

echo "==> done"
ls -lh "$DIST"
