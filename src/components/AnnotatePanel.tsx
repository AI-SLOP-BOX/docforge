import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { DocumentService } from '../services/documentService'
import { Input, NumInput, ColorInput, AccentBtn } from './UIControls'
import { HighlightIcon, RectIcon } from './Icons'

export function AnnotatePanel({
  exec,
  pdfData,
  docId,
  annotationColor,
  setAnnotationColor,
  stickyNoteText,
  setStickyNoteText,
  strokeWidth,
  setStrokeWidth,
  onActivateDraw,
  currentPage = 0,
}: {
  exec: Function
  pdfData: number[] | null
  docId?: string | null
  annotationColor: string
  setAnnotationColor: (v: string) => void
  stickyNoteText: string
  setStickyNoteText: (v: string) => void
  strokeWidth: number
  setStrokeWidth: (v: number) => void
  onActivateDraw?: (mode: 'draw-highlight' | 'draw-rect') => void
  currentPage?: number
}) {
  const [annotations, setAnnotations] = useState<Array<{
    id: string
    type: string
    contents: string
    author: string
    page: number
    status: string
    replies: Array<{ author: string; contents: string }>
  }>>([])
  const [selectedAnnot, setSelectedAnnot] = useState<string | null>(null)

  const loadAnnotations = async () => {
    try {
      const bytes = docId ? await DocumentService.getSessionBytes(docId) : pdfData
      if (!bytes) return
      const result = await invoke<Array<{
        id: string
        type: string
        contents: string
        author: string
        page: number
        status: string
        replies: Array<{ author: string; contents: string }>
      }>>('get_annotations', { data: bytes })
      setAnnotations(result || [])
    } catch (err) {
      console.error('Failed to load annotations:', err)
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      {/* Card 1: Text Marking */}
      <div className="inspector-card">
        <div className="inspector-card-header">
          <span>テキストマーク・下線</span>
          <span style={{ fontSize: 9, color: 'var(--yellow)', fontWeight: 600 }}>MARK</span>
        </div>
        <div className="inspector-card-desc">蛍光ペンまたはアンダーラインによる強調</div>
        <ColorInput value={annotationColor} onChange={setAnnotationColor} label="注釈カラー" />
        <div style={{ display: 'flex', gap: 6, marginTop: 6 }}>
          <AccentBtn onClick={() => onActivateDraw?.('draw-highlight')} style={{ flex: 1, background: 'var(--yellow)', color: '#000' }}>
            <HighlightIcon size={14} /> ドラッグ描画
          </AccentBtn>
          <AccentBtn onClick={() => exec('add_highlight', { page_index: currentPage, x: 50, y: 700, width: 200, height: 20, color: annotationColor })} style={{ flex: 1, background: 'var(--bg-2)', color: 'var(--text)', border: '1px solid var(--border)' }}>
            即時挿入
          </AccentBtn>
        </div>
        <AccentBtn onClick={() => exec('add_underline', { page_index: currentPage, x: 50, y: 700, width: 200, color: annotationColor })} style={{ marginTop: 6, background: 'var(--bg-2)', color: 'var(--text)', border: '1px solid var(--border)' }}>
          アンダーラインを追加
        </AccentBtn>
      </div>

      {/* Card 2: Sticky Note */}
      <div className="inspector-card">
        <div className="inspector-card-header">
          <span>付箋メモ (Sticky Note)</span>
          <span style={{ fontSize: 9, color: 'var(--text-muted)' }}>NOTE</span>
        </div>
        <div className="inspector-card-desc">ページ上の任意位置にコメント付箋を固定</div>
        <Input value={stickyNoteText} onChange={setStickyNoteText} placeholder="コメント・メモを入力..." />
        <AccentBtn onClick={() => exec('add_sticky_note', { page_index: currentPage, x: 500, y: 750, text: stickyNoteText, color: annotationColor })}>
          付箋を配置 (p{currentPage + 1})
        </AccentBtn>
      </div>

      {/* Card 3: Vector Shapes & Stamp */}
      <div className="inspector-card">
        <div className="inspector-card-header">
          <span>図形描画・スタンプ</span>
          <span style={{ fontSize: 9, color: 'var(--purple)', fontWeight: 600 }}>VECTOR</span>
        </div>
        <NumInput value={strokeWidth} onChange={setStrokeWidth} label="線幅 (pt)" />
        <div style={{ display: 'flex', gap: 6, marginTop: 4 }}>
          <AccentBtn onClick={() => onActivateDraw?.('draw-rect')} style={{ flex: 1, background: 'var(--purple)' }}>
            <RectIcon size={14} /> 四角描画
          </AccentBtn>
          <AccentBtn onClick={() => exec('add_rectangle', { page_index: currentPage, x: 50, y: 700, width: 200, height: 100, stroke_color: annotationColor, fill_color: '#FFFFFF00', stroke_width: strokeWidth })} style={{ flex: 1, background: 'var(--bg-2)', color: 'var(--text)', border: '1px solid var(--border)' }}>
            四角即時追加
          </AccentBtn>
        </div>
        <div style={{ display: 'flex', gap: 6, marginTop: 6 }}>
          <AccentBtn onClick={() => exec('add_stamp', { page_index: currentPage, text: 'APPROVED', x: 200, y: 400, rotation: -30, color: '#00AA00', font_size: 72 })} style={{ background: 'rgba(46, 160, 67, 0.15)', color: '#2ea043', border: '1px solid rgba(46, 160, 67, 0.4)' }}>
            APPROVED
          </AccentBtn>
          <AccentBtn onClick={() => exec('add_stamp', { page_index: currentPage, text: 'DRAFT', x: 200, y: 400, rotation: -30, color: '#FF0000', font_size: 72 })} style={{ background: 'rgba(248, 81, 73, 0.15)', color: '#f85149', border: '1px solid rgba(248, 81, 73, 0.4)' }}>
            DRAFT
          </AccentBtn>
        </div>
      </div>

      {/* Card 4: Annotation Inspector */}
      <div className="inspector-card">
        <div className="inspector-card-header">
          <span>注釈一覧</span>
          <span style={{ fontSize: 9, color: 'var(--text-muted)' }}>{annotations.length} 件</span>
        </div>
        <AccentBtn onClick={() => loadAnnotations()} style={{ background: 'var(--bg-2)', color: 'var(--text)', border: '1px solid var(--border)' }}>
          注釈を読み込み
        </AccentBtn>
        {annotations.length > 0 && (
          <div style={{ maxHeight: 200, overflowY: 'auto', marginTop: 8 }}>
            {annotations.map((a, i) => (
              <div key={i} style={{
                padding: 6, marginBottom: 4, background: 'var(--bg-0)',
                borderRadius: 'var(--radius-sm)', border: '1px solid var(--border)',
                fontSize: 11, cursor: 'pointer',
                borderColor: selectedAnnot === a.id ? 'var(--accent)' : 'var(--border)',
              }} onClick={() => setSelectedAnnot(a.id)}>
                <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                  <span style={{ color: 'var(--text)' }}>{a.type}</span>
                  <span style={{ color: 'var(--text-muted)' }}>p{a.page + 1}</span>
                </div>
                {a.contents && <div style={{ color: 'var(--text-dim)', marginTop: 2 }}>{a.contents.substring(0, 50)}</div>}
                {a.replies.length > 0 && (
                  <div style={{ color: 'var(--accent)', marginTop: 2 }}>{a.replies.length}件の返信</div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
