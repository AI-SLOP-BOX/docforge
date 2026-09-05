import React from 'react'
import {
  ChevronLeftIcon,
  ChevronRightIcon,
  MinusIcon,
  PlusIcon,
} from './Icons'
import { toolBtnStyle } from './PDFViewerToolbar'

interface PDFViewerHUDProps {
  currentPage: number
  pageCount: number
  goToPage: (page: number) => void
  zoom: number
  setZoom: (fn: (prev: number) => number) => void
  setPan: (pan: { x: number; y: number }) => void
}

export const PDFViewerHUD: React.FC<PDFViewerHUDProps> = ({
  currentPage,
  pageCount,
  goToPage,
  zoom,
  setZoom,
  setPan,
}) => {
  if (pageCount <= 0) return null

  return (
    <div
      className="floating-hud"
      style={{
        position: 'absolute',
        bottom: 20,
        left: '50%',
        transform: 'translateX(-50%)',
        display: 'flex',
        alignItems: 'center',
        gap: 6,
        padding: '5px 12px',
        zIndex: 100,
        userSelect: 'none',
      }}
    >
      <button
        onClick={() => goToPage(currentPage - 1)}
        disabled={currentPage <= 0}
        style={{
          ...toolBtnStyle,
          padding: '4px 6px',
          borderRadius: 20,
          opacity: currentPage <= 0 ? 0.3 : 1,
          display: 'flex',
          alignItems: 'center',
        }}
        title="前のページ"
      >
        <ChevronLeftIcon size={14} />
      </button>

      <div
        style={{
          fontSize: 12,
          fontWeight: 600,
          color: 'var(--text)',
          padding: '0 6px',
          minWidth: 64,
          textAlign: 'center',
        }}
      >
        {currentPage + 1} / {pageCount}
      </div>

      <button
        onClick={() => goToPage(currentPage + 1)}
        disabled={currentPage >= pageCount - 1}
        style={{
          ...toolBtnStyle,
          padding: '4px 6px',
          borderRadius: 20,
          opacity: currentPage >= pageCount - 1 ? 0.3 : 1,
          display: 'flex',
          alignItems: 'center',
        }}
        title="次のページ"
      >
        <ChevronRightIcon size={14} />
      </button>

      <div style={{ width: 1, height: 16, background: 'rgba(255,255,255,0.15)', margin: '0 4px' }} />

      <button
        onClick={() => setZoom(prev => Math.max(0.25, prev - 0.2))}
        style={{ ...toolBtnStyle, padding: '4px 6px', borderRadius: 20, display: 'flex', alignItems: 'center' }}
        title="縮小"
      >
        <MinusIcon size={12} />
      </button>

      <button
        onClick={() => {
          setZoom(() => 1.0)
          setPan({ x: 0, y: 0 })
        }}
        style={{
          background: 'rgba(255,255,255,0.06)',
          border: '1px solid rgba(255,255,255,0.1)',
          borderRadius: 12,
          padding: '2px 8px',
          fontSize: 11,
          fontWeight: 600,
          color: 'var(--text)',
          cursor: 'pointer',
          minWidth: 46,
          textAlign: 'center',
        }}
        title="ズームリセット (100%)"
      >
        {Math.round(zoom * 100)}%
      </button>

      <button
        onClick={() => setZoom(prev => Math.min(4.0, prev + 0.2))}
        style={{ ...toolBtnStyle, padding: '4px 6px', borderRadius: 20, display: 'flex', alignItems: 'center' }}
        title="拡大"
      >
        <PlusIcon size={12} />
      </button>

      <div style={{ width: 1, height: 16, background: 'rgba(255,255,255,0.15)', margin: '0 4px' }} />

      <button
        onClick={() => {
          setZoom(() => 0.85)
          setPan({ x: 0, y: 0 })
        }}
        style={{
          ...toolBtnStyle,
          fontSize: 11,
          padding: '3px 8px',
          borderRadius: 12,
          color: 'var(--text-dim)',
          background: 'rgba(255,255,255,0.04)',
        }}
        title="全体を表示"
      >
        全体
      </button>
    </div>
  )
}
