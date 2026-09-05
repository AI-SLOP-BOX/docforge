import { useState } from 'react'
import { open } from '@tauri-apps/plugin-dialog'
import { Input, NumInput, ColorInput, AccentBtn } from './UIControls'

export function EditPanel({
  exec,
  pdfData,
  showToast,
  currentPage = 0,
}: {
  exec: Function
  pdfData: number[] | null
  showToast: (msg: string) => void
  currentPage?: number
}) {
  const [text, setText] = useState('')
  const [fontSize, setFontSize] = useState(16)
  const [textColor, setTextColor] = useState('#000000')
  const [mergePaths, setMergePaths] = useState<string[]>([])

  // Text editing state
  const [searchText, setSearchText] = useState('')
  const [replaceText, setReplaceText] = useState('')
  const [reflowText, setReflowText] = useState('')
  const [reflowWidth, setReflowWidth] = useState(400)
  const [lineHeight, setLineHeight] = useState(14)

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      {/* Card 1: Direct Text Add */}
      <div className="inspector-card">
        <div className="inspector-card-header">
          <span>テキスト挿入</span>
          <span style={{ fontSize: 9, color: 'var(--accent)', fontWeight: 600 }}>POINT</span>
        </div>
        <div className="inspector-card-desc">クリックした位置にベクターテキストを配置</div>
        <Input value={text} onChange={setText} placeholder="テキストを入力..." />
        <div style={{ display: 'flex', gap: 8, marginTop: 4 }}>
          <NumInput value={fontSize} onChange={setFontSize} label="サイズ (pt)" />
          <ColorInput value={textColor} onChange={setTextColor} label="カラー" />
        </div>
        <AccentBtn onClick={() => exec('add_text', { page_index: currentPage, text, x: 50, y: 700, size: fontSize, color: textColor })}>
          ページに追加 (p{currentPage + 1})
        </AccentBtn>
      </div>

      {/* Card 2: Find & Replace */}
      <div className="inspector-card">
        <div className="inspector-card-header">
          <span>検索＆インプレース置換</span>
          <span style={{ fontSize: 9, color: 'var(--text-muted)' }}>STREAM</span>
        </div>
        <div className="inspector-card-desc">PDF内部の文字オペレータを直接書換</div>
        <Input value={searchText} onChange={setSearchText} placeholder="検索対象の文字列" />
        <Input value={replaceText} onChange={setReplaceText} placeholder="新しい文字列" />
        <AccentBtn onClick={() => exec('edit_text', { page_index: currentPage, search_text: searchText, replacement: replaceText, font_name: 'Helvetica', font_size: fontSize, color: textColor })}>
          テキストを置換 (p{currentPage + 1})
        </AccentBtn>
      </div>

      {/* Card 3: Reflow with Japanese Kinsoku Shori */}
      <div className="inspector-card">
        <div className="inspector-card-header">
          <span>文章自動リフロー・組版</span>
          <span style={{ fontSize: 9, color: 'var(--accent)', fontWeight: 600 }}>JIS X 4051</span>
        </div>
        <div className="inspector-card-desc">禁則処理（追い出し・追い込み）とグリフ幅を考慮して自動折り返し</div>
        <textarea
          value={reflowText}
          onChange={e => setReflowText(e.target.value)}
          placeholder="流し込む文章を入力（日本語・英語混在対応）..."
          style={{
            width: '100%', height: 68, padding: '6px 8px', background: 'var(--bg-0)',
            border: '1px solid var(--border)', borderRadius: 'var(--radius-sm)',
            color: 'var(--text)', fontSize: 11, resize: 'vertical',
            boxShadow: 'inset 0 1px 3px rgba(0,0,0,0.25)',
          }}
        />
        <div style={{ display: 'flex', gap: 8, marginTop: 4 }}>
          <NumInput value={reflowWidth} onChange={setReflowWidth} label="行送り最大幅 (pt)" />
          <NumInput value={lineHeight} onChange={setLineHeight} label="行送り (pt)" />
        </div>
        <AccentBtn onClick={() => exec('reflow_text', { page_index: currentPage, new_text: reflowText, start_x: 50, start_y: 700, max_width: reflowWidth, font_size: fontSize, line_height: lineHeight, color: textColor })}>
          組版リフロー流し込み (p{currentPage + 1})
        </AccentBtn>
      </div>

      {/* Card 4: Merge PDF */}
      <div className="inspector-card">
        <div className="inspector-card-header">
          <span>PDFファイル結合</span>
          <span style={{ fontSize: 9, color: 'var(--text-muted)' }}>BATCH</span>
        </div>
        <AccentBtn
          onClick={async () => {
            const paths = await open({ filters: [{ name: 'PDF', extensions: ['pdf'] }], multiple: true })
            if (paths) setMergePaths(Array.isArray(paths) ? paths : [paths])
          }}
          style={{ background: 'var(--bg-2)', color: 'var(--text)', border: '1px solid var(--border)' }}
        >
          ファイルを選択...
        </AccentBtn>
        {mergePaths.length > 0 && (
          <div style={{ fontSize: 11, color: 'var(--accent)', marginTop: 6, fontWeight: 500 }}>
            {mergePaths.length} 個のファイルを選択中
          </div>
        )}
        <AccentBtn
          onClick={() => exec('merge_pdfs', { paths: mergePaths })}
          disabled={mergePaths.length < 2}
          style={{ marginTop: 8 }}
        >
          結合を実行
        </AccentBtn>
      </div>
    </div>
  )
}
