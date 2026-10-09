#!/usr/bin/env bash
# Build, sign and (optionally) notarize the macOS release artifact:
#
#   $DIST/consisterm-<version>-macos-<arch>.dmg   consisTerm.app on a drag-to-Applications DMG
#
# Usage: packaging/macos/package.sh [--arch universal|aarch64|x86_64] [--skip-build]
#
# Signing (env, all optional):
#   MACOS_SIGN_IDENTITY   codesign identity (name or SHA-1). Default "-" = ad-hoc: the app runs on
#                         the Mac that built it, and elsewhere only after the user clears quarantine.
#   MACOS_KEYCHAIN        keychain holding the identity
# Notarization (env; all three, and a real identity): APPLE_ID, APPLE_PASSWORD (app-specific
# password), APPLE_TEAM_ID.
set -euo pipefail
# shellcheck source=../env.sh
. "$(dirname "${BASH_SOURCE[0]}")/../env.sh"
HERE="$ROOT/packaging/macos"

ARCH=universal
SKIP_BUILD=0
while [ $# -gt 0 ]; do
  case "$1" in
    --arch) ARCH="$2"; shift 2 ;;
    --skip-build) SKIP_BUILD=1; shift ;;
    -h | --help) sed -n '2,13p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
case "$ARCH" in
  universal) TARGETS=(aarch64-apple-darwin x86_64-apple-darwin) ;;
  aarch64) TARGETS=(aarch64-apple-darwin) ;;
  x86_64) TARGETS=(x86_64-apple-darwin) ;;
  *) echo "unknown --arch $ARCH" >&2; exit 2 ;;
esac

# Keep in sync with LSMinimumSystemVersion in Info.plist.in.
export MACOSX_DEPLOYMENT_TARGET=11.0
IDENTITY="${MACOS_SIGN_IDENTITY:--}"
SHORT_VERSION="${VERSION%%-*}"
WORK="$CARGO_TARGET_DIR/macos-package"
APP="$WORK/$APP_NAME.app"
DMG="$DIST/$BIN_NAME-$VERSION-macos-$ARCH.dmg"

NOTARIZE=0
if [ "$IDENTITY" = "-" ]; then
  warn "macOS: ad-hoc signing (no MACOS_SIGN_IDENTITY); the DMG is not notarized"
elif [ -n "${APPLE_ID:-}" ] && [ -n "${APPLE_PASSWORD:-}" ] && [ -n "${APPLE_TEAM_ID:-}" ]; then
  NOTARIZE=1
else
  warn "macOS: APPLE_ID / APPLE_PASSWORD / APPLE_TEAM_ID incomplete; signed but not notarized"
fi

echo "==> $APP_NAME $VERSION for macOS ($ARCH), identity: $IDENTITY, notarize: $NOTARIZE"

# ---- build -------------------------------------------------------------------------------------
if [ "$SKIP_BUILD" = 0 ]; then
  args=()
  for t in "${TARGETS[@]}"; do args+=(--target "$t"); done
  (cd "$ROOT" && cargo build --release --locked "${args[@]}")
fi

rm -rf "$WORK"
mkdir -p "$WORK"
inputs=()
for t in "${TARGETS[@]}"; do inputs+=("$CARGO_TARGET_DIR/$t/release/$BIN_NAME"); done
lipo -create -output "$WORK/$BIN_NAME" "${inputs[@]}"
lipo -info "$WORK/$BIN_NAME"

sign() {
  # A real identity gets a secure timestamp; an ad-hoc signature cannot have one.
  local ts=(--timestamp) kc=()
  [ "$IDENTITY" = "-" ] && ts=(--timestamp=none)
  [ -n "${MACOS_KEYCHAIN:-}" ] && kc=(--keychain "$MACOS_KEYCHAIN")
  codesign --force --sign "$IDENTITY" ${kc[@]+"${kc[@]}"} "${ts[@]}" "$@"
}

notarize() {
  echo "==> notarizing $(basename "$1") (this can take a few minutes)"
  xcrun notarytool submit "$1" --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" --wait --timeout 1h
}

# ---- consisTerm.app ----------------------------------------------------------------------------
echo "==> assembling $APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$WORK/$BIN_NAME" "$APP/Contents/MacOS/$BIN_NAME"
cp "$ROOT/assets/consisterm.icns" "$APP/Contents/Resources/consisterm.icns"
sed -e "s/@VERSION@/$VERSION/g" -e "s/@SHORT_VERSION@/$SHORT_VERSION/g" \
  -e "s/@BUILD_SHA@/${CONSISTERM_BUILD_SHA:-unknown}/g" \
  "$HERE/Info.plist.in" >"$APP/Contents/Info.plist"
plutil -lint "$APP/Contents/Info.plist"
printf 'APPL????' >"$APP/Contents/PkgInfo"

# Inside-out: the executable, then the bundle (no --deep on the final signature).
sign --options runtime --entitlements "$HERE/entitlements.plist" "$APP/Contents/MacOS/$BIN_NAME"
sign --options runtime --entitlements "$HERE/entitlements.plist" "$APP"
codesign --verify --strict --deep --verbose=2 "$APP"

if [ "$NOTARIZE" = 1 ]; then
  ditto -c -k --keepParent "$APP" "$WORK/notarize.zip"
  notarize "$WORK/notarize.zip"
  xcrun stapler staple "$APP"
fi

# ---- DMG ---------------------------------------------------------------------------------------
echo "==> building $DMG"
STAGE="$WORK/dmg"
mkdir -p "$STAGE"
ditto "$APP" "$STAGE/$APP_NAME.app"
ln -s /Applications "$STAGE/Applications"
rm -f "$DMG" "$WORK/raw.dmg"
# makehybrid + convert builds the image without attaching a device, unlike `create -srcfolder`,
# which is flaky on CI runners ("Resource busy").
hdiutil makehybrid -hfs -hfs-volume-name "$APP_NAME $VERSION" -hfs-openfolder "$STAGE" \
  -o "$WORK/raw.dmg" "$STAGE"
hdiutil convert "$WORK/raw.dmg" -format UDZO -imagekey zlib-level=9 -o "$DMG"
rm -f "$WORK/raw.dmg"
sign "$DMG"
codesign --verify --strict --verbose=2 "$DMG"
if [ "$NOTARIZE" = 1 ]; then
  notarize "$DMG"
  xcrun stapler staple "$DMG"
fi

echo "==> done"
ls -lh "$DMG"
