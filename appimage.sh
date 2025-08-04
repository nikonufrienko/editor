#!/bin/bash
set -euo pipefail

APP_NAME="editor"
TARGET_DIR="AppImage/AppDir"
APPIMAGETOOL="appimagetool-x86_64.AppImage"
APPIMAGETOOL_URL="https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage"

optimize_binary() {
    local binary="$1"
    echo "Optimizing binary size..."
    strip --strip-all "$binary"
    upx --ultra-brute "$binary" || upx --best "$binary"
}

if [ ! -f "$APPIMAGETOOL" ]; then
    echo "Downloading appimagetool..."
    wget "$APPIMAGETOOL_URL" -O "$APPIMAGETOOL"
    chmod +x "$APPIMAGETOOL"
fi

rm -rf "$TARGET_DIR" "${APP_NAME}-x86_64.AppImage"

mkdir -p "$TARGET_DIR/usr/bin"
mkdir -p "$TARGET_DIR/usr/share/applications"

cp "target/release/$APP_NAME" "$TARGET_DIR/usr/bin/"
optimize_binary "$TARGET_DIR/usr/bin/$APP_NAME"

DESKTOP_FILE="$TARGET_DIR/$APP_NAME.desktop"
cat > "$DESKTOP_FILE" <<EOL
[Desktop Entry]
Name=$APP_NAME
Exec=$APP_NAME
Icon=app_icon
Type=Application
Categories=Utility;
EOL

cp "assets/icon-256.png" "$TARGET_DIR/app_icon.png"

cat > "$TARGET_DIR/AppRun" <<EOL
#!/bin/sh
HERE=\$(dirname "\$(readlink -f "\$0")")
exec "\$HERE/usr/bin/$APP_NAME" "\$@"
EOL
chmod +x "$TARGET_DIR/AppRun"

export APPIMAGE_EXTRACT_AND_RUN=1
./"$APPIMAGETOOL" \
    --comp xz \
    --no-appstream \
    "$TARGET_DIR" \
    "${APP_NAME}-x86_64.AppImage"

echo "Successfully built optimized ${APP_NAME}-x86_64.AppImage"
rm -rf "AppImage"
