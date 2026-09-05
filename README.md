# DocForge - PDF & Document Tool

プロフェッショナル仕様の高度な機能を備えた統合型PDFエディター。100%完全ローカル処理。

## 機能一覧

### PDF操作（110関数以上）
- マージ/分割/削除/回転/並べ替え/複製/抽出
- テキスト追加/画像追加
- トリミング/クロップ
- ウォーターマーク/ヘッダー/フッター/ページ番号
- ブックマーク/ベイズナンバリング
- PDF最適化/破損救済・修復
- PDF/A変換
- パスワード保護/解除

### テキスト編集（ダイレクト編集）
- テキストブロック選択・編集
- テキスト移動・削除
- フォント検出・変換
- テキスト色・サイズ変更
- 段落リフロー

### 注釈
- ハイライト/下線
- ステッキーノート
- 図形/線
- スタンプ
- リプライ機能
- ステータス管理
- XFDFインポート/エクスポート

### フォーム
- フォーム作成/編集
- 計算フィールド
- フォームデータ集計
- チェックボックス/ラジオボタン
- ドロップダウン/署名フィールド

### セキュリティ
- パスワード保護
- デジタル署名（PKCS#7）
- ハードウェアトークン（PKCS#11）
- 墨消し（完全データ消去）

### 変換
- PDF→画像（JPG/PNG）
- 画像→PDF
- HTML→PDF
- PDF→テキスト/CSV
- PDF/A変換

### OCR
- 高精度文字認識（Tesseract LSTM）
- 検索可能PDF作成
- EPUB変換
- レイアウト保持OCR

### カラーマネジメント
- RGB↔CMYK変換
- ICCプロファイル埋め込み
- インキ総量チェック

### 印刷製版
- プリフライトチェック
- フォント埋め込み検証
- PDF/X変換
- 色分解プレビュー

### 高度なエンジニアリング機能
- PDFポートフォリオ
- アクションウィザード
- トランスペアレンシー平坦化
- アクセシビリティチェック
- JavaScript埋め込み
- デジタルID管理
- タイムスタンプ
- 証明書ストア連携
- 破損PDFバイナリ救済・修復
- スキャン文書の傾き自動補正・裏写り除去
- 2文書の差分比較（テキスト・座標解析）

## ダウンロード（ワンクリック起動）

