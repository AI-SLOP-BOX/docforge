#!/bin/bash
set -e

# DocForge One-Line Installer
REPO="AI-SLOP-BOX/docforge"
OS="$(uname -s)"
ARCH="$(uname -m)"

echo "DocForge Installer"
echo "=================="
echo "Detected Platform: $OS ($ARCH)"

case "$OS" in
  Darwin*)
    echo "Downloading DocForge for macOS..."
    LATEST_TAG=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    if [ -z "$LATEST_TAG" ]; then
      LATEST_TAG="v1.0.0"
    fi
    DMG_URL="https://github.com/$REPO/releases/download/$LATEST_TAG/DocForge_${ARCH}.dmg"
    echo "URL: $DMG_URL"
    curl -Lo /tmp/DocForge.dmg "$DMG_URL" || true
    if [ -f /tmp/DocForge.dmg ]; then
      hdiutil attach /tmp/DocForge.dmg -nobrowse -mountpoint /Volumes/DocForge
      cp -R "/Volumes/DocForge/DocForge.app" /Applications/
      hdiutil detach /Volumes/DocForge
      rm -f /tmp/DocForge.dmg
      echo "DocForge installed successfully to /Applications/DocForge.app"
    else
      echo "Pre-built binary not found. Running local build fallback..."
      git clone "https://github.com/$REPO.git"
      cd docforge && ./build.sh
    fi
    ;;
  Linux*)
    echo "Downloading DocForge for Linux (AppImage)..."
    LATEST_TAG=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    if [ -z "$LATEST_TAG" ]; then
      LATEST_TAG="v1.0.0"
    fi
    APPIMAGE_URL="https://github.com/$REPO/releases/download/$LATEST_TAG/DocForge_x86_64.AppImage"
    curl -Lo /tmp/DocForge.AppImage "$APPIMAGE_URL" || true
    if [ -f /tmp/DocForge.AppImage ]; then
      chmod +x /tmp/DocForge.AppImage
      mkdir -p "$HOME/.local/bin"
      mv /tmp/DocForge.AppImage "$HOME/.local/bin/docforge"
      echo "DocForge installed to $HOME/.local/bin/docforge"
    else
      echo "Pre-built binary not found. Running local build fallback..."
      git clone "https://github.com/$REPO.git"
      cd docforge && ./build.sh
    fi
    ;;
  *)
    echo "Unsupported OS: $OS. Please install from https://github.com/$REPO/releases"
    exit 1
    ;;
esac
