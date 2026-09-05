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
   * Get undo/redo availability and metrics for active session.
   */
  static async getHistoryStatus(docId: string): Promise<{
    can_undo: boolean
    can_redo: boolean
    undo_count: number
    redo_count: number
    history_bytes: number
  }> {
    return invoke('session_get_history_status', { docId })
  }

  /**
   * Session-based Inspection & Query (Zero IPC byte passing)
   */
  static async getPageCount(docIdOrData: string | number[]): Promise<number> {
    if (typeof docIdOrData === 'string') {
      return invoke<number>('session_get_page_count', { docId: docIdOrData })
    }
    return invoke<number>('get_page_count', { data: docIdOrData })
  }

  static async getPageDimensions(docIdOrData: string | number[], pageIndex: number): Promise<PageDimensions> {
    if (typeof docIdOrData === 'string') {
      return invoke<PageDimensions>('session_get_page_dimensions', {
        docId: docIdOrData,
        pageIndex,
      })
    }
    return invoke<PageDimensions>('get_page_dimensions', {
      data: docIdOrData,
      page_index: pageIndex,
    })
  }

  static async getTextBlocks(docIdOrData: string | number[], pageIndex: number): Promise<TextBlock[]> {
    if (typeof docIdOrData === 'string') {
      return invoke<TextBlock[]>('session_get_text_blocks', {
        docId: docIdOrData,
        pageIndex,
      })
    }
    return invoke<TextBlock[]>('get_text_blocks', {
      data: docIdOrData,
      page_index: pageIndex,
    })
  }

  static async getPdfMetadata(docIdOrData: string | number[]): Promise<Record<string, unknown>> {
    if (typeof docIdOrData === 'string') {
      return invoke<Record<string, unknown>>('session_get_metadata', { docId: docIdOrData })
    }
    return invoke<Record<string, unknown>>('get_pdf_metadata', { data: docIdOrData })
  }

  static async getBookmarks(docIdOrData: string | number[]): Promise<Bookmark[]> {
    if (typeof docIdOrData === 'string') {
      return invoke<Bookmark[]>('session_get_bookmarks', { docId: docIdOrData })
    }
    return invoke<Bookmark[]>('get_bookmarks', { data: docIdOrData })
  }

  static async getFormFields(docIdOrData: string | number[]): Promise<FormField[]> {
    if (typeof docIdOrData === 'string') {
      return invoke<FormField[]>('session_get_form_fields', { docId: docIdOrData })
    }
    return invoke<FormField[]>('get_form_fields', { data: docIdOrData })
  }

  static async searchPdf(docIdOrData: string | number[], query: string): Promise<SearchResult[]> {
    if (typeof docIdOrData === 'string') {
      return invoke<SearchResult[]>('session_search_text', { docId: docIdOrData, query })
    }
    return invoke<SearchResult[]>('search_text', { data: docIdOrData, query })
  }
}
