#!/bin/bash

# DocForge Build Script for Linux/macOS

set -e

echo "DocForge Build Script"
echo "===================="

# Check if Node.js is installed
if ! command -v node &> /dev/null; then
    echo "Error: Node.js is not installed"
    echo "Please install Node.js from https://nodejs.org/"
    exit 1
fi

# Check if Rust is installed
if ! command -v rustc &> /dev/null; then
    echo "Error: Rust is not installed"
    echo "Please install Rust from https://www.rust-lang.org/tools/install"
    exit 1
fi

# Check if npm is installed
if ! command -v npm &> /dev/null; then
    echo "Error: npm is not installed"
    echo "Please install npm from https://nodejs.org/"
    exit 1
fi

# Check if Tauri CLI is installed
if ! command -v cargo-tauri &> /dev/null && ! npx tauri --version &> /dev/null; then
    echo "Installing Tauri CLI..."
    cargo install tauri-cli
fi

# Check for platform-specific dependencies
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    echo "Checking Linux dependencies..."
    if ! dpkg -l | grep -q "libwebkit2gtk-4.1-dev"; then
        echo "Warning: libwebkit2gtk-4.1-dev is not installed"
        echo "Run: sudo apt install libwebkit2gtk-4.1-dev"
    fi
elif [[ "$OSTYPE" == "darwin"* ]]; then
    echo "Checking macOS dependencies..."
    if ! command -v brew &> /dev/null; then
        echo "Warning: Homebrew is not installed"
        echo "Install from https://brew.sh/"
    fi
fi

# Install npm dependencies
echo "Installing npm dependencies..."
npm install

# Build frontend
echo "Building frontend..."
npm run build

# Build Tauri app
echo "Building Tauri app..."
npx tauri build

echo "Build complete!"
echo "Check src-tauri/target/release/bundle/ for output files."
