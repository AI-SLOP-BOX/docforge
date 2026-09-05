import React from 'react'
import {
  ChevronLeftIcon,
  ChevronRightIcon,
  MinusIcon,
  PlusIcon,
  ThumbnailsIcon,
  EyeIcon,
  TypeIcon,
  RectIcon,
  HighlightIcon,
  RedactIcon,
  FormIcon,
  SearchIcon,
  BookmarkIcon,
  InfoIcon,
} from './Icons'

export type PanelType = 'none' | 'thumbnails' | 'search' | 'bookmarks' | 'forms' | 'info' | 'separations'
export type InteractiveMode = 'view' | 'select-text' | 'draw-rect' | 'draw-highlight' | 'draw-redact' | 'place-form'

interface PDFViewerToolbarProps {
  activePanel: PanelType
  setActivePanel: (panel: PanelType) => void
  currentPage: number
  pageCount: number
  goToPage: (page: number) => void
  zoom: number
  setZoom: (fn: (prev: number) => number) => void
  onResetZoom: () => void
  interactiveMode: InteractiveMode
  loading: boolean
}

export const toolBtnStyle: React.CSSProperties = {
  background: 'transparent',
  border: '1px solid transparent',
  color: 'var(--text-muted)',
  padding: '4px 8px',
  borderRadius: 4,
  cursor: 'pointer',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  fontSize: 12,
  transition: 'all 0.15s ease',
}

export const PDFViewerToolbar: React.FC<PDFViewerToolbarProps> = ({
  activePanel,
  setActivePanel,
  currentPage,
  pageCount,
  goToPage,
  zoom,
  setZoom,
  onResetZoom,
  interactiveMode,
  loading,
}) => {
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        padding: '6px 12px',
        background: 'var(--bg-1)',
        borderBottom: '1px solid var(--border)',
        fontSize: 12,
        flexShrink: 0,
      }}
    >
      <button
        onClick={() => setActivePanel(activePanel === 'thumbnails' ? 'none' : 'thumbnails')}
        style={{
          ...toolBtnStyle,
          background: activePanel === 'thumbnails' ? 'var(--accent)' : undefined,
          color: activePanel === 'thumbnails' ? 'var(--bg-0)' : undefined,
        }}
        title="ページサムネイル一覧"
      >
        <ThumbnailsIcon size={14} />
      </button>

      <div style={{ width: 1, height: 20, background: 'var(--border)', margin: '0 2px' }} />

      <button
        onClick={() => goToPage(currentPage - 1)}
        disabled={currentPage <= 0}
        style={toolBtnStyle}
        title="前のページ"
      >
        <ChevronLeftIcon size={14} />
      </button>
      <span style={{ color: 'var(--text)', minWidth: 80, textAlign: 'center', fontWeight: 500 }}>
        {currentPage + 1} / {pageCount}
      </span>
      <button
        onClick={() => goToPage(currentPage + 1)}
        disabled={currentPage >= pageCount - 1}
        style={toolBtnStyle}
        title="次のページ"
      >
        <ChevronRightIcon size={14} />
      </button>

      <div style={{ width: 1, height: 20, background: 'var(--border)', margin: '0 4px' }} />

      <button
        onClick={() => setZoom(prev => Math.max(0.25, prev - 0.25))}
        style={toolBtnStyle}
        title="縮小"
      >
        <MinusIcon size={13} />
      </button>
      <span style={{ color: 'var(--text)', minWidth: 50, textAlign: 'center' }}>
        {Math.round(zoom * 100)}%
      </span>
      <button
        onClick={() => setZoom(prev => Math.min(4.0, prev + 0.25))}
        style={toolBtnStyle}
        title="拡大"
      >
        <PlusIcon size={13} />
      </button>
      <button
        onClick={onResetZoom}
        style={toolBtnStyle}
        title="ズームリセット"
      >
        リセット
      </button>

      <div style={{ width: 1, height: 20, background: 'var(--border)', margin: '0 4px' }} />

      {/* Current Mode Badge */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 6,
          padding: '3px 10px',
          borderRadius: 12,
          fontSize: 11,
          background: interactiveMode === 'view' ? 'var(--bg-2)' : 'var(--accent)',
          color: interactiveMode === 'view' ? 'var(--text-muted)' : 'var(--bg-0)',
          fontWeight: 600,
        }}
      >
        {interactiveMode === 'view' && <><EyeIcon size={13} /> 閲覧モード</>}
        {interactiveMode === 'select-text' && <><TypeIcon size={13} /> テキスト選択・移動</>}
        {interactiveMode === 'draw-rect' && <><RectIcon size={13} /> 四角描画モード</>}
        {interactiveMode === 'draw-highlight' && <><HighlightIcon size={13} /> ハイライト描画</>}
        {interactiveMode === 'draw-redact' && <><RedactIcon size={13} /> 黒塗り描画モード</>}
        {interactiveMode === 'place-form' && <><FormIcon size={13} /> フォーム配置モード</>}
      </div>

      <div style={{ flex: 1 }} />

      <button
        onClick={() => setActivePanel(activePanel === 'separations' ? 'none' : 'separations')}
        style={{
          ...toolBtnStyle,
          background: activePanel === 'separations' ? 'var(--accent)' : undefined,
          color: activePanel === 'separations' ? 'var(--bg-0)' : undefined,
          fontWeight: 600,
          fontSize: 10,
          padding: '2px 8px',
        }}
        title="分版プレビュー (Separations Preview)"
      >
        CMYK分版
      </button>
      <button
        onClick={() => setActivePanel(activePanel === 'search' ? 'none' : 'search')}
        style={{
          ...toolBtnStyle,
          background: activePanel === 'search' ? 'var(--accent)' : undefined,
          color: activePanel === 'search' ? 'var(--bg-0)' : undefined,
        }}
        title="検索"
      >
        <SearchIcon size={14} />
      </button>
      <button
        onClick={() => setActivePanel(activePanel === 'bookmarks' ? 'none' : 'bookmarks')}
        style={{
          ...toolBtnStyle,
          background: activePanel === 'bookmarks' ? 'var(--accent)' : undefined,
          color: activePanel === 'bookmarks' ? 'var(--bg-0)' : undefined,
        }}
        title="しおり"
      >
        <BookmarkIcon size={14} />
      </button>
      <button
        onClick={() => setActivePanel(activePanel === 'forms' ? 'none' : 'forms')}
        style={{
          ...toolBtnStyle,
          background: activePanel === 'forms' ? 'var(--accent)' : undefined,
          color: activePanel === 'forms' ? 'var(--bg-0)' : undefined,
        }}
        title="フォーム一覧"
      >
        <FormIcon size={14} />
      </button>
      <button
        onClick={() => setActivePanel(activePanel === 'info' ? 'none' : 'info')}
        style={{
          ...toolBtnStyle,
          background: activePanel === 'info' ? 'var(--accent)' : undefined,
          color: activePanel === 'info' ? 'var(--bg-0)' : undefined,
        }}
        title="PDF詳細情報"
      >
        <InfoIcon size={14} />
      </button>

      {loading && (
        <span style={{ color: 'var(--accent)', fontSize: 11, marginLeft: 8 }}>読み込み中...</span>
      )}
    </div>
  )
}
