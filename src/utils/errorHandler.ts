/**
 * Formats low-level errors into human-readable, actionable diagnostic messages.
 */
export function formatError(err: unknown, fallbackMessage = '処理中にエラーが発生しました'): string {
  if (!err) return fallbackMessage

  const rawMessage = typeof err === 'string'
    ? err
    : err instanceof Error
      ? err.message
      : String(err)

  // System command missing / not installed
  if (rawMessage.includes('No such file or directory') || rawMessage.includes('not found')) {
    if (rawMessage.includes('pdftocairo') || rawMessage.includes('poppler')) {
      return 'Poppler (pdftocairo) がシステムに見つかりません。brew install poppler または apt install poppler-utils でインストールしてください。'
    }
    if (rawMessage.includes('tesseract')) {
      return 'Tesseract OCR がシステムに見つかりません。brew install tesseract tesseract-lang または apt install tesseract-ocr でインストールしてください。'
    }
    return `依存プログラムまたはファイルが見つかりません: ${rawMessage}`
  }

  // PDF syntax / corrupted document
  if (rawMessage.includes('Invalid PDF') || rawMessage.includes('Failed to load PDF') || rawMessage.includes('syntax error')) {
    return 'PDFファイルの形式が破損しているか、対応していない暗号化が施されています。'
  }

  // Password / encryption
  if (rawMessage.includes('password') || rawMessage.includes('encrypted')) {
    return 'パスワードで保護されているか、権限が不足しています。正しいパスワードを入力してください。'
  }

  // Page range / index out of bounds
  if (rawMessage.includes('out of range') || rawMessage.includes('index')) {
    return '指定されたページ番号がドキュメントの範囲外です。'
  }

  // File permission / IO
  if (rawMessage.includes('Permission denied')) {
    return 'ファイルへのアクセス権限がありません。保存先フォルダの書き込み権限をご確認ください。'
  }

  // Return clean formatted error
  return `${fallbackMessage}: ${rawMessage}`
}
