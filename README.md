# DocForge - Desktop & Mobile PDF Suite

A high-performance, professional-grade integrated PDF editor with 100% local, offline processing. Built with Tauri v2, Rust, and React.

[English](README.md) | [日本語](README.ja.md)

---

## Features

### Core PDF Manipulation (110+ Operations)
- Merge, Split, Delete, Rotate, Reorder, Duplicate, Extract
- Insert Text, Insert Images, Dynamic Overlays
- Crop, Trim, Margin Adjustments
- Watermarks, Headers, Footers, Page Numbering
- Bookmarks, Bates Numbering
- Document Optimization, Corrupted PDF Repair & Salvage
- PDF/A Archival Conversion
- Password Protection, Permission Flags

### Direct Text Editing
- Select & edit text blocks directly in PDF stream
- Move, reflow, or delete text paragraphs
- Automatic font detection and matching
- Typography formatting: color, size, baseline, alignment
- Text reflow across bounding boxes

### Annotations & Review
- Highlights, Underlines, Squiggles, Strikethroughs
- Sticky notes, Freehand ink drawing
- Vector shapes: rectangles, circles, arrows, lines
- Custom stamps & status badges
- Comment threads with hierarchical replies
- Review workflow status tracking (Accepted, Rejected, Completed)
- XFDF import & export

### Interactive Forms
- Form creation & field layout
- Calculation fields & automated arithmetic
- Form data collection & summary export
- Checkboxes, radio groups, dropdown combos
- Digital signature fields

