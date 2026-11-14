#!/usr/bin/env bash
# Build, sign, notarize, and staple the macOS app.
#
# Required for distribution:
#   GITUI_SIGN_IDENTITY  "Developer ID Application: Name (TEAMID)"
#   GITUI_NOTARY_PROFILE notarytool keychain profile
#
# Optional:
#   GITUI_VERSION (default 1.0.0), GITUI_BUILD (default timestamp)
#   GITUI_OUT     (default dist)
set -euo pipefail

cd "$(dirname "$0")/.."

VERSION="${GITUI_VERSION:-1.0.0}"
BUILD="${GITUI_BUILD:-$(date +%Y%m%d%H%M)}"
OUT="${GITUI_OUT:-dist}"
IDENTITY="${GITUI_SIGN_IDENTITY:-}"
PROFILE="${GITUI_NOTARY_PROFILE:-}"

if [[ -n "$PROFILE" && -z "$IDENTITY" ]]; then
  echo "error: GITUI_NOTARY_PROFILE requires GITUI_SIGN_IDENTITY" >&2
  exit 1
fi

mkdir -p "$OUT"

nsh package \
  --target macos \
  --out "$OUT" \
  --name gitui \
  --id dev.gitui.app \
  --app-version "$VERSION" \
  --build "$BUILD"

APP="$OUT/gitui.app"
ZIP="$OUT/gitui-$VERSION.zip"

if [[ -n "$IDENTITY" ]]; then
  codesign \
    --force \
    --deep \
    --options runtime \
    --timestamp \
    --sign "$IDENTITY" \
    "$APP"
  codesign --verify --deep --strict --verbose=2 "$APP"
else
  echo "warning: GITUI_SIGN_IDENTITY unset; keeping nsh's ad-hoc signature" >&2
fi

# Notarization takes a zip of the app; submit before stapling, then re-zip.
ditto -c -k --keepParent "$APP" "$ZIP"

if [[ -n "$PROFILE" ]]; then
  xcrun notarytool submit "$ZIP" \
    --keychain-profile "$PROFILE" \
    --wait
  xcrun stapler staple "$APP"
  xcrun stapler validate "$APP"
  rm -f "$ZIP"
  ditto -c -k --keepParent "$APP" "$ZIP"
fi

shasum -a 256 "$ZIP"
echo "app: $APP"
echo "zip: $ZIP"
