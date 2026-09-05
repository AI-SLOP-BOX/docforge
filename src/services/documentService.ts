import { invoke } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'

export interface SessionInfo {
  id: string
  dirty: boolean
  undoCount: number
  redoCount: number
}

export interface TextBlock {
  id: number
  text: string
  x: number
  y: number
  width: number
  height: number
  font_name: string
  font_size: number
  color: string
  page_index: number
}

export interface PageDimensions {
  width: number
  height: number
}

export interface SearchResult {
  page: number
  text: string
}

export interface Bookmark {
  title: string
  page: number
}

export interface FormField {
  name: string
  type: string
  value: string
}

export class DocumentService {
  /**
   * Open native file dialog to pick a PDF and read its bytes.
   */
  static async openFileDialog(): Promise<{ path: string; name: string; bytes: number[] } | null> {
    const selected = await open({
      filters: [{ name: 'PDF', extensions: ['pdf'] }],
      multiple: false,
    })
    if (!selected || typeof selected !== 'string') return null
    const bytes = await invoke<number[]>('read_file_bytes', { path: selected })
    const name = selected.split(/[/\\]/).pop() || 'document.pdf'
    return { path: selected, name, bytes }
  }

  /**
   * Save bytes to native file path picked by user.
   */
  static async saveFileDialog(defaultName: string, data: number[]): Promise<string | null> {
    const path = await save({
      defaultPath: defaultName || 'document.pdf',
      filters: [{ name: 'PDF', extensions: ['pdf'] }],
    })
    if (!path) return null
    await invoke('write_file_bytes', { path, data })
    return path
  }

  /**
   * Create an in-memory document session on the Rust backend with per-document RwLock.
   */
  static async createSession(data: number[]): Promise<string> {
    return invoke<string>('session_open_pdf', { data })
  }

  /**
   * Close and evict an in-memory document session.
   */
  static async closeSession(docId: string): Promise<boolean> {
    return invoke<boolean>('session_close', { docId })
  }

  /**
   * Retrieve serialized PDF bytes from active session.
   */
  static async getSessionBytes(docId: string): Promise<number[]> {
    return invoke<number[]>('session_get_bytes', { docId })
  }

  /**
   * Rotate a page using a lightweight delta command.
   */
  static async rotatePage(docId: string, pageIndex: number, degrees: number): Promise<void> {
    return invoke<void>('session_rotate_page', {
      docId,
      pageIndex,
      degrees,
    })
  }

  /**
   * Delete a page using a byte-bounded snapshot backup.
   */
  static async deletePage(docId: string, pageIndex: number): Promise<void> {
    return invoke<void>('session_delete_page', {
      docId,
      pageIndex,
    })
  }

  /**
   * Undo last operation on session. Returns true if an action was undone.
   */
  static async undo(docId: string): Promise<boolean> {
    return invoke<boolean>('session_undo', { docId })
  }

  /**
   * Redo previously undone operation on session. Returns true if an action was redone.
   */
  static async redo(docId: string): Promise<boolean> {
    return invoke<boolean>('session_redo', { docId })
  }

  /**
   * Inspection queries
   */
  static async getPageCount(data: number[]): Promise<number> {
    return invoke<number>('get_page_count', { data })
  }

  static async getPageDimensions(data: number[], pageIndex: number): Promise<PageDimensions> {
    return invoke<PageDimensions>('get_page_dimensions', {
      data,
      page_index: pageIndex,
    })
  }

  static async getTextBlocks(data: number[], pageIndex: number): Promise<TextBlock[]> {
    return invoke<TextBlock[]>('get_text_blocks', {
      data,
      page_index: pageIndex,
    })
  }

  static async getPdfMetadata(data: number[]): Promise<Record<string, unknown>> {
    return invoke<Record<string, unknown>>('get_pdf_metadata', { data })
  }

  static async getBookmarks(data: number[]): Promise<Bookmark[]> {
    return invoke<Bookmark[]>('get_bookmarks', { data })
  }

  static async getFormFields(data: number[]): Promise<FormField[]> {
    return invoke<FormField[]>('get_form_fields', { data })
  }

  static async searchPdf(data: number[], query: string): Promise<SearchResult[]> {
    return invoke<SearchResult[]>('search_text', { data, query })
  }
}
