import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { SectionTitle, AccentBtn } from './UIControls'
import { ShieldCheckIcon, SearchIcon, ZapIcon } from './Icons'

export interface CompareDiffItem {
  page: usize
  kind: string
  original_text: string
  revised_text: string
  x: number
  y: number
  width: number
  height: number
}

type usize = number

export interface CompareReport {
  total_pages_original: number
  total_pages_revised: number
  total_changes: number
  changes_added: number
  changes_deleted: number
  changes_modified: number
  diffs: CompareDiffItem[]
}

interface ToolsAdvancedEngineeringSectionProps {
  pdfData: number[] | null
  showToast: (msg: string) => void
  onPdfUpdate?: (data: number[]) => void
}

export function ToolsAdvancedEngineeringSection({
  pdfData,
  showToast,
  onPdfUpdate,
}: ToolsAdvancedEngineeringSectionProps) {
  const [busy, setBusy] = useState(false)
  const [compareReport, setCompareReport] = useState<CompareReport | null>(null)

  const handleAutoRepair = async () => {
    if (!pdfData || busy) return
    setBusy(true)
    try {
      const repaired = await invoke<number[]>('repair_corrupt_pdf', { data: pdfData })
      onPdfUpdate?.(repaired)
      showToast('破損・XRef障害の自動修復が完了しました (PDF構造再構築済)')
    } catch (err) {
      showToast(`修復エラー: ${err}`)
    } finally {
      setBusy(false)
    }
  }

  const handleScanEnhance = async (deskew: boolean, removeBleed: boolean, binarize: boolean) => {
    if (!pdfData || busy) return
    setBusy(true)
    try {
      const enhanced = await invoke<number[]>('enhance_scanned_pdf', {
        data: pdfData,
        options: {
          deskew,
          remove_bleedthrough: removeBleed,
          binarize_text: binarize,
          contrast_boost: 1.3,
        }
      })
      onPdfUpdate?.(enhanced)
      showToast('スキャン画像の美化処理完了 (傾き補正・裏写り除去適用)')
    } catch (err) {
      showToast(`スキャン美化エラー: ${err}`)
    } finally {
      setBusy(false)
    }
  }

  const handleCompareWith = async () => {
    if (!pdfData || busy) return
    const path = await open({
      filters: [{ name: 'PDF', extensions: ['pdf'] }],
      multiple: false,
    })
    if (!path) return
    setBusy(true)
    try {
      const otherBytes = await invoke<number[]>('read_file_bytes', { path: path as string })
      const report = await invoke<CompareReport>('compare_pdf_documents', {
        original: pdfData,
        revised: otherBytes,
      })
      setCompareReport(report)
      showToast(`2文書の比較完了: ${report.total_changes}件の差異を検出`)
    } catch (err) {
      showToast(`比較エラー: ${err}`)
    } finally {
      setBusy(false)
    }
  }

  return (
    <div>
      <SectionTitle>破損PDFの自動修復・再構築（PDF Repair）</SectionTitle>
      <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 8, lineHeight: 1.5 }}>
        不正XRef・オフセットズレ・構文エラーのあるPDFをバイナリ全走査し、正常なCatalog/Pages構造を再構築します
      </div>
      <AccentBtn
        onClick={handleAutoRepair}
        disabled={busy || !pdfData}
        style={{ background: '#2ea043', color: '#fff', marginBottom: 16 }}
      >
        <ShieldCheckIcon size={14} /> 破損PDFを修復・再構築
      </AccentBtn>

      <SectionTitle>スキャン書類の美化・傾き補正（Scan Enhancer）</SectionTitle>
      <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 8, lineHeight: 1.5 }}>
        斜めスキャンの自動回転補正（Deskew）および適応的裏写り・影・黄ばみの純白化除去
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 6, marginBottom: 16 }}>
        <AccentBtn
          onClick={() => handleScanEnhance(true, true, false)}
          disabled={busy || !pdfData}
          style={{ background: 'var(--bg-2)', color: 'var(--text)', border: '1px solid var(--border)' }}
        >
          <ZapIcon size={13} /> 傾き補正＋裏写り除去
        </AccentBtn>
        <AccentBtn
          onClick={() => handleScanEnhance(true, true, true)}
          disabled={busy || !pdfData}
          style={{ background: 'var(--bg-2)', color: 'var(--text)', border: '1px solid var(--border)' }}
        >
          文字くっきり2値化
        </AccentBtn>
      </div>

      <SectionTitle>セマンティック2文書比較（Compare Files）</SectionTitle>
      <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 8, lineHeight: 1.5 }}>
        別バージョンのPDFを選択し、テキスト変更・追加・削除をグラフィカルに自動検出
      </div>
      <AccentBtn
        onClick={handleCompareWith}
        disabled={busy || !pdfData}
        style={{ background: 'linear-gradient(135deg, #8957e5 0%, #a371f7 100%)', color: '#fff', marginBottom: 12 }}
      >
        <SearchIcon size={14} /> 他のPDFと比較して差分検出
      </AccentBtn>

      {compareReport && (
        <div style={{
          padding: 12,
          borderRadius: 8,
          background: 'var(--bg-1)',
          border: '1px solid var(--border)',
          marginBottom: 16,
          fontSize: 11,
        }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 6 }}>
            <b>比較レポート（検出差分: {compareReport.total_changes}件）</b>
            <button onClick={() => setCompareReport(null)} style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer' }}>[x]</button>
          </div>
          <div style={{ display: 'flex', gap: 12, marginBottom: 8, color: 'var(--text-muted)' }}>
            <span style={{ color: '#2ea043' }}>追加: {compareReport.changes_added}</span>
            <span style={{ color: '#da3633' }}>削除: {compareReport.changes_deleted}</span>
            <span style={{ color: '#f0883e' }}>変更: {compareReport.changes_modified}</span>
          </div>
          <div style={{ maxHeight: 180, overflowY: 'auto' }}>
            {compareReport.diffs.map((d, i) => (
              <div key={i} style={{ padding: '4px 0', borderBottom: '1px solid var(--border-subtle)' }}>
                <span style={{
                  padding: '1px 4px',
                  borderRadius: 3,
                  fontSize: 9,
                  marginRight: 6,
                  background: d.kind === 'added' ? 'rgba(46,160,67,0.2)' : d.kind === 'deleted' ? 'rgba(218,54,51,0.2)' : 'rgba(240,136,62,0.2)',
                  color: d.kind === 'added' ? '#2ea043' : d.kind === 'deleted' ? '#da3633' : '#f0883e',
                }}>
                  P.{d.page + 1} {d.kind}
                </span>
                <span>{d.kind === 'modified' ? `${d.original_text} -> ${d.revised_text}` : (d.revised_text || d.original_text)}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}
