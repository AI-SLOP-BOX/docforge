import { useState } from 'react'
import { Input, NumInput, AccentBtn } from './UIControls'

export function OrganizePanel({ exec }: { exec: Function }) {
  const [pageIndex, setPageIndex] = useState(0)
  const [rotation, setRotation] = useState(90)
  const [fromIdx, setFromIdx] = useState(0)
  const [toIdx, setToIdx] = useState(0)
  const [extractIndices, setExtractIndices] = useState('')
  const [rangeStart, setRangeStart] = useState(0)
  const [rangeEnd, setRangeEnd] = useState(4)

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      {/* Card 1: Page Manipulations */}
      <div className="inspector-card">
        <div className="inspector-card-header">
          <span>ページ回転・複製・削除</span>
          <span style={{ fontSize: 9, color: 'var(--accent)', fontWeight: 600 }}>PAGE</span>
        </div>
        <NumInput value={pageIndex} onChange={setPageIndex} label="対象ページ番号 (0始まり)" />
        <div style={{ display: 'flex', gap: 6, marginTop: 4 }}>
          <AccentBtn onClick={() => exec('rotate_page', { page_index: pageIndex, degrees: rotation })} style={{ flex: 2 }}>
            {rotation}° 回転
          </AccentBtn>
          <AccentBtn onClick={() => setRotation(prev => (prev + 90) % 360)} style={{ flex: 1, background: 'var(--bg-2)', color: 'var(--text)', border: '1px solid var(--border)' }}>
            +90°
          </AccentBtn>
        </div>
        <div style={{ display: 'flex', gap: 6, marginTop: 6 }}>
          <AccentBtn onClick={() => exec('duplicate_page', { page_index: pageIndex })} style={{ flex: 1, background: 'var(--bg-2)', color: 'var(--text)', border: '1px solid var(--border)' }}>
            ページ複製
          </AccentBtn>
          <AccentBtn onClick={() => exec('delete_page', { page_index: pageIndex })} style={{ flex: 1, background: 'rgba(248, 81, 73, 0.15)', color: '#f85149', border: '1px solid rgba(248, 81, 73, 0.3)' }}>
            ページ削除
          </AccentBtn>
        </div>
      </div>

      {/* Card 2: Reorder */}
      <div className="inspector-card">
        <div className="inspector-card-header">
          <span>ページ順序の移動</span>
          <span style={{ fontSize: 9, color: 'var(--text-muted)' }}>MOVE</span>
        </div>
        <div style={{ display: 'flex', gap: 8 }}>
          <NumInput value={fromIdx} onChange={setFromIdx} label="移動元 (from)" />
          <NumInput value={toIdx} onChange={setToIdx} label="移動先 (to)" />
        </div>
        <AccentBtn onClick={() => exec('reorder_pages', { from_index: fromIdx, to_index: toIdx })} style={{ marginTop: 6 }}>
          順序を変更
        </AccentBtn>
      </div>

      {/* Card 3: Extraction */}
      <div className="inspector-card">
        <div className="inspector-card-header">
          <span>ページ抽出・分割</span>
          <span style={{ fontSize: 9, color: 'var(--text-muted)' }}>EXTRACT</span>
        </div>
        <div className="inspector-card-desc">カンマ区切り（例: 0, 2, 4）または連続範囲</div>
        <Input value={extractIndices} onChange={setExtractIndices} placeholder="指定: 0,2,4" />
        <AccentBtn onClick={() => {
          const indices = extractIndices.split(',').map(s => parseInt(s.trim())).filter(n => !isNaN(n))
          if (indices.length > 0) exec('extract_pages', { indices })
        }} style={{ background: 'var(--bg-2)', color: 'var(--text)', border: '1px solid var(--border)' }}>
          指定ページを抽出
        </AccentBtn>
        <div style={{ display: 'flex', gap: 8, marginTop: 10 }}>
          <NumInput value={rangeStart} onChange={setRangeStart} label="開始" />
          <NumInput value={rangeEnd} onChange={setRangeEnd} label="終了" />
        </div>
        <AccentBtn onClick={() => {
          const indices = Array.from(
            { length: Math.max(0, rangeEnd - rangeStart + 1) },
            (_, i) => rangeStart + i
          )
          if (indices.length > 0) exec('extract_pages', { indices })
        }} style={{ marginTop: 6 }}>
          範囲を一括抽出
        </AccentBtn>
      </div>

      {/* Card 4: Page Crop / TrimBox */}
      <CropCard exec={exec} />
    </div>
  )
}

function CropCard({ exec }: { exec: Function }) {
  const [page, setPage] = useState(0)
  const [cropX, setCropX] = useState(20)
  const [cropY, setCropY] = useState(20)
  const [cropW, setCropW] = useState(555)
  const [cropH, setCropH] = useState(802)

  return (
    <div className="inspector-card">
      <div className="inspector-card-header">
        <span>ページトリミング・クロップ</span>
        <span style={{ fontSize: 9, color: 'var(--green, #3fb950)', fontWeight: 600 }}>CROP</span>
      </div>
      <div className="inspector-card-desc">MediaBox/CropBox領域を再定義して余白やトンボをカット</div>
      <NumInput value={page} onChange={setPage} label="対象ページ (0始まり)" />
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 6, marginTop: 4 }}>
        <NumInput value={cropX} onChange={setCropX} label="X始点 (pt)" />
        <NumInput value={cropY} onChange={setCropY} label="Y始点 (pt)" />
        <NumInput value={cropW} onChange={setCropW} label="幅 (pt)" />
        <NumInput value={cropH} onChange={setCropH} label="高さ (pt)" />
      </div>
      <AccentBtn
        onClick={() => exec('crop_page', { page_index: page, x: cropX, y: cropY, width: cropW, height: cropH })}
        style={{ marginTop: 8, background: 'var(--green, #238636)', color: '#fff' }}
      >
        クロップを適用
      </AccentBtn>
    </div>
  )
}
