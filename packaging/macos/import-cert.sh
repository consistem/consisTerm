#!/usr/bin/env bash
# CI only: import a Developer ID certificate into a temporary keychain, if the secrets exist.
#
#   APPLE_CERTIFICATE           base64 of the .p12
#   APPLE_CERTIFICATE_PASSWORD  its password
#
# With them, exports MACOS_SIGN_IDENTITY and MACOS_KEYCHAIN to later steps through $GITHUB_ENV.
# Without them it does nothing, and package.sh signs ad-hoc.
set -euo pipefail

if [ -z "${APPLE_CERTIFICATE:-}" ] || [ -z "${APPLE_CERTIFICATE_PASSWORD:-}" ]; then
  echo "::warning::no APPLE_CERTIFICATE secret; the macOS build will be signed ad-hoc"
  exit 0
fi

KEYCHAIN="$RUNNER_TEMP/release.keychain-db"
KEYCHAIN_PASSWORD="$(openssl rand -hex 24)"
CERT="$RUNNER_TEMP/cert.p12"
printf '%s' "$APPLE_CERTIFICATE" | base64 --decode >"$CERT"

security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN"
security set-keychain-settings -lut 21600 "$KEYCHAIN"
security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN"
security import "$CERT" -P "$APPLE_CERTIFICATE_PASSWORD" -A -t cert -f pkcs12 -k "$KEYCHAIN"
security set-key-partition-list -S apple-tool:,apple: -k "$KEYCHAIN_PASSWORD" "$KEYCHAIN" >/dev/null
# Put it on the search list next to the existing keychains so codesign finds the chain.
# shellcheck disable=SC2046
security list-keychains -d user -s "$KEYCHAIN" $(security list-keychains -d user | tr -d '"')
rm -f "$CERT"

IDENTITY="$(security find-identity -v -p codesigning "$KEYCHAIN" | awk -F'"' '/Developer ID Application/ { print $2; exit }')"
if [ -z "$IDENTITY" ]; then
  echo "::error::the certificate holds no 'Developer ID Application' identity"
  exit 1
fi
{
  echo "MACOS_SIGN_IDENTITY=$IDENTITY"
  echo "MACOS_KEYCHAIN=$KEYCHAIN"
} >>"$GITHUB_ENV"
echo "imported signing identity: $IDENTITY"
