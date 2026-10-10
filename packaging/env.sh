# shellcheck shell=bash
# Shared setup for the packaging scripts. Source it: `. "$(dirname "$0")/../env.sh"`.
# The scripts run unchanged on a laptop and in .github/workflows/release.yml, which only calls them.
#
# Exports:
#   ROOT              repository root
#   VERSION           [package] version from Cargo.toml (override: CONSISTERM_VERSION)
#   DIST              where release artifacts land (default: $ROOT/dist/release)
#   CONSISTERM_BUILD_SHA  git commit, recorded in the macOS Info.plist
#   CARGO_TARGET_DIR  cargo's target dir (default: $ROOT/target)

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export ROOT

APP_NAME=consisTerm
BIN_NAME=consisterm
APP_ID=br.com.consistem.consisterm
export APP_NAME BIN_NAME APP_ID

# The version lives in exactly one place: `[package] version` in Cargo.toml.
package_version() {
  awk '
    /^\[/ { in_pkg = ($0 == "[package]"); next }
    in_pkg && $1 == "version" { gsub(/[" ]/, "", $3); print $3; exit }
  ' "$ROOT/Cargo.toml"
}

VERSION="${CONSISTERM_VERSION:-$(package_version)}"
if [ -z "$VERSION" ]; then
  echo "error: could not read [package] version from $ROOT/Cargo.toml" >&2
  exit 1
fi
export VERSION

DIST="${DIST:-$ROOT/dist/release}"
mkdir -p "$DIST"
export DIST

if [ -z "${CONSISTERM_BUILD_SHA:-}" ]; then
  CONSISTERM_BUILD_SHA="$(git -C "$ROOT" rev-parse HEAD 2>/dev/null || true)"
fi
export CONSISTERM_BUILD_SHA
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"

# Emit a GitHub Actions warning (plain stderr outside Actions).
warn() {
  if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::warning::$*"; else echo "warning: $*" >&2; fi
}

# Copy the readme and licence, where they exist, into a package directory.
copy_docs() {
  local dest="$1" f
  for f in README.md README.en.md LICENSE LICENSE-MIT; do
    if [ -f "$ROOT/$f" ]; then cp "$ROOT/$f" "$dest/"; fi
  done
}
