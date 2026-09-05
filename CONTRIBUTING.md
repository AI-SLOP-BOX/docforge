# Contributing to DocForge

Thank you for your interest in contributing to DocForge!

## Getting Started

1. Fork the repository
2. Clone your fork
3. Install dependencies: `npm install`
4. Start development: `npx tauri dev`

## Development Setup

### Prerequisites
- Node.js (v18+)
- Rust (latest stable)
- Tauri CLI

### Platform-specific dependencies

#### macOS
```bash
brew install poppler tesseract tesseract-lang
```

#### Ubuntu/Debian
```bash
sudo apt install poppler-utils tesseract-ocr tesseract-ocr-jpn build-essential libwebkit2gtk-4.1-dev libgtk-3-dev libappindicator3-dev librsvg2-dev patchelf
```

#### Windows
- Install Visual Studio Build Tools
- Install poppler and tesseract via Chocolatey or Scoop

## Code Style

### Rust
- Follow standard Rust conventions
- Run `cargo fmt` before committing
- Run `cargo clippy` to check for warnings

### TypeScript/React
- Use functional components with hooks
- Follow existing code patterns
- Run `npm run build` to check for type errors

## Pull Request Process

1. Create a feature branch from `main`
2. Make your changes
3. Test on your platform
4. Update documentation if needed
5. Submit a pull request

## Reporting Issues

- Use GitHub Issues
- Include OS and version
- Include steps to reproduce
- Include error messages if any

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
