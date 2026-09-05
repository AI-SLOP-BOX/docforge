import { invoke } from '@tauri-apps/api/core'

export interface RenderRequest {
  pdfData: number[]
  pageIndex: number
  dpi: number
  signal?: AbortSignal
}

export interface SeparationRenderRequest extends RenderRequest {
  showC: boolean
  showM: boolean
  showY: boolean
  showK: boolean
  highlightTac: boolean
  tacLimit: number
}

export interface PDFRenderer {
  renderPageToUrl(req: RenderRequest): Promise<string>
  renderSeparationToUrl(req: SeparationRenderRequest): Promise<string>
  cancelAll(): void
}

/**
 * DefaultRenderer provides reliable rendering using the backend rendering pipeline
 * with in-memory caching and request cancellation tokens.
 */
export class DefaultRenderer implements PDFRenderer {
  private activeTokens = new Set<number>()
  private tokenSeq = 0

  async renderPageToUrl(req: RenderRequest): Promise<string> {
    const token = ++this.tokenSeq
    this.activeTokens.add(token)

    try {
      const pngBytes = await invoke<number[]>('render_page_to_png', {
        data: req.pdfData,
        page_index: req.pageIndex,
        dpi: req.dpi,
      })

      if (!this.activeTokens.has(token) || req.signal?.aborted) {
        throw new Error('Render cancelled')
      }

      const blob = new Blob([new Uint8Array(pngBytes)], { type: 'image/png' })
      return URL.createObjectURL(blob)
    } finally {
      this.activeTokens.delete(token)
    }
  }

  async renderSeparationToUrl(req: SeparationRenderRequest): Promise<string> {
    const token = ++this.tokenSeq
    this.activeTokens.add(token)

    try {
      const pngBytes = await invoke<number[]>('render_color_separation', {
        data: req.pdfData,
        page_index: req.pageIndex,
        dpi: req.dpi,
        show_c: req.showC,
        show_m: req.showM,
        show_y: req.showY,
        show_k: req.showK,
        highlight_tac: req.highlightTac,
        tac_limit: req.tacLimit,
      })

      if (!this.activeTokens.has(token) || req.signal?.aborted) {
        throw new Error('Render cancelled')
      }

      const blob = new Blob([new Uint8Array(pngBytes)], { type: 'image/png' })
      return URL.createObjectURL(blob)
    } finally {
      this.activeTokens.delete(token)
    }
  }

  cancelAll(): void {
    this.activeTokens.clear()
  }
}

export const defaultRenderer = new DefaultRenderer()
