import React from 'react'
import {
  FolderOpenIcon, SaveIcon, PrinterIcon, UndoIcon, RedoIcon,
  EyeIcon, TypeIcon, HighlightIcon, RectIcon, RedactIcon, SearchIcon
} from './Icons'
import { ToolBtn } from './UIControls'
import { InteractiveMode } from './PDFViewer'
import { t } from '../utils/i18n'

interface EditorHeaderProps {
  onOpen: () => void
  onSave: () => void
  onPrint: () => void
  canSave: boolean
  undo: () => void
  redo: () => void
  canUndo: boolean
  canRedo: boolean
  interactiveMode: InteractiveMode
  setInteractiveMode: (mode: InteractiveMode) => void
  fileName: string
  pageCount: number
  currentPage: number
  onOpenCommandPalette: () => void
}

export function EditorHeader({
  onOpen,
  onSave,
  onPrint,
  canSave,
  undo,
  redo,
  canUndo,
  canRedo,
  interactiveMode,
  setInteractiveMode,
  fileName,
  pageCount,
  currentPage,
  onOpenCommandPalette,
}: EditorHeaderProps) {
  return (
    <div style={{
      display: 'flex', alignItems: 'center', gap: 6, padding: '7px 16px',
      background: 'var(--bg-1)', borderBottom: '1px solid var(--border)',
      fontSize: 12, flexShrink: 0,
    }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 2 }}>
        <ToolBtn icon={<FolderOpenIcon size={15} />} onClick={onOpen} title={t().open} shortcut="⌘O" />
        <ToolBtn icon={<SaveIcon size={15} />} onClick={onSave} disabled={!canSave} title={t().save} shortcut="⌘S" />
        <ToolBtn icon={<PrinterIcon size={15} />} onClick={onPrint} disabled={!canSave} title={t().print} shortcut="⌘P" />
      </div>
      <div style={{ width: 1, height: 18, background: 'var(--border)', margin: '0 4px' }} />
      <div style={{ display: 'flex', alignItems: 'center', gap: 2 }}>
        <ToolBtn icon={<UndoIcon size={15} />} onClick={undo} disabled={!canUndo} title={t().undo} shortcut="⌘Z" />
        <ToolBtn icon={<RedoIcon size={15} />} onClick={redo} disabled={!canRedo} title={t().redo} shortcut="⌘⇧Z" />
      </div>
      <div style={{ width: 1, height: 18, background: 'var(--border)', margin: '0 4px' }} />

      {/* Apple Segmented Control for Interaction Modes */}
      <div style={{
        display: 'flex', alignItems: 'center', gap: 2, padding: 2,
        background: 'var(--bg-0)', border: '1px solid var(--border)',
        borderRadius: 'var(--radius-sm)'
      }}>
        {([
          ['view', <EyeIcon size={13} />, t().modeView],
          ['select-text', <TypeIcon size={13} />, t().modeText],
          ['draw-highlight', <HighlightIcon size={13} />, t().modeHighlight],
          ['draw-rect', <RectIcon size={13} />, t().modeRect],
          ['draw-redact', <RedactIcon size={13} />, t().modeRedact],
        ] as const).map(([mode, icon, label]) => {
          const active = interactiveMode === mode
          return (
            <button
              key={mode}
              onClick={() => setInteractiveMode(mode as InteractiveMode)}
              style={{
                display: 'flex', alignItems: 'center', gap: 5,
                padding: '4px 9px', borderRadius: 4, fontSize: 11, fontWeight: active ? 600 : 500,
                background: active ? 'var(--bg-2)' : 'transparent',
                color: active ? 'var(--text)' : 'var(--text-muted)',
                boxShadow: active ? '0 1px 4px rgba(0,0,0,0.3)' : 'none',
              }}
            >
              <span style={{ color: active ? 'var(--accent)' : 'inherit', display: 'flex' }}>{icon}</span>
              {label}
            </button>
          )
        })}
      </div>

      <div style={{ flex: 1 }} />

      {fileName && (
        <div style={{
          display: 'flex', alignItems: 'center', gap: 8,
          padding: '3px 10px', background: 'rgba(255,255,255,0.03)',
          borderRadius: 6, border: '1px solid var(--border-subtle)'
        }}>
          <span style={{ color: 'var(--text)', fontSize: 12, fontWeight: 500 }}>{fileName}</span>
          {pageCount > 0 && (
            <span style={{ color: 'var(--text-muted)', fontSize: 11 }}>
              · {currentPage + 1} / {pageCount} ページ
            </span>
          )}
        </div>
      )}

      {/* ⌘K Command Palette Trigger Button */}
      <button
        onClick={onOpenCommandPalette}
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 6,
          padding: '4px 10px',
          background: 'var(--bg-0)',
          border: '1px solid var(--border)',
          borderRadius: 'var(--radius-sm)',
          color: 'var(--text-dim)',
          fontSize: 11,
          cursor: 'pointer',
        }}
        title="ツール・機能検索 (⌘K)"
      >
        <SearchIcon size={12} color="var(--accent)" />
        <span>ツール検索</span>
        <span
          style={{
            padding: '1px 5px',
            borderRadius: 3,
            background: 'rgba(255, 255, 255, 0.08)',
            fontSize: 10,
            fontFamily: 'monospace',
            color: 'var(--text-muted)',
          }}
        >
          ⌘K
        </span>
      </button>
    </div>
  )
}
