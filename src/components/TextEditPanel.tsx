import { useState, useCallback, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { SectionTitle, Input, NumInput, ColorInput, AccentBtn } from './UIControls'
import type { TextBlock } from './PDFViewer'

export function TextEditPanel({
  pdfData,
  exec,
  showToast,
  onPdfUpdate,
  selectedBlockFromCanvas,
  currentPage
}: {
  pdfData: number[] | null
  exec: Function
  showToast: (msg: string) => void
  onPdfUpdate: (data: number[]) => void
  selectedBlockFromCanvas?: TextBlock | null
  currentPage?: number
}) {
  type Block = { id: number; text: string; x: number; y: number; font_name: string; font_size: number; color: string }
  const [textBlocks, setTextBlocks] = useState<Block[]>([])
  const [selectedBlock, setSelectedBlock] = useState<number | null>(null)
  const [editTextVal, setEditTextVal] = useState('')
  const [pageIndex, setPageIndex] = useState(currentPage ?? 0)
  const [moveX, setMoveX] = useState(100)
  const [moveY, setMoveY] = useState(700)
  const [colorOld, setColorOld] = useState('#000000')
  const [colorNew, setColorNew] = useState('#FF0000')
  const [sizeOld, setSizeOld] = useState(12)
  const [sizeNew, setSizeNew] = useState(14)
  const [fontOld, setFontOld] = useState('Helvetica')
  const [fontNew, setFontNew] = useState('Helvetica-Bold')

  useEffect(() => {
    if (currentPage !== undefined) {
      setPageIndex(currentPage)
    }
  }, [currentPage])

  useEffect(() => {
    if (selectedBlockFromCanvas) {
      setSelectedBlock(selectedBlockFromCanvas.id)
      setEditTextVal(selectedBlockFromCanvas.text)
      setMoveX(Math.round(selectedBlockFromCanvas.x))
      setMoveY(Math.round(selectedBlockFromCanvas.y))
    }
  }, [selectedBlockFromCanvas])

  const reloadBlocks = useCallback(async (data: number[]) => {
    try {
      const blocks = await invoke<Block[]>('get_text_blocks', { data, page_index: pageIndex })
      setTextBlocks(blocks)
    } catch (err) { showToast(`エラー: ${err}`) }
  }, [pageIndex, showToast])

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      {/* Card 1: Text Inspector & Direct Modify */}
      <div className="inspector-card">
        <div className="inspector-card-header">
          <span>テキスト直接選択・編集</span>
          <span style={{ fontSize: 9, color: 'var(--accent)', fontWeight: 600 }}>OCR/VECTOR</span>
        </div>
        <div className="inspector-card-desc">画面クリックまたは一覧からテキストを選択して編集</div>

        <NumInput value={pageIndex} onChange={setPageIndex} label="ページ番号" />
        <AccentBtn onClick={() => { if (pdfData) reloadBlocks(pdfData) }} style={{ width: '100%', marginBottom: 8, background: 'var(--bg-2)', color: 'var(--text)', border: '1px solid var(--border)' }}>
          テキストブロック読込
        </AccentBtn>

        {textBlocks.length > 0 && (
          <div style={{ maxHeight: 150, overflow: 'auto', marginBottom: 8, display: 'flex', flexDirection: 'column', gap: 3 }}>
            {textBlocks.map((block) => (
              <div
                key={block.id}
                onClick={() => {
                  setSelectedBlock(block.id)
                  setEditTextVal(block.text)
                  setMoveX(Math.round(block.x))
                  setMoveY(Math.round(block.y))
                }}
                style={{
                  padding: '5px 8px', background: 'var(--bg-0)',
                  borderRadius: 'var(--radius-sm)', border: '1px solid',
                  fontSize: 11, cursor: 'pointer',
                  borderColor: selectedBlock === block.id ? 'var(--accent)' : 'var(--border)',
                }}
              >
                <div style={{ color: 'var(--text)', fontWeight: selectedBlock === block.id ? 600 : 400 }}>
                  {block.text.substring(0, 26)}{block.text.length > 26 ? '…' : ''}
                </div>
                <div style={{ color: 'var(--text-dim)', fontSize: 10, marginTop: 2 }}>
                  {block.font_name} {block.font_size}pt @ ({Math.round(block.x)}, {Math.round(block.y)})
                </div>
              </div>
            ))}
          </div>
        )}

        {selectedBlock !== null && (
          <div style={{ padding: 8, background: 'var(--bg-2)', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border)', marginTop: 4 }}>
            <div style={{ fontSize: 10, fontWeight: 600, color: 'var(--text-muted)', marginBottom: 4 }}>選択中のテキスト内容</div>
            <Input value={editTextVal} onChange={setEditTextVal} placeholder="新しいテキスト" />
            <div style={{ display: 'flex', gap: 6, marginTop: 4 }}>
              <AccentBtn onClick={async () => {
                if (!pdfData) return
                try {
                  const result = await invoke<number[]>('edit_text_block', { data: pdfData, page_index: pageIndex, block_id: selectedBlock, new_text: editTextVal })
                  onPdfUpdate(result)
                  await reloadBlocks(result)
                  showToast('テキストを更新しました')
                } catch (err) { showToast(`エラー: ${err}`) }
              }} style={{ flex: 1, background: 'var(--green)' }}>
                更新
              </AccentBtn>
              <AccentBtn onClick={async () => {
                if (!pdfData) return
                try {
                  const result = await invoke<number[]>('delete_text_block', { data: pdfData, page_index: pageIndex, block_id: selectedBlock })
                  onPdfUpdate(result)
                  await reloadBlocks(result)
                  setSelectedBlock(null)
                  showToast('テキストを削除しました')
                } catch (err) { showToast(`エラー: ${err}`) }
              }} style={{ flex: 1, background: 'rgba(248, 81, 73, 0.15)', color: '#f85149', border: '1px solid rgba(248, 81, 73, 0.4)' }}>
                削除
              </AccentBtn>
            </div>

            <div style={{ fontSize: 10, fontWeight: 600, color: 'var(--text-muted)', marginTop: 8, marginBottom: 4 }}>座標移動</div>
            <div style={{ display: 'flex', gap: 8 }}>
              <NumInput value={moveX} onChange={setMoveX} label="X座標 (pt)" />
              <NumInput value={moveY} onChange={setMoveY} label="Y座標 (pt)" />
            </div>
            <AccentBtn onClick={async () => {
              if (!pdfData) return
              try {
                const result = await invoke<number[]>('move_text_block', { data: pdfData, page_index: pageIndex, block_id: selectedBlock, new_x: moveX, new_y: moveY })
                onPdfUpdate(result)
                await reloadBlocks(result)
                showToast('テキストを移動しました')
              } catch (err) { showToast(`エラー: ${err}`) }
            }} style={{ background: 'var(--purple)', marginTop: 4 }}>
              移動実行
            </AccentBtn>
          </div>
        )}
      </div>

      {/* Card 2: Color & Size Replacement */}
      <div className="inspector-card">
        <div className="inspector-card-header">
          <span>スタイル一括変換</span>
          <span style={{ fontSize: 9, color: 'var(--text-muted)' }}>BATCH</span>
        </div>
        <div style={{ display: 'flex', gap: 8, marginBottom: 6 }}>
          <ColorInput value={colorOld} onChange={setColorOld} label="置換元カラー" />
          <ColorInput value={colorNew} onChange={setColorNew} label="置換後カラー" />
        </div>
        <AccentBtn onClick={() => exec('change_text_color', { page_index: pageIndex, old_color: colorOld, new_color: colorNew })} style={{ background: 'var(--bg-2)', color: 'var(--text)', border: '1px solid var(--border)' }}>
          色を一括変換
        </AccentBtn>

        <div style={{ display: 'flex', gap: 8, marginTop: 10, marginBottom: 6 }}>
          <NumInput value={sizeOld} onChange={setSizeOld} label="元のサイズ" />
          <NumInput value={sizeNew} onChange={setSizeNew} label="新しいサイズ" />
        </div>
        <AccentBtn onClick={() => exec('change_font_size', { page_index: pageIndex, old_size: sizeOld, new_size: sizeNew })} style={{ background: 'var(--bg-2)', color: 'var(--text)', border: '1px solid var(--border)' }}>
          サイズを一括変換
        </AccentBtn>
      </div>

      {/* Card 3: Font Management */}
      <div className="inspector-card">
        <div className="inspector-card-header">
          <span>フォント置換</span>
          <span style={{ fontSize: 9, color: 'var(--text-muted)' }}>FONT</span>
        </div>
        <AccentBtn onClick={async () => {
          if (!pdfData) return
          try {
            const fonts = await invoke<Array<{name: string; type: string}>>('get_fonts', { data: pdfData })
            showToast(`${fonts.length}個のフォントを検出`)
          } catch (err) { showToast(`エラー: ${err}`) }
        }} style={{ background: 'var(--bg-2)', color: 'var(--text)', border: '1px solid var(--border)' }}>
          フォント一覧を検出
        </AccentBtn>
        <div style={{ marginTop: 8 }}>
          <Input value={fontOld} onChange={setFontOld} placeholder="元フォント名 (例: Helvetica)" />
          <Input value={fontNew} onChange={setFontNew} placeholder="新フォント名 (例: Times-Roman)" />
        </div>
        <AccentBtn onClick={() => exec('replace_font', { old_font: fontOld, new_font: fontNew })} style={{ marginTop: 4 }}>
          フォントを置換
        </AccentBtn>
      </div>
    </div>
  )
}
