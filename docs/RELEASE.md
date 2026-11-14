# RELEASE — macOS

## Build

```sh
cd apps/gitui-macos
GITUI_VERSION=1.0.0 GITUI_BUILD=1 ./scripts/release-macos.sh
```

Without `GITUI_SIGN_IDENTITY`, this produces the ad-hoc signed app from
`nsh package` and a zip under `dist/`. It is for local verification only.

## Sign and notarize

Prerequisites:

- Xcode Command Line Tools
- `Developer ID Application` certificate in the login keychain
- a notarytool profile:

```sh
xcrun notarytool store-credentials gitui-notary \
  --apple-id "you@example.com" \
  --team-id "TEAMID" \
  --password "app-specific-password"
```

Run:

```sh
cd apps/gitui-macos
GITUI_VERSION=1.0.0 \
GITUI_BUILD=1 \
GITUI_SIGN_IDENTITY="Developer ID Application: Name (TEAMID)" \
GITUI_NOTARY_PROFILE=gitui-notary \
./scripts/release-macos.sh
```

The script:

1. runs `nsh package --target macos`
2. codesigns with hardened runtime and timestamp
3. verifies the signature
4. zips the app
5. submits to `notarytool --wait`
6. staples and validates the ticket
7. re-zips the stapled app
8. prints the SHA-256

## Sparkle 2

Sparkle is not wired into the app yet. Once `Sparkle.framework` is embedded,
publish the final `dist/gitui-$VERSION.zip` with Sparkle's
`generate_appcast` and its EdDSA key, then point the app's feed URL at the
generated `appcast.xml`. Do not publish an unstapled or ad-hoc zip to the
stable feed.

## Notes

- `--deep` is used only at packaging time; the app has no nested code today.
- Verify the final artifact with `spctl --assess --type execute -vv dist/gitui.app`.
- Keep the notarytool password in Keychain, never in the repo or CI logs.
