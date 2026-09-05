import React, { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { RotateIcon, TrashIcon } from './Icons'

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

export function SearchPanel({
  query,
  setQuery,
  results,
  onSearch,
  onGoToPage,
}: {
  query: string
  setQuery: (v: string) => void
  results: SearchResult[]
  onSearch: () => void
  onGoToPage: (page: number) => void
}) {
  return (
    <div style={{ padding: 12 }}>
      <div style={{ fontSize: 11, fontWeight: 600, color: 'var(--text-muted)', textTransform: 'uppercase', marginBottom: 8 }}>
        テキスト検索
      </div>
      <div style={{ display: 'flex', gap: 4 }}>
        <input
          value={query}
          onChange={e => setQuery(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && onSearch()}
          placeholder="検索..."
          style={{ flex: 1, padding: '6px 8px', background: 'var(--bg-0)', border: '1px solid var(--border)', borderRadius: 'var(--radius-sm)', color: 'var(--text)', fontSize: 12 }}
        />
        <button onClick={onSearch} style={{ padding: '6px 12px', background: 'var(--accent)', color: 'var(--bg-0)', border: 'none', borderRadius: 'var(--radius-sm)', fontSize: 12, fontWeight: 600 }}>
          検索
        </button>
      </div>
      {results.length > 0 && (
        <div style={{ marginTop: 12, fontSize: 12, color: 'var(--text-dim)' }}>
          {results.length}件の結果
          <div style={{ marginTop: 8, maxHeight: 300, overflowY: 'auto' }}>
            {results.map((r, i) => (
              <div
                key={i}
                onClick={() => onGoToPage(r.page)}
                style={{
                  padding: '8px', marginBottom: 4, background: 'var(--bg-0)',
                  borderRadius: 'var(--radius-sm)', cursor: 'pointer',
                  border: '1px solid var(--border)',
                }}
              >
                <div style={{ fontSize: 10, color: 'var(--accent)' }}>ページ {r.page + 1}</div>
                <div style={{ fontSize: 11, color: 'var(--text)', marginTop: 4, lineHeight: 1.4 }}>
                  {r.text.substring(0, 100)}...
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}

export function BookmarksPanel({
  bookmarks,
  onGoToPage,
}: {
  bookmarks: Bookmark[]
  onGoToPage: (page: number) => void
}) {
  return (
    <div style={{ padding: 12 }}>
      <div style={{ fontSize: 11, fontWeight: 600, color: 'var(--text-muted)', textTransform: 'uppercase', marginBottom: 8 }}>
        ブックマーク ({bookmarks.length})
      </div>
      {bookmarks.length === 0 ? (
        <div style={{ fontSize: 12, color: 'var(--text-muted)', textAlign: 'center', padding: 20 }}>
          ブックマークなし
        </div>
      ) : (
        <div style={{ maxHeight: 400, overflowY: 'auto' }}>
          {bookmarks.map((bm, i) => (
            <div
              key={i}
              onClick={() => onGoToPage(bm.page)}
              style={{
                padding: '8px', marginBottom: 4, background: 'var(--bg-0)',
                borderRadius: 'var(--radius-sm)', cursor: 'pointer',
                border: '1px solid var(--border)',
              }}
            >
              <div style={{ fontSize: 12, color: 'var(--text)' }}>{bm.title}</div>
              <div style={{ fontSize: 10, color: 'var(--text-muted)' }}>ページ {bm.page + 1}</div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

export function FormsPanel({
  fields,
  pdfData,
}: {
  fields: FormField[]
  pdfData: number[]
}) {
  const [values, setValues] = useState<Record<string, string>>({})

  const handleSave = async (fieldName: string) => {
    try {
      await invoke('set_form_field', {
        data: pdfData,
        field_name: fieldName,
        value: values[fieldName] || '',
      })
    } catch (err) {
      console.error('Failed to set field:', err)
    }
  }

  return (
    <div style={{ padding: 12 }}>
      <div style={{ fontSize: 11, fontWeight: 600, color: 'var(--text-muted)', textTransform: 'uppercase', marginBottom: 8 }}>
        フォームフィールド ({fields.length})
      </div>
      {fields.length === 0 ? (
        <div style={{ fontSize: 12, color: 'var(--text-muted)', textAlign: 'center', padding: 20 }}>
          フォームなし
        </div>
      ) : (
        <div style={{ maxHeight: 400, overflowY: 'auto' }}>
          {fields.map((field, i) => (
            <div key={i} style={{ marginBottom: 8 }}>
              <label style={{ fontSize: 11, color: 'var(--text-dim)', display: 'block', marginBottom: 2 }}>
                {field.name} ({field.type})
              </label>
              <div style={{ display: 'flex', gap: 4 }}>
                <input
                  value={values[field.name] ?? field.value}
                  onChange={e => setValues(prev => ({ ...prev, [field.name]: e.target.value }))}
                  style={{
                    flex: 1, padding: '5px 6px', background: 'var(--bg-0)',
                    border: '1px solid var(--border)', borderRadius: 'var(--radius-sm)',
                    color: 'var(--text)', fontSize: 12,
                  }}
                />
                <button
                  onClick={() => handleSave(field.name)}
                  style={{
                    padding: '5px 8px', background: 'var(--accent)', color: 'var(--bg-0)',
                    border: 'none', borderRadius: 'var(--radius-sm)', fontSize: 11,
                  }}
                >保存</button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

export function ThumbnailsPanel({
  pdfData,
  pageCount,
  currentPage,
  onGoToPage,
  onPdfUpdate,
}: {
  pdfData: number[]
  pageCount: number
  currentPage: number
  onGoToPage: (page: number) => void
  onPdfUpdate?: (data: number[]) => void
}) {
  const [thumbUrls, setThumbUrls] = useState<Map<number, string>>(new Map())
  const [draggedIdx, setDraggedIdx] = useState<number | null>(null)
  const [dragOverIdx, setDragOverIdx] = useState<number | null>(null)
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    let isMounted = true
    const loadThumbnails = async () => {
      const maxToLoad = Math.min(pageCount, 40)
      for (let i = 0; i < maxToLoad; i++) {
        if (!isMounted) break
        if (thumbUrls.has(i)) continue
        try {
          const png = await invoke<number[]>('render_page_to_png', {
            data: pdfData,
            page_index: i,
            dpi: 54,
          })
          if (!isMounted) break
          const blob = new Blob([new Uint8Array(png)], { type: 'image/png' })
          const url = URL.createObjectURL(blob)
          setThumbUrls(prev => new Map(prev).set(i, url))
        } catch {
          // ignore thumbnail fail
        }
      }
    }

    loadThumbnails()
    return () => {
      isMounted = false
    }
  }, [pdfData, pageCount])

  const handleRotate = async (e: React.MouseEvent, pageIdx: number) => {
    e.stopPropagation()
    if (busy || !onPdfUpdate) return
    setBusy(true)
    try {
      const updated = await invoke<number[]>('rotate_page', {
        data: pdfData,
        page_index: pageIdx,
        degrees: 90,
      })
      onPdfUpdate(updated)
    } catch (err) {
      console.error('Rotate failed:', err)
    } finally {
      setBusy(false)
    }
  }

  const handleDelete = async (e: React.MouseEvent, pageIdx: number) => {
    e.stopPropagation()
    if (busy || !onPdfUpdate || pageCount <= 1) return
    if (!confirm(`ページ ${pageIdx + 1} を削除しますか？`)) return
    setBusy(true)
    try {
      const updated = await invoke<number[]>('delete_page', {
        data: pdfData,
        page_index: pageIdx,
      })
      onPdfUpdate(updated)
      if (currentPage >= pageIdx && currentPage > 0) {
        onGoToPage(currentPage - 1)
      }
    } catch (err) {
      console.error('Delete failed:', err)
    } finally {
      setBusy(false)
    }
  }

  const handleDragStart = (e: React.DragEvent, idx: number) => {
    setDraggedIdx(idx)
    e.dataTransfer.effectAllowed = 'move'
    e.dataTransfer.setData('text/plain', String(idx))
  }

  const handleDragOver = (e: React.DragEvent, idx: number) => {
    e.preventDefault()
    e.dataTransfer.dropEffect = 'move'
    if (dragOverIdx !== idx) {
      setDragOverIdx(idx)
    }
  }

  const handleDragEnd = () => {
    setDraggedIdx(null)
    setDragOverIdx(null)
  }

  const handleDrop = async (e: React.DragEvent, targetIdx: number) => {
    e.preventDefault()
    const sourceIdx = draggedIdx
    setDraggedIdx(null)
    setDragOverIdx(null)

    if (sourceIdx === null || sourceIdx === targetIdx || busy || !onPdfUpdate) return
    setBusy(true)
    try {
      const updated = await invoke<number[]>('reorder_pages', {
        data: pdfData,
        from_index: sourceIdx,
        to_index: targetIdx,
      })
      onPdfUpdate(updated)
      onGoToPage(targetIdx)
    } catch (err) {
      console.error('Reorder failed:', err)
    } finally {
      setBusy(false)
    }
  }

  return (
    <div style={{ padding: 10, display: 'flex', flexDirection: 'column', height: '100%', overflowY: 'auto' }}>
      <div style={{ fontSize: 11, fontWeight: 600, color: 'var(--text-muted)', textTransform: 'uppercase', marginBottom: 8, display: 'flex', justifyContent: 'space-between' }}>
        <span>ページサムネイル</span>
        <span>{pageCount} ページ</span>
      </div>
      <div style={{ fontSize: 10, color: 'var(--text-dim)', marginBottom: 8 }}>
        ドラッグ＆ドロップで並べ替え
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
        {Array.from({ length: pageCount }).map((_, idx) => {
          const isSelected = idx === currentPage
          const isDragging = idx === draggedIdx
          const isOver = idx === dragOverIdx
          const thumbUrl = thumbUrls.get(idx)

          return (
            <div
              key={idx}
              draggable
              onDragStart={e => handleDragStart(e, idx)}
              onDragOver={e => handleDragOver(e, idx)}
              onDragEnd={handleDragEnd}
              onDrop={e => handleDrop(e, idx)}
              onClick={() => onGoToPage(idx)}
              style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                cursor: isDragging ? 'grabbing' : 'grab',
                padding: 4,
                borderRadius: 'var(--radius-sm)',
                border: '1.5px solid',
                borderColor: isOver ? 'var(--yellow, #e3b341)' : isSelected ? 'var(--accent)' : 'transparent',
                background: isSelected ? 'var(--bg-2)' : 'transparent',
                opacity: isDragging ? 0.4 : 1.0,
                transform: isOver ? 'scale(1.03)' : 'scale(1)',
                transition: 'all 0.15s ease',
                position: 'relative',
              }}
            >
              <div style={{
                width: '100%',
                aspectRatio: '0.707',
                background: 'var(--bg-0)',
                border: '1px solid var(--border)',
                borderRadius: 2,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                overflow: 'hidden',
                boxShadow: isSelected ? '0 2px 8px rgba(0,0,0,0.5)' : '0 1px 3px rgba(0,0,0,0.2)',
                position: 'relative',
              }}>
                {thumbUrl ? (
                  <img src={thumbUrl} alt={`p${idx + 1}`} style={{ width: '100%', height: '100%', objectFit: 'contain', pointerEvents: 'none' }} />
                ) : (
                  <span style={{ fontSize: 10, color: 'var(--text-muted)' }}>p{idx + 1}</span>
                )}

                {/* Quick actions overlay on hover */}
                <div
                  className="thumb-actions"
                  style={{
                    position: 'absolute',
                    top: 2,
                    right: 2,
                    display: 'flex',
                    gap: 2,
                    background: 'rgba(0,0,0,0.65)',
                    padding: '2px 4px',
                    borderRadius: 4,
                    backdropFilter: 'blur(4px)',
                  }}
                  onClick={e => e.stopPropagation()}
                >
                  <button
                    onClick={e => handleRotate(e, idx)}
                    title="右に90°回転"
                    style={{
                      background: 'none',
                      border: 'none',
                      padding: 2,
                      cursor: 'pointer',
                      color: 'var(--text-dim)',
                      display: 'flex',
                      alignItems: 'center',
                    }}
                    onMouseEnter={e => (e.currentTarget.style.color = 'var(--text)')}
                    onMouseLeave={e => (e.currentTarget.style.color = 'var(--text-dim)')}
                  >
                    <RotateIcon size={11} />
                  </button>
                  {pageCount > 1 && (
                    <button
                      onClick={e => handleDelete(e, idx)}
                      title="ページ削除"
                      style={{
                        background: 'none',
                        border: 'none',
                        padding: 2,
                        cursor: 'pointer',
                        color: 'var(--red, #f85149)',
                        display: 'flex',
                        alignItems: 'center',
                      }}
                    >
                      <TrashIcon size={11} />
                    </button>
                  )}
                </div>
              </div>
              <span style={{ fontSize: 10, fontWeight: isSelected ? 600 : 400, color: isSelected ? 'var(--accent)' : 'var(--text-dim)', marginTop: 4 }}>
                {idx + 1}
              </span>
            </div>
          )
        })}
      </div>
    </div>
  )
}

export function InfoPanel({ metadata }: { metadata: Record<string, unknown> | null }) {
  if (!metadata) return (
    <div style={{ padding: 12, fontSize: 12, color: 'var(--text-muted)', textAlign: 'center' }}>
      読み込み中...
    </div>
  )

  return (
    <div style={{ padding: 12 }}>
      <div style={{ fontSize: 11, fontWeight: 600, color: 'var(--text-muted)', textTransform: 'uppercase', marginBottom: 8 }}>
        PDF情報
      </div>
      <div style={{ fontSize: 12, color: 'var(--text-dim)', lineHeight: 1.8 }}>
        <div><span style={{ color: 'var(--text-muted)' }}>タイトル:</span> {(metadata.title as string) || 'なし'}</div>
        <div><span style={{ color: 'var(--text-muted)' }}>著者:</span> {(metadata.author as string) || 'なし'}</div>
        <div><span style={{ color: 'var(--text-muted)' }}>ページ数:</span> {metadata.page_count as number}</div>
        <div><span style={{ color: 'var(--text-muted)' }}>バージョン:</span> PDF {(metadata.version as string) || '?'}</div>
        <div><span style={{ color: 'var(--text-muted)' }}>ファイルサイズ:</span> {((metadata.size as number) || 0).toLocaleString()} bytes</div>
      </div>
    </div>
  )
}

export function SeparationsPanel({
  sepPlates,
  setSepPlates,
  tacLimit,
  setTacLimit,
}: {
  sepPlates: { c: boolean; m: boolean; y: boolean; k: boolean; tac: boolean }
  setSepPlates: React.Dispatch<React.SetStateAction<{ c: boolean; m: boolean; y: boolean; k: boolean; tac: boolean }>>
  tacLimit: number
  setTacLimit: (val: number) => void
}) {
  return (
    <div style={{ padding: 14, overflowY: 'auto' }}>
      <div style={{ fontSize: 13, fontWeight: 700, color: 'var(--text)', marginBottom: 4 }}>
        分版プレビュー (Separations)
      </div>
      <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 12, lineHeight: 1.4 }}>
        印刷版ごとの表示切り替えおよび総インキ量(TAC)をリアルタイム検証
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 8, marginBottom: 16 }}>
        {[
          { key: 'c', label: 'シアン (Cyan 版)', color: '#00d2ff' },
          { key: 'm', label: 'マゼンタ (Magenta 版)', color: '#ff2d75' },
          { key: 'y', label: 'イエロー (Yellow 版)', color: '#ffe600' },
          { key: 'k', label: 'ブラック (Key/Black 版)', color: '#aaaaaa' },
        ].map(plate => (
          <label
            key={plate.key}
            style={{
              display: 'flex', alignItems: 'center', gap: 8,
              padding: '6px 10px', borderRadius: 4, background: 'var(--bg-0)',
              border: '1px solid var(--border)', fontSize: 11, cursor: 'pointer',
              color: 'var(--text)'
            }}
          >
            <input
              type="checkbox"
              checked={sepPlates[plate.key as 'c' | 'm' | 'y' | 'k']}
              onChange={e => setSepPlates(prev => ({ ...prev, [plate.key]: e.target.checked }))}
            />
            <span style={{ display: 'inline-block', width: 10, height: 10, borderRadius: '50%', background: plate.color }} />
            <span style={{ fontWeight: 500 }}>{plate.label}</span>
          </label>
        ))}
      </div>

      <div style={{
        padding: 10, borderRadius: 6, background: 'var(--bg-0)',
        border: `1px solid ${sepPlates.tac ? '#ff0055' : 'var(--border)'}`,
        marginBottom: 12
      }}>
        <label style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 11, fontWeight: 600, color: sepPlates.tac ? '#ff0055' : 'var(--text)', cursor: 'pointer', marginBottom: 8 }}>
          <input
            type="checkbox"
            checked={sepPlates.tac}
            onChange={e => setSepPlates(prev => ({ ...prev, tac: e.target.checked }))}
          />
          総インキ量 (TAC) 警告表示
        </label>
        <div style={{ fontSize: 10, color: 'var(--text-muted)', marginBottom: 8 }}>
          許容インキ上限を超える領域をネオンカラーで強調ハイライト
        </div>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', fontSize: 11 }}>
          <span style={{ color: 'var(--text-muted)' }}>上限値 (%):</span>
          <select
            value={tacLimit}
            onChange={e => setTacLimit(Number(e.target.value))}
            style={{ background: 'var(--bg-2)', color: 'var(--text)', border: '1px solid var(--border)', borderRadius: 4, padding: '2px 6px', fontSize: 11 }}
          >
            <option value={280}>280% (新聞・オフセット標準)</option>
            <option value={300}>300% (Japan Color 枚葉一般)</option>
            <option value={320}>320% (高品質商業印刷)</option>
            <option value={350}>350% (特殊コート紙)</option>
          </select>
        </div>
      </div>

      <button
        onClick={() => setSepPlates({ c: true, m: true, y: true, k: true, tac: false })}
        style={{
          width: '100%', padding: '6px 12px', background: 'var(--bg-2)',
          border: '1px solid var(--border)', color: 'var(--text)',
          borderRadius: 4, fontSize: 11, cursor: 'pointer'
        }}
      >
        全版ON・通常表示に戻す
      </button>
    </div>
  )
}