開発環境（Node.js/Rust/Docker等）のセットアップは一切不要です。[Releases ページ](https://github.com/AI-SLOP-BOX/docforge/releases) からお使いのOSに合ったファイルをダウンロードするだけで即座に利用できます。

| OS / デバイス | 配布形式 | ダウンロード・インストール |
| :--- | :--- | :--- |
| **macOS** (Apple Silicon / Intel) | `.dmg` | [最新の .dmg を入手](https://github.com/AI-SLOP-BOX/docforge/releases) → Applications にドラッグして起動 |
| **Android** (スマートフォン/タブレット) | `.apk` | [最新の .apk を入手](https://github.com/AI-SLOP-BOX/docforge/releases) → タップしてそのままインストール |
| **Linux** (Ubuntu / Fedora / Arch 等) | `.AppImage`, `.deb` | [最新の .AppImage を入手](https://github.com/AI-SLOP-BOX/docforge/releases) → 実行権限を付与して即起動 |
| **Windows** (10 / 11) | `.exe` (ポータブル / インストーラー) | [最新の .exe を入手](https://github.com/AI-SLOP-BOX/docforge/releases) → ダブルクリックで即起動 |

### 1行ワンライナー導入（macOS / Linux）
```bash
curl -fsSL https://raw.githubusercontent.com/AI-SLOP-BOX/docforge/main/install.sh | bash
```

---

## 開発用・ソースコードからのビルド

### 必要なもの
- [Node.js](https://nodejs.org/) (v18以上)
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri CLI](https://tauri.app/)

### macOS
```bash
# Homebrewでインストール
brew install node rust poppler tesseract tesseract-lang

# プロジェクトをクローン
git clone <repository-url>
cd docforge

# 依存関係をインストール
npm install

# 開発モードで実行
npx tauri dev
```

### Windows
```bash
# Chocolateyでインストール
choco install nodejs rust poppler tesseract

# またはScoopでインストール
scoop install nodejs rust poppler tesseract

# Visual Studio Build Toolsが必要
# https://visual.microsoft.com/visual-cpp-build-tools

# プロジェクトをクローン
git clone <repository-url>
cd docforge

# 依存関係をインストール
npm install

# 開発モードで実行
npx tauri dev
```

### Linux (Ubuntu/Debian)
```bash
# aptでインストール
sudo apt update
sudo apt install nodejs npm rustc poppler-utils tesseract-ocr tesseract-ocr-jpn build-essential libwebkit2gtk-4.1-dev libgtk-3-dev libappindicator3-dev librsvg2-dev patchelf

# プロジェクトをクローン
git clone <repository-url>
cd docforge

# 依存関係をインストール
npm install

# 開発モードで実行
npx tauri dev
```

### Linux (Fedora/RHEL)
```bash
# dnfでインストール
sudo dnf install nodejs npm rust poppler-utils tesseract tesseract-langpack-jpn webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel librsvg2-devel patchelf

# プロジェクトをクローン
git clone <repository-url>
cd docforge

# 依存関係をインストール
npm install

# 開発モードで実行
npx tauri dev
```

### Linux (Arch)
```bash
# pacmanでインストール
sudo pacman -S nodejs npm rust poppler tesseract tesseract-data-jpn webkit2gtk gtk3 libappindicator-gtk3 librsvg patchelf

# プロジェクトをクローン
git clone <repository-url>
cd docforge

# 依存関係をインストール
npm install

# 開発モードで実行
npx tauri dev
```

## ビルド

### 開発ビルド
```bash
npx tauri dev
```

### リリースビルド
```bash
npx tauri build
```

ビルド成果物は以下に生成されます：
- macOS: `src-tauri/target/release/bundle/`
- Windows: `src-tauri/target/release/bundle/`
- Linux: `src-tauri/target/release/bundle/`

## ディレクトリ構成

```
docforge/
├── .gitignore
├── index.html
├── package.json
├── package-lock.json
├── tsconfig.json
├── tsconfig.node.json
├── vite.config.ts
├── src/
│   ├── App.tsx
│   ├── main.tsx
│   ├── types.ts
│   ├── components/
│   │   ├── PDFViewer.tsx
│   │   └── Sidebar.tsx
│   ├── styles/
│   │   └── global.css
│   └── views/
│       ├── PDFEditorView.tsx
│       ├── ScannerView.tsx
│       └── OCRView.tsx
└── src-tauri/
    ├── Cargo.toml
    ├── Cargo.lock
    ├── build.rs
    ├── tauri.conf.json
    ├── icons/
    └── src/
        ├── lib.rs
        ├── pdf_engine.rs
        ├── image_engine.rs
        └── ocr_engine.rs
```

## トラブルシューティング

### macOS: "developer"の確認
```bash
xcode-select --install
```

### Windows: ビルドエラー
- [Visual Studio Build Tools](https://visual.microsoft.com/visual-cpp-build-tools) をインストール
- "C++ build tools"ワークロードを選択

### Linux: 依存関係エラー
Linuxでビルドまたは実行する際は、Tauri v2に必要なWebKitGTKおよびPoppler等のネイティブライブラリが必要です。

```bash
# Ubuntu / Debian
sudo apt update
sudo apt install -y build-essential curl wget file libssl-dev libgtk-3-dev \
    libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf \
    poppler-utils tesseract-ocr tesseract-ocr-jpn

# Fedora / RHEL
sudo dnf install -y gcc gcc-c++ make openssl-devel gtk3-devel \
    webkit2gtk4.1-devel libappindicator-gtk3-devel librsvg2-devel patchelf \
    poppler-utils tesseract tesseract-langpack-jpn

# Arch Linux
sudo pacman -S --needed base-devel openssl gtk3 webkit2gtk-4.1 \
    libappindicator-gtk3 librsvg patchelf poppler tesseract tesseract-data-jpn
```

### OCRが動かない
```bash
# tesseractがインストールされているか確認
tesseract --version

# 言語データ（日本語）が不足している場合
# macOS: brew install tesseract-lang
# Ubuntu: sudo apt install tesseract-ocr-jpn
# Fedora: sudo dnf install tesseract-langpack-jpn
# Windows: choco install tesseract またはインストーラーで追加
```

## ライセンス・依存関係

本プロジェクトは [MIT License](LICENSE) の下で公開されています。

### サードパーティおよび派生コードのライセンス
- **Tauri / Tao**: DocForgeに含まれるウィンドウ管理パッチ（`tao-patch`）は、[Tauri Programme within The Commons Conservancy](https://github.com/tauri-apps/tao) の著作物であり、**Apache License 2.0** の下で提供されています。
- **pdfjs-dist**: [Mozilla Foundation](https://github.com/mozilla/pdf.js) の著作物であり、**Apache License 2.0** の下で提供されています。
- **Poppler (`pdftocairo`)**: PDFのレンダリングおよびベクターアウトライン化処理に外部バイナリとして連携します（GPLv2/GPLv3）。
- **Tesseract OCR**: 光学文字認識エンジンとして外部CLI連携します（Apache License 2.0）。


