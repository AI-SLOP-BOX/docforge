import React from 'react'
import { invoke } from '@tauri-apps/api/core'
import { TextBlock, InteractiveMode } from './PDFViewer'

interface PDFViewerOverlayProps {
  interactiveMode: InteractiveMode
  selectedTextBlockId?: number | null
  textBlocks: TextBlock[]
  tempBlockPos: { id: number; x: number; y: number } | null
  pdfToDom: (x: number, y: number, w: number, h: number) => { left: number; top: number; width: number; height: number }
  onSelectTextBlock?: (block: TextBlock | null) => void
  setDraggingBlockId: (id: number | null) => void
  setBlockDragOffset: (offset: { x: number; y: number }) => void
  setTempBlockPos: (pos: { id: number; x: number; y: number } | null) => void
  zoom: number
  editingBlockId: number | null
  setEditingBlockId: (id: number | null) => void
  editingTextVal: string
  setEditingTextVal: (val: string) => void
  pdfData: number[]
  currentPage: number
  onPdfUpdate?: (data: number[]) => void
  imgRenderedSize: { width: number; height: number }
  pageSize: { width: number; height: number }
  drawBox: { startX: number; startY: number; currentX: number; currentY: number } | null
  handleOverlayMouseDown: (e: React.MouseEvent<HTMLDivElement>) => void
  handleOverlayMouseMove: (e: React.MouseEvent<HTMLDivElement>) => void
  handleOverlayMouseUp: () => void
}

