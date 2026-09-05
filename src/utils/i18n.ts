export type Language = 'ja' | 'en'

export const translations = {
  ja: {
    // Navigation & Workspace
    workspace: 'ワークスペース',
    pdfEditor: 'PDFエディター',
    scanner: 'スキャナー',
    ocrEpub: 'OCR / EPUB',
    pdfEditorDesc: '結合・分割・編集・署名',
    scannerDesc: '歪み補正・影除去・明度調整',
    ocrDesc: '文字認識・テキスト/EPUB変換',
    nativeCore: 'ネイティブ Rust コア',
    privacyBadge: 'クラウド非送信 · 透かしなし · 100% ローカル処理',

    // Common actions & buttons
    open: '開く',
    save: '保存',
    print: '印刷',
    undo: '元に戻す',
    redo: 'やり直し',
    cancel: 'キャンセル',
    confirm: '確認',
    close: '閉じる',
    execute: '実行',
    reset: 'リセット',
    delete: '削除',
    rotate: '回転',
    loading: '読み込み中...',
    processing: '処理中...',
    completed: '完了しました',
    errorTitle: 'エラーが発生しました',

    // Modes
    modeView: '閲覧',
    modeText: 'テキスト',
    modeHighlight: 'ハイライト',
    modeRect: '四角',
    modeRedact: '黒塗り',

    // Tabs
    tabEdit: '編集',
    tabAnnotate: '注釈',
    tabForms: 'フォーム',
    tabOrganize: '整理',
    tabPages: 'ページ',
    tabSecurity: '暗号化',
    tabText: 'テキスト',
    tabTools: 'ツール',

    // Editor empty state
    noFileLoaded: 'PDFファイルを開いてください',
    openFileBtn: 'ファイルを開く',

    // Toasts & feedback
    pdfLoaded: 'PDFを読み込みました',
    savedSuccess: 'ファイルを保存しました',
    redactApplied: '指定エリアを黒塗りしました',
    highlightAdded: 'ハイライトを追加しました',
    rectAdded: '四角形注釈を追加しました',
    textMoved: (id: number, x: number, y: number) => `テキストブロック #${id} を (${x}, ${y}) に移動しました`,
    passwordMismatch: 'パスワードが一致しません',
    verifyCount: (count: number) => `${count}件の署名を検証しました`,
    noSignatures: '署名は検出されませんでした',
  },
  en: {
    // Navigation & Workspace
    workspace: 'Workspace',
    pdfEditor: 'PDF Editor',
    scanner: 'Scanner',
    ocrEpub: 'OCR / EPUB',
    pdfEditorDesc: 'Merge, Split, Edit, Sign',
    scannerDesc: 'Deskew, Deshadow, Enhance',
    ocrDesc: 'Text Recognition & EPUB Export',
    nativeCore: 'Native Rust Core',
    privacyBadge: 'Zero Cloud · No Watermark · 100% Local',

    // Common actions & buttons
    open: 'Open',
    save: 'Save',
    print: 'Print',
    undo: 'Undo',
    redo: 'Redo',
    cancel: 'Cancel',
    confirm: 'Confirm',
    close: 'Close',
    execute: 'Execute',
    reset: 'Reset',
    delete: 'Delete',
    rotate: 'Rotate',
    loading: 'Loading...',
    processing: 'Processing...',
    completed: 'Completed',
    errorTitle: 'An error occurred',

    // Modes
    modeView: 'View',
    modeText: 'Text',
    modeHighlight: 'Highlight',
    modeRect: 'Rectangle',
    modeRedact: 'Redact',

    // Tabs
    tabEdit: 'Edit',
    tabAnnotate: 'Annotate',
    tabForms: 'Forms',
    tabOrganize: 'Organize',
    tabPages: 'Pages',
    tabSecurity: 'Security',
    tabText: 'Text',
    tabTools: 'Tools',

    // Editor empty state
    noFileLoaded: 'Please open a PDF file',
    openFileBtn: 'Open File',

    // Toasts & feedback
    pdfLoaded: 'PDF loaded successfully',
    savedSuccess: 'File saved successfully',
    redactApplied: 'Area redacted permanently',
    highlightAdded: 'Highlight added',
    rectAdded: 'Rectangle annotation added',
    textMoved: (id: number, x: number, y: number) => `Moved text block #${id} to (${x}, ${y})`,
    passwordMismatch: 'Passwords do not match',
    verifyCount: (count: number) => `Verified ${count} digital signature(s)`,
    noSignatures: 'No digital signatures detected',
  }
}

let currentLang: Language = (typeof localStorage !== 'undefined' && localStorage.getItem('docforge_lang') as Language) || 'ja'

export function getLanguage(): Language {
  return currentLang
}

export function setLanguage(lang: Language) {
  currentLang = lang
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem('docforge_lang', lang)
  }
}

export function t(): typeof translations['ja'] {
  return translations[currentLang] || translations.ja
}
