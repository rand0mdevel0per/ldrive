#!/bin/bash
set -e

REPO="rand0mdevel0per/ldrive"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.ldrive}"
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"

echo "🚀 Installing LDrive..."

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case $ARCH in
    x86_64) ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    *) echo "❌ Unsupported architecture: $ARCH"; exit 1 ;;
esac

# Get latest release
RELEASE_URL="https://api.github.com/repos/$REPO/releases/latest"
DOWNLOAD_URL=$(curl -s $RELEASE_URL | grep "browser_download_url.*ldrive-node-$OS-$ARCH" | cut -d '"' -f 4)

if [ -z "$DOWNLOAD_URL" ]; then
    echo "❌ No release found for $OS-$ARCH"
    exit 1
fi

# Download and install
mkdir -p "$INSTALL_DIR" "$BIN_DIR"
echo "📦 Downloading from $DOWNLOAD_URL..."
curl -L "$DOWNLOAD_URL" -o "$INSTALL_DIR/ldrive-node"
chmod +x "$INSTALL_DIR/ldrive-node"
ln -sf "$INSTALL_DIR/ldrive-node" "$BIN_DIR/ldrive-node"

# Create config directory
mkdir -p "$HOME/.config/ldrive"

echo "✅ LDrive installed to $BIN_DIR/ldrive-node"
echo ""
echo "📝 Next steps:"
echo "1. Get your token from https://ldrive-web.pages.dev"
echo "2. Run: ldrive-node storage --token YOUR_TOKEN --storage-path ~/.ldrive/data --quota 50GB"
echo ""
echo "📚 Documentation: https://github.com/$REPO/tree/main/docs"