export const PDFViewerOverlay: React.FC<PDFViewerOverlayProps> = ({
  interactiveMode,
  selectedTextBlockId,
  textBlocks,
  tempBlockPos,
  pdfToDom,
  onSelectTextBlock,
  setDraggingBlockId,
  setBlockDragOffset,
  setTempBlockPos,
  zoom,
  editingBlockId,
  setEditingBlockId,
  editingTextVal,
  setEditingTextVal,
  pdfData,
  currentPage,
  onPdfUpdate,
  imgRenderedSize,
  pageSize,
  drawBox,
  handleOverlayMouseDown,
  handleOverlayMouseMove,
  handleOverlayMouseUp,
}) => {
  return (
    <div
      onMouseDown={handleOverlayMouseDown}
      onMouseMove={handleOverlayMouseMove}
      onMouseUp={handleOverlayMouseUp}
      style={{
        position: 'absolute',
        top: 0,
        left: 0,
        width: imgRenderedSize.width,
        height: imgRenderedSize.height,
        cursor: interactiveMode === 'view'
          ? 'default'
          : interactiveMode === 'select-text'
            ? 'text'
            : 'crosshair',
        pointerEvents: interactiveMode === 'view' ? 'none' : 'auto',
      }}
    >
      {/* Interactive Text Blocks (when in select-text or always subtle in edit) */}
      {(interactiveMode === 'select-text' || selectedTextBlockId !== undefined) &&
        textBlocks.map(block => {
          const isSelected = selectedTextBlockId === block.id
          const isBeingMoved = tempBlockPos && tempBlockPos.id === block.id
          const blockX = isBeingMoved ? tempBlockPos.x : block.x
          const blockY = isBeingMoved ? tempBlockPos.y : block.y

          const dom = pdfToDom(blockX, blockY, block.width, block.height)

          return (
            <div
              key={block.id}
              title={`${block.text} (${Math.round(block.x)}, ${Math.round(block.y)})`}
              onClick={(e) => {
                e.stopPropagation()
                onSelectTextBlock?.(block)
              }}
              onMouseDown={(e) => {
                if (e.button === 0 && !e.altKey && interactiveMode === 'select-text') {
                  e.stopPropagation()
                  onSelectTextBlock?.(block)
                  setDraggingBlockId(block.id)
                  const overlayRect = e.currentTarget.parentElement?.getBoundingClientRect()
                  if (overlayRect) {
                    const clickX = (e.clientX - overlayRect.left) / zoom
                    const clickY = (e.clientY - overlayRect.top) / zoom
                    setBlockDragOffset({
                      x: clickX - dom.left,
                      y: clickY - dom.top,
                    })
                    setTempBlockPos({ id: block.id, x: block.x, y: block.y })
                  }
                }
              }}
              onDoubleClick={(e) => {
                e.stopPropagation()
                setEditingBlockId(block.id)
                setEditingTextVal(block.text)
              }}
              style={{
                position: 'absolute',
                left: dom.left,
                top: dom.top,
                width: dom.width,
                height: dom.height,
                border: isSelected
                  ? '2px solid var(--accent)'
                  : '1px dashed rgba(0, 200, 255, 0.4)',
                background: isSelected
                  ? 'rgba(0, 200, 255, 0.25)'
                  : 'rgba(0, 200, 255, 0.05)',
                borderRadius: 2,
                boxSizing: 'border-box',
                cursor: editingBlockId === block.id ? 'text' : 'move',
                transition: isBeingMoved ? 'none' : 'background 0.15s, border 0.15s',
                zIndex: isSelected || editingBlockId === block.id ? 10 : 1,
              }}
            >
              {editingBlockId === block.id ? (
                <input
                  autoFocus
                  value={editingTextVal}
                  onChange={e => setEditingTextVal(e.target.value)}
                  onKeyDown={async (e) => {
                    if (e.key === 'Enter') {
                      e.preventDefault()
                      e.stopPropagation()
                      if (pdfData && editingTextVal !== block.text) {
                        try {
                          const updated = await invoke<number[]>('edit_text_block', {
                            data: pdfData,
                            page_index: currentPage,
                            block_id: block.id,
                            new_text: editingTextVal,
                          })
                          onPdfUpdate?.(updated)
                        } catch (err) {
                          console.error('Failed to update text in-place:', err)
                        }
                      }
                      setEditingBlockId(null)
                    } else if (e.key === 'Escape') {
                      e.stopPropagation()
                      setEditingBlockId(null)
                    }
                  }}
                  onBlur={async () => {
                    if (pdfData && editingTextVal !== block.text) {
                      try {
                        const updated = await invoke<number[]>('edit_text_block', {
                          data: pdfData,
                          page_index: currentPage,
                          block_id: block.id,
                          new_text: editingTextVal,
                        })
                        onPdfUpdate?.(updated)
                      } catch (err) {
                        console.error('Failed to update text in-place:', err)
                      }
                    }
                    setEditingBlockId(null)
                  }}
                  onClick={e => e.stopPropagation()}
                  onMouseDown={e => e.stopPropagation()}
                  style={{
                    width: '100%',
                    height: '100%',
                    background: 'rgba(0, 0, 0, 0.85)',
                    color: '#00ff88',
                    border: '1px solid var(--accent)',
                    outline: 'none',
                    fontSize: Math.max(10, Math.round(block.font_size * (imgRenderedSize.height / pageSize.height))),
                    fontFamily: block.font_name.toLowerCase().includes('sans') ? 'sans-serif' : 'serif',
                    padding: '0 2px',
                    boxSizing: 'border-box',
                    borderRadius: 2,
                  }}
                />
              ) : (
                isSelected && (
                  <div style={{
                    position: 'absolute',
                    top: -18,
                    left: 0,
                    background: 'var(--accent)',
                    color: 'var(--bg-0)',
                    fontSize: 9,
                    fontWeight: 700,
                    padding: '1px 4px',
                    borderRadius: 2,
                    whiteSpace: 'nowrap',
                  }}>
                    {block.font_name || 'Text'} ({Math.round(blockX)}, {Math.round(blockY)}) [Wクリックで編集]
                  </div>
                )
              )}
            </div>
          )
        })}

      {/* Active Drag-To-Draw Box Preview */}
      {drawBox && (
        <div
          style={{
            position: 'absolute',
            left: Math.min(drawBox.startX, drawBox.currentX),
            top: Math.min(drawBox.startY, drawBox.currentY),
            width: Math.abs(drawBox.currentX - drawBox.startX),
            height: Math.abs(drawBox.currentY - drawBox.startY),
            border: interactiveMode === 'draw-redact'
              ? '2px solid #ff3344'
              : interactiveMode === 'draw-highlight'
                ? '2px solid #ffcc00'
                : interactiveMode === 'place-form'
                  ? '2px dashed #00d2ff'
                  : '2px solid var(--accent)',
            background: interactiveMode === 'draw-redact'
              ? 'rgba(0, 0, 0, 0.75)'
              : interactiveMode === 'draw-highlight'
                ? 'rgba(255, 235, 59, 0.4)'
                : interactiveMode === 'place-form'
                  ? 'rgba(0, 210, 255, 0.2)'
                  : 'rgba(0, 200, 255, 0.2)',
            pointerEvents: 'none',
            zIndex: 20,
          }}
        >
          <span style={{
            position: 'absolute',
            bottom: 2,
            right: 4,
            fontSize: 10,
            color: '#fff',
            textShadow: '0 1px 2px #000',
            fontWeight: 600,
          }}>
            {Math.round(Math.abs(drawBox.currentX - drawBox.startX))} × {Math.round(Math.abs(drawBox.currentY - drawBox.startY))}
          </span>
        </div>
      )}
    </div>
  )
}
