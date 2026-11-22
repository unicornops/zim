#!/usr/bin/env bash
set -euo pipefail

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml)
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]
bundle=Zim.app
staging=$(mktemp -d)
keychain=
cleanup() {
  if [[ -n "$keychain" ]]; then
    security delete-keychain "$keychain" || true
  fi
  rm -rf "$staging"
}
trap cleanup EXIT

mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources/licenses" artifacts
cp target/release/zim-app "$bundle/Contents/MacOS/Zim"
cp LICENSE "$bundle/Contents/Resources/"
cp licenses/GPUI-Apache-2.0.txt "$bundle/Contents/Resources/licenses/"

cat > "$bundle/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
  <dict>
    <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
    <key>CFBundleExecutable</key><string>Zim</string>
    <key>CFBundleIdentifier</key><string>ie.unicornops.zim</string>
    <key>CFBundleName</key><string>Zim</string>
    <key>CFBundleDisplayName</key><string>Zim</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>CFBundleVersion</key><string>$version</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>LSMinimumSystemVersion</key><string>12.0</string>
  </dict>
</plist>
PLIST
plutil -lint "$bundle/Contents/Info.plist"

if [[ -n "${APPLE_CERTIFICATE_BASE64:-}" ]]; then
  keychain="$staging/signing.keychain-db"
  password=$(uuidgen)
  security create-keychain -p "$password" "$keychain"
  security set-keychain-settings -lut 7200 "$keychain"
  security unlock-keychain -p "$password" "$keychain"
  printf '%s' "$APPLE_CERTIFICATE_BASE64" | base64 --decode > "$staging/cert.p12"
  security import "$staging/cert.p12" -k "$keychain" -P "${APPLE_CERTIFICATE_PASSWORD:-}" -T /usr/bin/codesign
  rm -f "$staging/cert.p12"
  security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$password" "$keychain"
  identity=$(security find-identity -v -p codesigning "$keychain" | awk '/Developer ID Application/ && !found {print $2; found=1}')
  test -n "$identity"
  codesign --force --sign "$identity" --keychain "$keychain" --options runtime --timestamp "$bundle"

  if [[ -n "${APPLE_DEVELOPER_ID:-}" && -n "${APPLE_APP_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]]; then
    ditto -c -k --keepParent "$bundle" "$staging/notarize.zip"
    auth=(--apple-id "$APPLE_DEVELOPER_ID" --password "$APPLE_APP_PASSWORD" --team-id "$APPLE_TEAM_ID")
    submission=$(xcrun notarytool submit "$staging/notarize.zip" "${auth[@]}" --wait --timeout 45m --output-format json)
    if [[ "$(jq -r '.status' <<< "$submission")" != Accepted ]]; then
      xcrun notarytool log "$(jq -r '.id' <<< "$submission")" "${auth[@]}" || true
      exit 1
    fi
    xcrun stapler staple "$bundle"
    xcrun stapler validate "$bundle"
    spctl --assess --type execute --verbose "$bundle"
  else
    echo '::notice::Apple notary credentials are unavailable. The bundle is Developer ID signed.'
  fi
else
  codesign --force --sign - "$bundle"
  echo '::notice::The macOS bundle is ad-hoc signed.'
fi
codesign --verify --deep --strict "$bundle"

ditto -c -k --sequesterRsrc --keepParent "$bundle" artifacts/zim-macos.app.zip
mkdir "$staging/dmg"
ditto "$bundle" "$staging/dmg/$bundle"
ln -s /Applications "$staging/dmg/Applications"
hdiutil create -fs HFS+ -format UDZO -volname "Zim $version" -srcfolder "$staging/dmg" "artifacts/Zim-$version.dmg"