### Security & Compliance
- AES-128 / AES-256 document encryption
- Digital Signatures (PKCS#7 / PAdES)
- Hardware Token Support (PKCS#11)
- Deep Redaction (complete physical eradication of underlying bytes & streams)

### Conversion & Interoperability
- PDF to High-Res Images (PNG, JPEG, WebP)
- Images to Vectorized PDF
- HTML to Print-Quality PDF
- PDF to Plain Text & Structured CSV
- PDF/A & PDF/X Standards Compliance

### OCR (Optical Character Recognition)
- High-accuracy multilingual text recognition (Tesseract LSTM engine)
- Searchable PDF generation (scanned image XObjects + invisible selectable text overlay)
- EPUB digital book conversion
- Layout-preserving OCR reconstruction

### Color Management & Prepress
- RGB to CMYK color space transformations
- ICC Output Intent embedding
- Total Area Coverage (TAC) ink limit inspection
- Preflight verification and font embedding validation
- Color separation preview

### Advanced Engineering & Forensics
- Structural PDF inspection & object tree visualizer
- Transparency flattening
- PDF/UA accessibility compliance validation
- Document difference comparison (visual and textual diffing)
- Automatic skew angle detection & bleed-through shadow removal

---

## Downloads

No development environment (Node.js, Rust, Docker) is required. Download the pre-built binary for your operating system from the [Releases](https://github.com/AI-SLOP-BOX/docforge/releases) page:

| Platform | Format | Installation |
| :--- | :--- | :--- |
| **macOS** (Apple Silicon / Intel) | `.dmg` | Download `.dmg` from [Releases](https://github.com/AI-SLOP-BOX/docforge/releases), drag to `Applications`, and launch. |
| **Android** (Mobile / Tablet) | `.apk` | Download `.apk` from [Releases](https://github.com/AI-SLOP-BOX/docforge/releases) and tap to install. |
| **Linux** (Ubuntu, Fedora, Arch, etc.) | `.AppImage`, `.deb` | Download `.AppImage`, make executable (`chmod +x`), and run directly. |
| **Windows** (10 / 11) | `.exe` | Download `.exe` from [Releases](https://github.com/AI-SLOP-BOX/docforge/releases) and run installer or portable executable. |

### One-Line Install Script (macOS / Linux)
```bash
curl -fsSL https://raw.githubusercontent.com/AI-SLOP-BOX/docforge/main/install.sh | bash
```

---

## Building from Source

### Prerequisites
- [Node.js](https://nodejs.org/) (v18 or higher)
- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)
- [Tauri CLI](https://tauri.app/) (v2)

### macOS
```bash
# Install system tools via Homebrew
brew install node rust poppler tesseract tesseract-lang

# Clone repository
git clone https://github.com/AI-SLOP-BOX/docforge.git
cd docforge

# Install dependencies
npm install

# Run development mode
npx tauri dev
```

### Linux (Ubuntu / Debian)
```bash
sudo apt update
sudo apt install -y build-essential curl wget file libssl-dev libgtk-3-dev \
    libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf \
    poppler-utils tesseract-ocr tesseract-ocr-jpn

git clone https://github.com/AI-SLOP-BOX/docforge.git
cd docforge
npm install
npx tauri dev
```

### Linux (Fedora / RHEL)
```bash
sudo dnf install -y gcc gcc-c++ make openssl-devel gtk3-devel \
    webkit2gtk4.1-devel libappindicator-gtk3-devel librsvg2-devel patchelf \
    poppler-utils tesseract tesseract-langpack-jpn

git clone https://github.com/AI-SLOP-BOX/docforge.git
cd docforge
npm install
npx tauri dev
```

### Windows
```bash
# Install using Chocolatey
choco install nodejs rust poppler tesseract

# Visual Studio C++ Build Tools required:
# https://visual.microsoft.com/visual-cpp-build-tools

git clone https://github.com/AI-SLOP-BOX/docforge.git
cd docforge
npm install
npx tauri dev
```

---

## Production Build

```bash
# Compile and package production desktop app
npx tauri build
```

The compiled bundles are generated in:
- macOS: `src-tauri/target/release/bundle/dmg/`
- Windows: `src-tauri/target/release/bundle/nsis/` or `msi/`
- Linux: `src-tauri/target/release/bundle/appimage/` or `deb/`

---

## Project Structure

```
docforge/
├── .github/workflows/       # CI/CD workflows for quality checks and multiplatform builds
├── src/                     # React + TypeScript frontend
│   ├── App.tsx              # Main view router & responsive layout
│   ├── main.tsx             # Application bootstrap
│   ├── types.ts             # Shared frontend type definitions
│   ├── components/          # Reusable UI components and tool panels
│   │   ├── CommandPalette.tsx # Universal command palette (Cmd+K)
│   │   ├── PDFViewer.tsx    # Core PDF viewer container
│   │   ├── UIControls.tsx   # Design system primitive components
│   │   └── ...              # Specialized panels for Annotate, Forms, Security, etc.
│   ├── views/               # Major view modes: PDFEditorView, ScannerView, OCRView
│   └── utils/               # Helpers, error handling, and internationalization
└── src-tauri/               # Tauri v2 backend & Rust PDF engine
    ├── Cargo.toml           # Rust package configuration
    ├── src/
    │   ├── lib.rs           # Tauri command registration & IPC boundary
    │   ├── image_engine.rs  # Scan enhancement, contrast, and perspective corrections
    │   ├── ocr_engine.rs    # OCR pipelines & searchable PDF creation
    │   └── pdf_engine/      # Modular Rust PDF processing engine
    │       ├── common.rs    # Core low-level lopdf helpers
    │       ├── redact.rs    # Physical byte-level redaction
    │       ├── forms.rs     # Interactive form processing
    │       ├── security.rs  # Encryption & digital signature validation
    │       ├── pdf_x.rs     # Preflight & PDF/X standard compliance
    │       └── ...
```

---

## Verification & Testing

DocForge includes automated test suites covering PDF integrity, byte-level redaction, and searchable PDF generation:

```bash
# Run Rust unit tests
cargo test --manifest-path src-tauri/Cargo.toml --lib

# Run code formatting check
cargo fmt --manifest-path src-tauri/Cargo.toml --check

# Check TypeScript build
npm run build
```

---

## License & Attribution

DocForge is open-source software licensed under the [MIT License](LICENSE).

### Third-Party & Derivative Code
- **Tauri / Tao**: The window management layer includes patches adapted from [Tauri / Tao](https://github.com/tauri-apps/tao), licensed under the **Apache License 2.0**.
- **pdfjs-dist**: PDF rendering is powered by [Mozilla PDF.js](https://github.com/mozilla/pdf.js), licensed under the **Apache License 2.0**.
- **Poppler**: Rendering and rasterization interop uses Poppler utilities (GPLv2/GPLv3).
- **Tesseract OCR**: Optical character recognition is powered by the Tesseract engine (Apache License 2.0).



