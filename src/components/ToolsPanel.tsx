import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import { SectionTitle, Input, SliderInput, ColorInput, AccentBtn } from './UIControls'
import { RedactIcon, VectorPathIcon, CheckIcon, ZapIcon } from './Icons'
import { AccessibilitySection, AccessibilityReport } from './ToolsAccessibilitySection'
import { ToolsPDFXSection } from './ToolsPDFXSection'
import { ToolsAdvancedEngineeringSection } from './ToolsAdvancedEngineeringSection'
import { ToolsBatchAndColorSection } from './ToolsBatchAndColorSection'

export function ToolsPanel({
  exec,
  pdfData,
  redactColor,
  setRedactColor,
  redactSearchText,
  setRedactSearchText,
  redactReplacement,
  setRedactReplacement,
  showToast,
  onActivateDrawRedact,
  onPdfUpdate,
}: {
  exec: Function
  pdfData: number[] | null
  redactColor: string
  setRedactColor: (v: string) => void
  redactSearchText: string
  setRedactSearchText: (v: string) => void
  redactReplacement: string
  setRedactReplacement: (v: string) => void
  showToast: (msg: string) => void
  onActivateDrawRedact?: () => void
  onPdfUpdate?: (data: number[]) => void
}) {
  const [compressQuality, setCompressQuality] = useState(85)
  const [outlineBusy, setOutlineBusy] = useState(false)
  const [accessReport, setAccessReport] = useState<AccessibilityReport | null>(null)
  const [optResult, setOptResult] = useState<{
    beforeBytes: number
    afterBytes: number
    savedBytes: number
    percent: number
  } | null>(null)

  const handleSanitize = async () => {
    if (!pdfData) return
    try {
      const [cleanBytes, summary] = await invoke<[number[], {
        metadata_removed: boolean
        annotations_purged: number
        attachments_removed: number
        javascript_removed: boolean
        thumbnails_purged: number
      }]>('sanitize_document', { data: pdfData })

      onPdfUpdate?.(cleanBytes)
      showToast(`完全サニタイズ完了: 注釈${summary.annotations_purged}件削除, メタデータ/JS完全パージ`)
    } catch (err) {
      showToast(`サニタイズエラー: ${err}`)
    }
  }

  const handleConvertToOutlines = async () => {
    if (!pdfData || outlineBusy) return
    setOutlineBusy(true)
    try {
      const outlinedBytes = await invoke<number[]>('convert_fonts_to_outlines', { data: pdfData })
      onPdfUpdate?.(outlinedBytes)
      showToast('全フォントのアウトライン化完了: 印刷用ベクターパスに変換されました')
    } catch (err) {
      showToast(`アウトライン化エラー: ${err}`)
    } finally {
      setOutlineBusy(false)
    }
  }

  return (
    <div>
      <SectionTitle>テキストのアウトライン化（Create Outlines）</SectionTitle>
      <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 8, lineHeight: 1.5 }}>
        PDF内の全フォント埋め込み・テキスト情報を、入稿・印刷用の完全なベクターパス図形へと変換します（文字化け・フォント置換ゼロ化）
      </div>
      <AccentBtn
        onClick={handleConvertToOutlines}
        disabled={outlineBusy}
        style={{
          background: 'linear-gradient(135deg, #1f6feb 0%, #388bfd 100%)',
          color: '#ffffff',
          marginBottom: 16,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          gap: 6,
        }}
      >
        <VectorPathIcon size={14} />
        {outlineBusy ? 'アウトライン化処理中...' : '全テキストをベクターパスに変換'}
      </AccentBtn>

      <SectionTitle>完全サニタイズ（Sanitize Document）</SectionTitle>
      <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 8, lineHeight: 1.5 }}>
        隠しレイヤー・非表示メタデータ・注釈履歴・JavaScript・添付ファイルを根こそぎ完全消去します
      </div>
      <AccentBtn onClick={handleSanitize} style={{ background: '#e03e3e', color: '#ffffff', marginBottom: 12 }}>
        文書をサニタイズして保存
      </AccentBtn>

      <SectionTitle>高度な黒塗り（Deep Redaction）</SectionTitle>
      <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 8, lineHeight: 1.5 }}>
        メタデータ・注釈・隠しオブジェクトを完全にパージします
      </div>
      <div style={{ display: 'flex', gap: 4 }}>
        <AccentBtn onClick={() => onActivateDrawRedact?.()} style={{ background: '#ff3344', color: '#fff' }}>
          <RedactIcon size={14} /> ドラッグ黒塗り描画
        </AccentBtn>
        <AccentBtn
          onClick={() => exec('deep_redact', { page_index: 0, x: 50, y: 700, width: 200, height: 20, color: redactColor })}
          style={{ background: 'var(--red)' }}
        >
          即時完全消去
        </AccentBtn>
      </div>

      <SectionTitle>テキスト黒塗り</SectionTitle>
      <Input value={redactSearchText} onChange={setRedactSearchText} placeholder="検索テキスト" />
      <Input value={redactReplacement} onChange={setRedactReplacement} placeholder="置換テキスト" />
      <ColorInput value={redactColor} onChange={setRedactColor} label="黒塗り色" />
      <AccentBtn onClick={() => exec('redact_area', { page_index: 0, x: 50, y: 700, width: 200, height: 20, color: redactColor })}>
        エリア黒塗り（現在設定値）
      </AccentBtn>
      <AccentBtn onClick={() => exec('redact_text', { search_text: redactSearchText, replacement: redactReplacement })}>
        テキスト検索＆黒塗り
      </AccentBtn>
      <AccentBtn onClick={() => exec('redact_text_deep', { search_text: redactSearchText, color: redactColor })} style={{ background: 'var(--red)' }}>
        テキスト完全消去（データ削除）
      </AccentBtn>

      <ToolsBatchAndColorSection
        exec={exec}
        pdfData={pdfData}
        showToast={showToast}
      />

      <ToolsAdvancedEngineeringSection
        pdfData={pdfData}
        showToast={showToast}
        onPdfUpdate={onPdfUpdate}
      />

      <SectionTitle>高度な最適化</SectionTitle>
      <AccentBtn onClick={() => exec('downsample_images', { target_dpi: 150, quality: 85 })}>
        画像ダウンサンプリング
      </AccentBtn>
      <AccentBtn onClick={() => exec('remove_metadata', {})}>
        メタデータ除去
      </AccentBtn>
      <AccentBtn onClick={() => exec('flatten_content', {})}>
        コンテンツフラット化
      </AccentBtn>

      <SectionTitle>ハードウェアトークン</SectionTitle>
      <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 8, lineHeight: 1.5 }}>
        USBトークン/ICカード連携
      </div>
      <AccentBtn onClick={async () => {
        try {
          const tokens = await invoke<Array<{slot_id: number; label: string; manufacturer: string}>>('detect_hardware_tokens')
          showToast(`検出: ${tokens.length}台`)
        } catch (err) { showToast(`エラー: ${err}`) }
      }}>
        トークン検出
      </AccentBtn>

      <SectionTitle>PDF比較</SectionTitle>
      <AccentBtn onClick={async () => {
        if (!pdfData) return
        const path = await open({ filters: [{ name: 'PDF', extensions: ['pdf'] }], multiple: false })
        if (path) {
          const bytes = await invoke<number[]>('read_file_bytes', { path: path as string })
          const result = await invoke<{pages_same: number; pages_different: number}>('compare_pdfs', { data1: pdfData, data2: bytes })
          showToast(`比較: ${result.pages_same}ページ一致, ${result.pages_different}ページ異なる`)
        }
      }}>
        PDF比較
      </AccentBtn>

      <SectionTitle>PDF最適化・データ削減メーター</SectionTitle>
      <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 8, lineHeight: 1.5 }}>
        重複オブジェクト除去、不要メタデータ整理、ストリーム再圧縮
      </div>
      <AccentBtn
        onClick={async () => {
          if (!pdfData) return
          const beforeBytes = pdfData.length
          try {
            const optimized = await invoke<number[]>('optimize_pdf', { data: pdfData })
            const afterBytes = optimized.length
            const savedBytes = Math.max(0, beforeBytes - afterBytes)
            const percent = beforeBytes > 0 ? ((savedBytes / beforeBytes) * 100) : 0
            setOptResult({ beforeBytes, afterBytes, savedBytes, percent })
            if (onPdfUpdate) {
              onPdfUpdate(optimized)
            } else {
              exec('optimize_pdf', {})
            }
            showToast(`最適化完了: ${(savedBytes / 1024).toFixed(1)} KB削減 (${percent.toFixed(1)}% 縮小)`)
          } catch (err) {
            showToast(`最適化エラー: ${err}`)
          }
        }}
        style={{ background: 'linear-gradient(135deg, #238636 0%, #2ea043 100%)', color: '#fff', marginBottom: 10 }}
      >
        <ZapIcon size={14} /> 最適化を実行（ロスレス）
      </AccentBtn>

      <SectionTitle>品質圧縮（ダウンサンプリング）</SectionTitle>
      <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 8 }}>
        画像品質と解像度を調整してファイルサイズを大幅に削減します
      </div>
      <SliderInput value={compressQuality} onChange={setCompressQuality} label="圧縮品質 (%)" min={10} max={100} step={5} />
      <AccentBtn
        onClick={async () => {
          if (!pdfData) return
          const beforeBytes = pdfData.length
          try {
            const compressed = await invoke<number[]>('compress_pdf_quality', { data: pdfData, quality: compressQuality })
            const afterBytes = compressed.length
            const savedBytes = Math.max(0, beforeBytes - afterBytes)
            const percent = beforeBytes > 0 ? ((savedBytes / beforeBytes) * 100) : 0
            setOptResult({ beforeBytes, afterBytes, savedBytes, percent })
            if (onPdfUpdate) {
              onPdfUpdate(compressed)
            } else {
              exec('compress_pdf_quality', { quality: compressQuality })
            }
            showToast(`圧縮完了: ${(savedBytes / 1024).toFixed(1)} KB削減 (${percent.toFixed(1)}% 縮小)`)
          } catch (err) {
            showToast(`圧縮エラー: ${err}`)
          }
        }}
        style={{ marginBottom: 12 }}
      >
        指定品質で圧縮実行
      </AccentBtn>

      {/* Visual Optimization Result Meter */}
      {optResult && (
        <div
          style={{
            background: 'var(--bg-0)',
            border: '1px solid var(--accent)',
            borderRadius: 8,
            padding: 12,
            marginBottom: 16,
            boxShadow: '0 4px 16px rgba(0,0,0,0.3)',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 8 }}>
            <span style={{ fontSize: 12, fontWeight: 600, color: 'var(--accent)', display: 'flex', alignItems: 'center', gap: 6 }}>
              <CheckIcon size={14} color="var(--accent)" /> 削減結果フィードバック
            </span>
            <span style={{ fontSize: 11, color: 'var(--green)', fontWeight: 600, fontFamily: 'monospace' }}>
              -{optResult.percent.toFixed(1)}%
            </span>
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8, marginBottom: 8 }}>
            <div style={{ background: 'var(--bg-1)', padding: '6px 8px', borderRadius: 4, border: '1px solid var(--border)' }}>
              <div style={{ fontSize: 10, color: 'var(--text-muted)' }}>圧縮前</div>
              <div style={{ fontSize: 12, fontWeight: 600, fontFamily: 'monospace' }}>
                {(optResult.beforeBytes / 1024 / 1024).toFixed(2)} MB
              </div>
            </div>
            <div style={{ background: 'var(--bg-1)', padding: '6px 8px', borderRadius: 4, border: '1px solid var(--border)' }}>
              <div style={{ fontSize: 10, color: 'var(--text-muted)' }}>圧縮後</div>
              <div style={{ fontSize: 12, fontWeight: 600, color: 'var(--green)', fontFamily: 'monospace' }}>
                {(optResult.afterBytes / 1024 / 1024).toFixed(2)} MB
              </div>
            </div>
          </div>

          {/* Graphical Meter Bar */}
          <div style={{ height: 6, background: 'rgba(255,255,255,0.08)', borderRadius: 3, overflow: 'hidden', display: 'flex' }}>
            <div
              style={{
                width: `${Math.max(4, Math.min(100, 100 - optResult.percent))}%`,
                background: 'var(--accent)',
                transition: 'width 0.4s ease',
              }}
            />
            <div
              style={{
                width: `${Math.max(0, Math.min(100, optResult.percent))}%`,
                background: 'rgba(255,255,255,0.1)',
              }}
            />
          </div>
          <div style={{ fontSize: 10, color: 'var(--text-muted)', marginTop: 4, textAlign: 'right' }}>
            削減量: {(optResult.savedBytes / 1024).toFixed(1)} KB
          </div>
        </div>
      )}

      <SectionTitle>PDF変換</SectionTitle>
      <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 8, lineHeight: 1.5 }}>
        PDFを様々な形式に変換
      </div>
      <AccentBtn onClick={async () => {
        if (!pdfData) return
        const dir = await open({ directory: true })
        if (dir) {
          try {
            const images = await invoke<string[]>('pdf_to_images', { data: pdfData, output_dir: dir as string, format: 'png', dpi: 200 })
            showToast(`${images.length}ページをPNGに変換しました`)
          } catch (err) { showToast(`エラー: ${err}`) }
        }
      }}>
        PDF→PNG
      </AccentBtn>
      <AccentBtn onClick={async () => {
        if (!pdfData) return
        const dir = await open({ directory: true })
        if (dir) {
          try {
            const images = await invoke<string[]>('pdf_to_images', { data: pdfData, output_dir: dir as string, format: 'jpg', dpi: 200 })
            showToast(`${images.length}ページをJPGに変換しました`)
          } catch (err) { showToast(`エラー: ${err}`) }
        }
      }}>
        PDF→JPG
      </AccentBtn>
      <AccentBtn onClick={async () => {
        if (!pdfData) return
        const path = await save({ defaultPath: 'output.txt', filters: [{ name: 'Text', extensions: ['txt'] }] })
        if (path) {
          try {
            await invoke('pdf_to_word', { data: pdfData, output_path: path })
            showToast('PDF→テキスト変換完了')
          } catch (err) { showToast(`エラー: ${err}`) }
        }
      }}>
        PDF→テキスト
      </AccentBtn>
      <AccentBtn onClick={async () => {
        if (!pdfData) return
        const path = await save({ defaultPath: 'output.csv', filters: [{ name: 'CSV', extensions: ['csv'] }] })
        if (path) {
          try {
            await invoke('pdf_to_excel', { data: pdfData, output_path: path })
            showToast('PDF→CSV変換完了')
          } catch (err) { showToast(`エラー: ${err}`) }
        }
      }}>
        PDF→CSV
      </AccentBtn>

      <SectionTitle>画像→PDF</SectionTitle>
      <AccentBtn onClick={async () => {
        const paths = await open({ filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }], multiple: true })
        if (paths) {
          const pathArray = Array.isArray(paths) ? paths : [paths]
          const outputPath = await save({ defaultPath: 'images.pdf', filters: [{ name: 'PDF', extensions: ['pdf'] }] })
          if (outputPath) {
            try {
              await invoke('images_to_pdf', { image_paths: pathArray, output_path: outputPath })
              showToast(`${pathArray.length}画像をPDFに変換しました`)
            } catch (err) { showToast(`エラー: ${err}`) }
          }
        }
      }}>
        画像→PDF
      </AccentBtn>

      <SectionTitle>HTML→PDF</SectionTitle>
      <AccentBtn onClick={async () => {
        const path = await open({ filters: [{ name: 'HTML', extensions: ['html', 'htm'] }], multiple: false })
        if (path) {
          const bytes = await invoke<number[]>('read_file_bytes', { path: path as string })
          const html = new TextDecoder().decode(new Uint8Array(bytes))
          const outputPath = await save({ defaultPath: 'output.pdf', filters: [{ name: 'PDF', extensions: ['pdf'] }] })
          if (outputPath) {
            try {
              await invoke('html_to_pdf', { html_content: html, output_path: outputPath })
              showToast('HTML→PDF変換完了')
            } catch (err) { showToast(`エラー: ${err}`) }
          }
        }
      }}>
        HTML→PDF
      </AccentBtn>

      <SectionTitle>PDF修復・解除</SectionTitle>
      <AccentBtn onClick={() => exec('repair_pdf', {})} style={{ background: 'var(--green)' }}>
        PDF修復
      </AccentBtn>
      <AccentBtn onClick={async () => {
        if (!pdfData) return
        const password = prompt('パスワードを入力:')
        if (password) {
          try {
            await invoke<number[]>('unlock_pdf', { data: pdfData, password })
            showToast('PDFロック解除完了')
          } catch (err) { showToast(`エラー: ${err}`) }
        }
      }}>
        パスワード解除
      </AccentBtn>

      <SectionTitle>ページ番号</SectionTitle>
      <AccentBtn onClick={() => exec('add_page_numbers', { position: 'bottom-center', font_size: 12, start_number: 1 })}>
        ページ番号追加（中央下）
      </AccentBtn>
      <AccentBtn onClick={() => exec('add_page_numbers', { position: 'bottom-right', font_size: 10, start_number: 1 })}>
        ページ番号追加（右下）
      </AccentBtn>

      <SectionTitle>高度なプロフェッショナル機能</SectionTitle>
      <AccentBtn onClick={async () => {
        const paths = await open({ filters: [{ name: 'Files', extensions: ['*'] }], multiple: true })
        if (paths) {
          const pathArray = Array.isArray(paths) ? paths : [paths]
          const outputPath = await save({ defaultPath: 'portfolio.pdf', filters: [{ name: 'PDF', extensions: ['pdf'] }] })
          if (outputPath) {
            try {
              await invoke('create_pdf_portfolio', { file_paths: pathArray, output_path: outputPath })
              showToast('ポートフォリオを作成しました')
            } catch (err) { showToast(`エラー: ${err}`) }
          }
        }
      }}>
        PDFポートフォリオ作成
      </AccentBtn>

      <AccentBtn onClick={() => exec('flatten_transparency', {})} style={{ background: 'var(--green)' }}>
        トランスペアレンシー平坦化
      </AccentBtn>

      <ToolsPDFXSection
        pdfData={pdfData}
        showToast={showToast}
        onPdfUpdate={onPdfUpdate}
      />

      <AccessibilitySection
        pdfData={pdfData}
        accessReport={accessReport}
        setAccessReport={setAccessReport}
        onPdfUpdate={onPdfUpdate}
        showToast={showToast}
      />

      <AccentBtn onClick={async () => {
        if (!pdfData) return
        try {
          const result = await invoke<{needs_cmyk_conversion: boolean}>('preview_color_separations', { data: pdfData })
          showToast(result.needs_cmyk_conversion ? 'CMYK変換推奨' : '色分解OK')
        } catch (err) { showToast(`エラー: ${err}`) }
      }}>
        色分解プレビュー
      </AccentBtn>

      <AccentBtn onClick={async () => {
        if (!pdfData) return
        const script = prompt('JavaScriptコードを入力:', 'app.alert("Hello from PDF!");')
        if (script) {
          try {
            await invoke<number[]>('embed_javascript', { data: pdfData, script })
            showToast('JavaScriptを埋め込みました')
          } catch (err) { showToast(`エラー: ${err}`) }
        }
      }}>
        JavaScript埋め込み
      </AccentBtn>

      <AccentBtn onClick={async () => {
        if (!pdfData) return
        const paths = await open({ filters: [{ name: 'PDF', extensions: ['pdf'] }], multiple: true })
        if (paths) {
          const pathArray = Array.isArray(paths) ? paths : [paths]
          try {
            const result = await invoke<{total_files: number}>('aggregate_form_data', { pdf_paths: pathArray })
            showToast(`${result.total_files}ファイルのフォームデータを集計`)
          } catch (err) { showToast(`エラー: ${err}`) }
        }
      }}>
        フォームデータ集計
      </AccentBtn>

      <AccentBtn onClick={async () => {
        const ids = await invoke<Array<{name: string; issuer: string}>>('list_digital_ids')
        showToast(`${ids.length}個のデジタルIDを検出`)
      }}>
        デジタルID一覧
      </AccentBtn>
    </div>
  )
}
