#!/bin/bash
set -euo pipefail

APP_NAME="GTop"
BUNDLE_ID="com.gtop.app"
BINARY_NAME="gtop"
VERSION="0.1.0"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
ICON_SVG="$PROJECT_DIR/assets/icon.svg"
BUILD_DIR="$PROJECT_DIR/target/release"
APP_DIR="$PROJECT_DIR/$APP_NAME.app"

echo "==> Building release binary..."
cd "$PROJECT_DIR"
cargo +nightly build --release

echo "==> Generating .icns from SVG..."
ICONSET_DIR=$(mktemp -d)/AppIcon.iconset
mkdir -p "$ICONSET_DIR"

# Generate all required icon sizes using rsvg-convert for accurate SVG rendering
for SIZE in 16 32 64 128 256 512 1024; do
    rsvg-convert -w "$SIZE" -h "$SIZE" "$ICON_SVG" -o "$ICONSET_DIR/icon_${SIZE}x${SIZE}.png"
done
for SIZE in 16 32 128 256 512; do
    DOUBLE=$((SIZE * 2))
    cp "$ICONSET_DIR/icon_${DOUBLE}x${DOUBLE}.png" "$ICONSET_DIR/icon_${SIZE}x${SIZE}@2x.png"
done

iconutil -c icns "$ICONSET_DIR" -o "$PROJECT_DIR/assets/AppIcon.icns"
rm -rf "$(dirname "$ICONSET_DIR")"

echo "==> Creating .app bundle..."
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

cp "$BUILD_DIR/$BINARY_NAME" "$APP_DIR/Contents/MacOS/$APP_NAME"
cp "$PROJECT_DIR/assets/AppIcon.icns" "$APP_DIR/Contents/Resources/AppIcon.icns"

cat > "$APP_DIR/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>$APP_NAME</string>
    <key>CFBundleDisplayName</key>
    <string>$APP_NAME</string>
    <key>CFBundleExecutable</key>
    <string>$APP_NAME</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleIdentifier</key>
    <string>$BUNDLE_ID</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSMinimumSystemVersion</key>
    <string>13.0</string>
</dict>
</plist>
EOF

echo "==> Done! Created $APP_DIR"
echo "    Run with: open $APP_DIR"
