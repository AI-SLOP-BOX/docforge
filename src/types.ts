export type View = 'pdf-editor' | 'scanner' | 'ocr'

export interface PDFPageInfo {
  index: number
  width: number
  height: number
  rotation: number
}

export interface ScanResult {
  originalPath: string
  correctedPath: string
  width: number
  height: number
}

export interface OCRResult {
  text: string
  confidence: number
  pageCount: number
}

export interface AppSettings {
  outputDir: string
  defaultDPI: number
  ocrLanguage: string
}
