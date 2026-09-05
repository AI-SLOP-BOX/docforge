.PHONY: dev build clean install help

# Default target
help:
	@echo "DocForge - PDF & Document Tool"
	@echo ""
	@echo "Usage:"
	@echo "  make dev        - Start development server"
	@echo "  make build      - Build for release"
	@echo "  make clean      - Clean build artifacts"
	@echo "  make install    - Install dependencies"
	@echo "  make tauri-dev  - Start Tauri dev server"
	@echo "  make tauri-build - Build Tauri app"
	@echo ""

# Install dependencies
install:
	npm install

# Start Vite dev server
dev:
	npm run dev

# Build frontend
build-frontend:
	npm run build

# Start Tauri dev server
tauri-dev:
	npx tauri dev

# Build Tauri app
tauri-build:
	npx tauri build

# Clean build artifacts
clean:
	rm -rf dist
	rm -rf src-tauri/target
	rm -rf node_modules

# Clean everything
clean-all: clean
	rm -rf package-lock.json

# Run tests
test:
	npm test

# Lint code
lint:
	npm run lint

# Format code (Rust)
fmt:
	cd src-tauri && cargo fmt

# Check code (Rust)
check:
	cd src-tauri && cargo check

# Clippy (Rust linter)
clippy:
	cd src-tauri && cargo clippy

# Build for specific platforms
build-macos:
	npx tauri build --target aarch64-apple-darwin
	npx tauri build --target x86_64-apple-darwin

build-windows:
	npx tauri build --target x86_64-pc-windows-msvc

build-linux:
	npx tauri build --target x86_64-unknown-linux-gnu

# Build all platforms (requires cross-compilation setup)
build-all: build-macos build-windows build-linux

# Install Tauri CLI
install-tauri:
	cargo install tauri-cli

# Install all dependencies
setup: install install-tauri
	@echo "Setup complete! Run 'make dev' to start."
