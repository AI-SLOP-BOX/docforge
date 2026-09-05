import { useState, useRef, useCallback, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import {
  FileIcon,
  RotateIcon, TrashIcon
} from './Icons'
import {
  SearchResult, Bookmark, FormField,
} from './PDFViewerPanels'
import { PDFViewerToolbar, PanelType } from './PDFViewerToolbar'
import { PDFViewerHUD } from './PDFViewerHUD'
import { PDFViewerOverlay } from './PDFViewerOverlay'
import { PDFViewerSidebar } from './PDFViewerSidebar'
import { PDFThumbnailStrip } from './PDFThumbnailStrip'

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

export type InteractiveMode =
  | 'view'
  | 'select-text'
  | 'draw-rect'
  | 'draw-highlight'
  | 'draw-redact'
  | 'place-form'

export interface PDFViewerProps {
  pdfData: number[]
  currentPage?: number
  onPageCountChange?: (count: number) => void
  onPageChange?: (page: number) => void
  interactiveMode?: InteractiveMode
  selectedTextBlockId?: number | null
  onSelectTextBlock?: (block: TextBlock | null) => void
  onMoveTextBlock?: (blockId: number, newX: number, newY: number) => void
  onDrawRectComplete?: (rect: { x: number; y: number; width: number; height: number; page: number }) => void
  onPdfUpdate?: (data: number[]) => void
}

export default function PDFViewer({
  pdfData,
  currentPage: externalCurrentPage,
  onPageCountChange,
  onPageChange,
  interactiveMode = 'view',
  selectedTextBlockId,
  onSelectTextBlock,
  onMoveTextBlock,
  onDrawRectComplete,
  onPdfUpdate,
}: PDFViewerProps) {
  const [pageCount, setPageCount] = useState(0)
  const [internalCurrentPage, setInternalCurrentPage] = useState(0)
  const currentPage = externalCurrentPage !== undefined ? externalCurrentPage : internalCurrentPage

  const [pageImage, setPageImage] = useState<string | null>(null)
  const [zoom, setZoom] = useState(1.0)
  const [pan, setPan] = useState({ x: 0, y: 0 })
  const [isDragging, setIsDragging] = useState(false)
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 })
  const [loading, setLoading] = useState(false)
  const [searchQuery, setSearchQuery] = useState('')
  const [searchResults, setSearchResults] = useState<SearchResult[]>([])
  const [bookmarks, setBookmarks] = useState<Bookmark[]>([])
  const [formFields, setFormFields] = useState<FormField[]>([])
  const [metadata, setMetadata] = useState<Record<string, unknown> | null>(null)
  const [activePanel, setActivePanel] = useState<'none' | 'thumbnails' | 'search' | 'bookmarks' | 'forms' | 'info' | 'separations'>('none')

  // Color Separation Preview state (CMYK + TAC)
  const [sepPlates, setSepPlates] = useState<{ c: boolean; m: boolean; y: boolean; k: boolean; tac: boolean }>({
    c: true,
    m: true,
    y: true,
    k: true,
    tac: false,
  })
  const [tacLimit, setTacLimit] = useState(300)

  // In-place direct text editing state
  const [editingBlockId, setEditingBlockId] = useState<number | null>(null)
  const [editingTextVal, setEditingTextVal] = useState('')

  // Text blocks & Page dimensions for interactive canvas overlay
  const [textBlocks, setTextBlocks] = useState<TextBlock[]>([])
  const [pageSize, setPageSize] = useState<{ width: number; height: number }>({ width: 595, height: 842 })
  const [imgRenderedSize, setImgRenderedSize] = useState<{ width: number; height: number }>({ width: 0, height: 0 })

  // Page Cache & Render Token for Instant Page Switching & Thread-safety
  const pageCache = useRef<Map<string, string>>(new Map())
  const renderSeq = useRef(0)

  // Clear cache when pdfData changes
  useEffect(() => {
    // Revoke old object URLs
    pageCache.current.forEach(url => URL.revokeObjectURL(url))
    pageCache.current.clear()
  }, [pdfData])

  // Dragging / Drawing state on overlay
  const [draggingBlockId, setDraggingBlockId] = useState<number | null>(null)
  const [blockDragOffset, setBlockDragOffset] = useState<{ x: number; y: number }>({ x: 0, y: 0 })
  const [tempBlockPos, setTempBlockPos] = useState<{ id: number; x: number; y: number } | null>(null)
  const [drawBox, setDrawBox] = useState<{ startX: number; startY: number; currentX: number; currentY: number } | null>(null)

  const containerRef = useRef<HTMLDivElement>(null)
  const imgRef = useRef<HTMLImageElement>(null)

  // Load PDF info
  useEffect(() => {
    if (!pdfData || pdfData.length === 0) return

    const loadInfo = async () => {
      try {
        const count = await invoke<number>('get_page_count', { data: pdfData })
        setPageCount(count)
        onPageCountChange?.(count)

        const meta = await invoke<Record<string, unknown>>('get_pdf_metadata', { data: pdfData })
        setMetadata(meta)

        const bms = await invoke<Bookmark[]>('get_bookmarks', { data: pdfData })
        setBookmarks(bms)

        const fields = await invoke<FormField[]>('get_form_fields', { data: pdfData })
        setFormFields(fields)
      } catch (err) {
        console.error('Failed to load PDF info:', err)
      }
    }

    loadInfo()
  }, [pdfData, onPageCountChange])

  // Load Page Dimensions and Text Blocks for current page
  useEffect(() => {
    if (!pdfData || pdfData.length === 0 || currentPage >= pageCount) return

    const loadPageData = async () => {
      try {
        const dims = await invoke<{ width: number; height: number }>('get_page_dimensions', {
          data: pdfData,
          page_index: currentPage,
        })
        if (dims && dims.width && dims.height) {
          setPageSize(dims)
        }
      } catch {
        // Default A4
        setPageSize({ width: 595, height: 842 })
      }

      try {
        const blocks = await invoke<TextBlock[]>('get_text_blocks', {
          data: pdfData,
          page_index: currentPage,
        })
        setTextBlocks(blocks || [])
      } catch {
        setTextBlocks([])
      }
    }

    loadPageData()
  }, [pdfData, currentPage, pageCount])

  // Render current page to PNG with Instant Cache
  // Dynamic DPI based on zoom factor (150 base, up to 300 for CAD precision)
  const targetDpi = Math.min(300, Math.max(120, Math.round(150 * Math.min(zoom, 2.0))))

  useEffect(() => {
    if (!pdfData || pdfData.length === 0 || currentPage >= pageCount) return

    const isCustomSep = !sepPlates.c || !sepPlates.m || !sepPlates.y || !sepPlates.k || sepPlates.tac
    const cacheKey = `${currentPage}@${targetDpi}_c${sepPlates.c ? 1 : 0}m${sepPlates.m ? 1 : 0}y${sepPlates.y ? 1 : 0}k${sepPlates.k ? 1 : 0}tac${sepPlates.tac ? 1 : 0}_${tacLimit}`
    const cachedUrl = pageCache.current.get(cacheKey)
    if (cachedUrl) {
      setPageImage(cachedUrl)
      setLoading(false)
      return
    }

    const currentToken = ++renderSeq.current
    setLoading(true)

    const timer = setTimeout(async () => {
      try {
        let pngData: number[]
        if (isCustomSep) {
          pngData = await invoke<number[]>('render_color_separation', {
            data: pdfData,
            page_index: currentPage,
            dpi: targetDpi,
            show_c: sepPlates.c,
            show_m: sepPlates.m,
            show_y: sepPlates.y,
            show_k: sepPlates.k,
            highlight_tac: sepPlates.tac,
            tac_limit: tacLimit,
          })
        } else {
          pngData = await invoke<number[]>('render_page_to_png', {
            data: pdfData,
            page_index: currentPage,
            dpi: targetDpi,
          })
        }

        if (currentToken !== renderSeq.current) return

        const blob = new Blob([new Uint8Array(pngData)], { type: 'image/png' })
        const url = URL.createObjectURL(blob)

        // Cache up to 16 rendered states in memory
        if (pageCache.current.size >= 16) {
          const firstKey = pageCache.current.keys().next().value
          if (firstKey !== undefined) {
            const oldUrl = pageCache.current.get(firstKey)
            if (oldUrl) URL.revokeObjectURL(oldUrl)
            pageCache.current.delete(firstKey)
          }
        }
        pageCache.current.set(cacheKey, url)

        setPageImage(url)
      } catch (err) {
        if (currentToken === renderSeq.current) {
          console.error('Failed to render page:', err)
          setPageImage(null)
        }
      } finally {
        if (currentToken === renderSeq.current) {
          setLoading(false)
        }
      }
    }, zoom !== 1.0 ? 80 : 0)

    return () => clearTimeout(timer)
  }, [pdfData, currentPage, pageCount, targetDpi, sepPlates, tacLimit])

  // Update image rendered dimensions
  const handleImageLoad = () => {
    if (imgRef.current) {
      setImgRenderedSize({
        width: imgRef.current.clientWidth,
        height: imgRef.current.clientHeight,
      })
    }
  }

  // Handle zoom
  const handleWheel = useCallback((e: React.WheelEvent) => {
    e.preventDefault()
    const delta = e.deltaY > 0 ? -0.1 : 0.1
    setZoom(prev => Math.max(0.25, Math.min(4.0, prev + delta)))
  }, [])

  // Handle pan when in view mode or holding middle-click / Alt
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (e.button === 1 || (e.button === 0 && e.altKey) || (interactiveMode === 'view' && e.button === 0 && e.target === containerRef.current)) {
      setIsDragging(true)
      setDragStart({ x: e.clientX - pan.x, y: e.clientY - pan.y })
    }
  }, [pan, interactiveMode])

  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    if (isDragging) {
      setPan({
        x: e.clientX - dragStart.x,
        y: e.clientY - dragStart.y,
      })
    }
  }, [isDragging, dragStart])

  const handleMouseUp = useCallback(() => {
    setIsDragging(false)
  }, [])

  // Page navigation
  const goToPage = useCallback((page: number) => {
    const newPage = Math.max(0, Math.min(pageCount - 1, page))
    setInternalCurrentPage(newPage)
    onPageChange?.(newPage)
    setPan({ x: 0, y: 0 })
  }, [pageCount, onPageChange])

  // Search
  const handleSearch = useCallback(async () => {
    if (!searchQuery.trim()) {
      setSearchResults([])
      return
    }

    try {
      const results = await invoke<SearchResult[]>('search_text', {
        data: pdfData,
        query: searchQuery,
      })
      setSearchResults(results)
    } catch (err) {
      console.error('Search failed:', err)
    }
  }, [pdfData, searchQuery])

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey) {
        switch (e.key) {
          case '=':
          case '+':
            e.preventDefault()
            setZoom(prev => Math.min(4.0, prev + 0.25))
            break
          case '-':
            e.preventDefault()
            setZoom(prev => Math.max(0.25, prev - 0.25))
            break
          case '0':
            e.preventDefault()
            setZoom(1.0)
            setPan({ x: 0, y: 0 })
            break
          case 'ArrowLeft':
            e.preventDefault()
            goToPage(currentPage - 1)
            break
          case 'ArrowRight':
            e.preventDefault()
            goToPage(currentPage + 1)
            break
        }
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [currentPage, goToPage])

  // --- Interactive Canvas Calculations (PDF Points <-> DOM Pixels) ---
  const scaleX = imgRenderedSize.width > 0 ? imgRenderedSize.width / pageSize.width : 1
  const scaleY = imgRenderedSize.height > 0 ? imgRenderedSize.height / pageSize.height : 1

  const pdfToDom = useCallback((pdfX: number, pdfY: number, pdfW: number, pdfH: number) => {
    const domX = pdfX * scaleX
    // PDF Y is bottom-up; DOM Y is top-down
    const domY = (pageSize.height - (pdfY + pdfH)) * scaleY
    const domW = Math.max(pdfW * scaleX, 10)
    const domH = Math.max(pdfH * scaleY, 12)
    return { left: domX, top: domY, width: domW, height: domH }
  }, [scaleX, scaleY, pageSize.height])

  const domToPdf = useCallback((domX: number, domY: number, domW: number, domH: number) => {
    const pdfX = scaleX > 0 ? domX / scaleX : 0
    const pdfW = scaleX > 0 ? domW / scaleX : 0
    const pdfH = scaleY > 0 ? domH / scaleY : 0
    const pdfY = scaleY > 0 ? pageSize.height - ((domY + domH) / scaleY) : 0
    return {
      x: Math.round(pdfX),
      y: Math.round(pdfY),
      width: Math.round(pdfW),
      height: Math.round(pdfH),
    }
  }, [scaleX, scaleY, pageSize.height])

  // Drawing mouse handlers on overlay
  const handleOverlayMouseDown = (e: React.MouseEvent<HTMLDivElement>) => {
    if (e.button !== 0 || e.altKey) return

    const overlayRect = e.currentTarget.getBoundingClientRect()
    const clickX = (e.clientX - overlayRect.left) / zoom
    const clickY = (e.clientY - overlayRect.top) / zoom

    if (interactiveMode === 'select-text') {
      // Deselect if clicking on empty background
      onSelectTextBlock?.(null)
    } else if (
      interactiveMode === 'draw-rect' ||
      interactiveMode === 'draw-highlight' ||
      interactiveMode === 'draw-redact' ||
      interactiveMode === 'place-form'
    ) {
      setDrawBox({
        startX: clickX,
        startY: clickY,
        currentX: clickX,
        currentY: clickY,
      })
    }
  }

  const handleOverlayMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    const overlayRect = e.currentTarget.getBoundingClientRect()
    const currentX = (e.clientX - overlayRect.left) / zoom
    const currentY = (e.clientY - overlayRect.top) / zoom

    // Handle drag-and-drop moving of a selected text block
    if (draggingBlockId !== null && tempBlockPos) {
      const newDomX = currentX - blockDragOffset.x
      const newDomY = currentY - blockDragOffset.y
      const block = textBlocks.find(b => b.id === draggingBlockId)
      if (block) {
        const domW = block.width * scaleX
        const domH = block.height * scaleY
        const pdfCoords = domToPdf(newDomX, newDomY, domW, domH)
        setTempBlockPos({ id: draggingBlockId, x: pdfCoords.x, y: pdfCoords.y })
      }
      return
    }

    // Handle box drawing
    if (drawBox) {
      setDrawBox(prev => prev ? { ...prev, currentX, currentY } : null)
    }
  }

  const handleOverlayMouseUp = () => {
    if (draggingBlockId !== null && tempBlockPos) {
      onMoveTextBlock?.(tempBlockPos.id, tempBlockPos.x, tempBlockPos.y)
      setDraggingBlockId(null)
      setTempBlockPos(null)
      return
    }

    if (drawBox) {
      const minX = Math.min(drawBox.startX, drawBox.currentX)
      const minY = Math.min(drawBox.startY, drawBox.currentY)
      const w = Math.abs(drawBox.currentX - drawBox.startX)
      const h = Math.abs(drawBox.currentY - drawBox.startY)

      if (w > 5 && h > 5) {
        const pdfRect = domToPdf(minX, minY, w, h)
        onDrawRectComplete?.({
          ...pdfRect,
          page: currentPage,
        })
      }
      setDrawBox(null)
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', background: '#1a1a2e' }}>
      <PDFViewerToolbar
        activePanel={activePanel}
        setActivePanel={setActivePanel}
        currentPage={currentPage}
        pageCount={pageCount}
        goToPage={goToPage}
        zoom={zoom}
        setZoom={setZoom}
        onResetZoom={() => { setZoom(1.0); setPan({ x: 0, y: 0 }) }}
        interactiveMode={interactiveMode}
        loading={loading}
      />

      <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
        {/* Side Panel */}
        <PDFViewerSidebar
          activePanel={activePanel}
          sepPlates={sepPlates}
          setSepPlates={setSepPlates}
          tacLimit={tacLimit}
          setTacLimit={setTacLimit}
          pdfData={pdfData}
          pageCount={pageCount}
          currentPage={currentPage}
          goToPage={goToPage}
          onPdfUpdate={onPdfUpdate}
          searchQuery={searchQuery}
          setSearchQuery={setSearchQuery}
          searchResults={searchResults}
          handleSearch={handleSearch}
          bookmarks={bookmarks}
          formFields={formFields}
          metadata={metadata}
        />

        {/* PDF Canvas with Interactive Overlay Layer */}
        <div
          ref={containerRef}
          onWheel={handleWheel}
          onMouseDown={handleMouseDown}
          onMouseMove={handleMouseMove}
          onMouseUp={handleMouseUp}
          onMouseLeave={handleMouseUp}
          style={{
            flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center',
            overflow: 'hidden', cursor: isDragging ? 'grabbing' : 'default',
            background: '#141420', position: 'relative',
          }}
        >
          {pageImage ? (
            <div style={{
              transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
              transformOrigin: 'center center',
              transition: isDragging ? 'none' : 'transform 0.05s ease',
              position: 'relative',
              display: 'inline-block',
              boxShadow: '0 12px 48px rgba(0,0,0,0.6)',
            }}>
              {/* Rendered PDF Page Image */}
              <img
                ref={imgRef}
                src={pageImage}
                alt={`Page ${currentPage + 1}`}
                onLoad={handleImageLoad}
                style={{
                  display: 'block',
                  maxWidth: '85vw',
                  maxHeight: '80vh',
                  userSelect: 'none',
                  borderRadius: 2,
                }}
                draggable={false}
              />

              {/* Interactive Vector / Bounding Box Overlay */}
              {imgRenderedSize.width > 0 && imgRenderedSize.height > 0 && (
                <PDFViewerOverlay
                  interactiveMode={interactiveMode}
                  selectedTextBlockId={selectedTextBlockId}
                  textBlocks={textBlocks}
                  tempBlockPos={tempBlockPos}
                  pdfToDom={pdfToDom}
                  onSelectTextBlock={onSelectTextBlock}
                  setDraggingBlockId={setDraggingBlockId}
                  setBlockDragOffset={setBlockDragOffset}
                  setTempBlockPos={setTempBlockPos}
                  zoom={zoom}
                  editingBlockId={editingBlockId}
                  setEditingBlockId={setEditingBlockId}
                  editingTextVal={editingTextVal}
                  setEditingTextVal={setEditingTextVal}
                  pdfData={pdfData}
                  currentPage={currentPage}
                  onPdfUpdate={onPdfUpdate}
                  imgRenderedSize={imgRenderedSize}
                  pageSize={pageSize}
                  drawBox={drawBox}
                  handleOverlayMouseDown={handleOverlayMouseDown}
                  handleOverlayMouseMove={handleOverlayMouseMove}
                  handleOverlayMouseUp={handleOverlayMouseUp}
                />
              )}
            </div>
          ) : (
            <div style={{ color: '#666', textAlign: 'center' }}>
              <div style={{ marginBottom: 16, opacity: 0.25, display: 'flex', justifyContent: 'center' }}>
                <FileIcon size={48} color="var(--text-dim)" />
              </div>
              <p>ページを読み込み中...</p>
            </div>
          )}

          {/* Apple Floating Glass HUD for Zoom & Navigation */}
          <PDFViewerHUD
            currentPage={currentPage}
            pageCount={pageCount}
            goToPage={goToPage}
            zoom={zoom}
            setZoom={setZoom}
            setPan={setPan}
          />
        </div>
      </div>

      {/* Thumbnail strip */}
      <PDFThumbnailStrip
        pageCount={pageCount}
        currentPage={currentPage}
        goToPage={goToPage}
      />
    </div>
  )
}


