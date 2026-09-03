#!/data/data/com.termux/files/usr/bin/bash
# Rabuka Reloaded - Android Package Creator
# Run this on your PC to create a transfer package for Android

set -e

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PACKAGE_DIR="$REPO_ROOT/android_package"
PACKAGE_NAME="rabuka_android_$(date +%Y%m%d).tar.gz"

echo "Creating Android transfer package..."

# Clean previous package
rm -rf "$PACKAGE_DIR"
mkdir -p "$PACKAGE_DIR"

# Copy essential directories
echo "Copying engine source..."
cp -r "$REPO_ROOT/engine" "$PACKAGE_DIR/"

echo "Copying web_ui..."
cp -r "$REPO_ROOT/web_ui" "$PACKAGE_DIR/"

echo "Copying cards..."
cp -r "$REPO_ROOT/cards" "$PACKAGE_DIR/"

echo "Copying Android scripts..."
cp "$REPO_ROOT/android/setup_termux.sh" "$PACKAGE_DIR/"
cp "$REPO_ROOT/android/start_android.sh" "$PACKAGE_DIR/"
cp "$REPO_ROOT/android/README_ANDROID.md" "$PACKAGE_DIR/"

# Create a simple transfer script
cat > "$PACKAGE_DIR/transfer_to_android.sh" << 'EOF'
#!/bin/bash
# Run this on your PC to transfer the package to Android via ADB

PACKAGE="rabuka_android_$(date +%Y%m%d).tar.gz"
ANDROID_PATH="/sdcard/Download/"

echo "Creating package..."
tar -czf "$PACKAGE" --exclude='.git' --exclude='target' --exclude='android_package' .

echo "Transferring to Android..."
adb push "$PACKAGE" "$ANDROID_PATH"

echo ""
echo "On your Android device (in Termux), run:"
echo "  cd /sdcard/Download"
echo "  tar -xzf $PACKAGE"
echo "  cd rabuka_reloaded"
echo "  ./setup_termux.sh"
EOF
chmod +x "$PACKAGE_DIR/transfer_to_android.sh"

# Create the final tarball
echo "Creating final package: $PACKAGE_NAME"
cd "$REPO_ROOT"
tar -czf "$PACKAGE_NAME" -C "$PACKAGE_DIR" .

echo ""
echo "=========================================="
echo "  Package Created: $PACKAGE_NAME"
echo "=========================================="
echo ""
echo "To transfer to Android:"
echo "  1. Enable USB Debugging on Android"
echo "  2. Connect via USB"
echo "  3. Run: adb push $PACKAGE_NAME /sdcard/Download/"
echo "  4. In Termux: cd /sdcard/Download && tar -xzf $PACKAGE_NAME && cd rabuka_reloaded && ./setup_termux.sh"
echo ""
echo "Or use the transfer script:"
echo "  cd $PACKAGE_DIR && ./transfer_to_android.sh"
EOF
chmod +x "$PACKAGE_DIR/transfer_to_android.sh"

echo "Package ready at: $PACKAGE_DIR"
echo "Run ./transfer_to_android.sh from $PACKAGE_DIR to push via ADB"