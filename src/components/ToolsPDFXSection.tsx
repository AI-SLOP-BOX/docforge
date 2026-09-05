import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { SectionTitle, AccentBtn } from './UIControls'

export interface PDFXReport {
  is_compliant: boolean
  standard: string
  output_condition: string
  passed_checks: string[]
  violations: string[]
}

interface ToolsPDFXSectionProps {
  pdfData: number[] | null
  showToast: (msg: string) => void
  onPdfUpdate?: (data: number[]) => void
}

export function ToolsPDFXSection({ pdfData, showToast, onPdfUpdate }: ToolsPDFXSectionProps) {
  const [pdfxReport, setPdfxReport] = useState<PDFXReport | null>(null)

  return (
    <div>
      <SectionTitle>印刷標準規格（ISO PDF/X準拠）</SectionTitle>
      <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 6, lineHeight: 1.5 }}>
        商業印刷・出版入稿に必須の ISO 15930 準拠認証およびプロファイル変換
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 6, marginBottom: 6 }}>
        <AccentBtn
          onClick={async () => {
            if (!pdfData) return
            try {
              const res = await invoke<number[]>('convert_to_pdfx_standard', {
                data: pdfData,
                standard: 'PDF/X-1a:2001',
                output_intent: 'Japan Color 2001 Coated'
              })
              onPdfUpdate?.(res)
              showToast('PDF/X-1a:2001 準拠変換完了 (CMYK/平坦化/TrimBox/OutputIntent適用)')
            } catch (err) {
              showToast(`PDF/X-1a変換エラー: ${err}`)
            }
          }}
          style={{ background: 'var(--green)' }}
        >
          PDF/X-1a (CMYK厳格)
        </AccentBtn>
        <AccentBtn
          onClick={async () => {
            if (!pdfData) return
            try {
              const res = await invoke<number[]>('convert_to_pdfx_standard', {
                data: pdfData,
                standard: 'PDF/X-4:2010',
                output_intent: 'Japan Color 2001 Coated'
              })
              onPdfUpdate?.(res)
              showToast('PDF/X-4:2010 準拠変換完了 (透明・RGB/OutputIntent適用)')
            } catch (err) {
              showToast(`PDF/X-4変換エラー: ${err}`)
            }
          }}
          style={{ background: 'var(--accent)' }}
        >
          PDF/X-4 (透明・RGB対応)
        </AccentBtn>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 6, marginBottom: 12 }}>
        <AccentBtn
          onClick={async () => {
            if (!pdfData) return
            try {
              const report = await invoke<PDFXReport>('validate_pdfx_compliance', {
                data: pdfData,
                target_standard: 'PDF/X-1a'
              })
              setPdfxReport(report)
            } catch (err) {
              showToast(`PDF/X-1a検証エラー: ${err}`)
            }
          }}
          style={{ background: 'var(--bg-2)', color: 'var(--text)', border: '1px solid var(--border)', fontSize: 11 }}
        >
          X-1a 適合検証
        </AccentBtn>
        <AccentBtn
          onClick={async () => {
            if (!pdfData) return
            try {
              const report = await invoke<PDFXReport>('validate_pdfx_compliance', {
                data: pdfData,
                target_standard: 'PDF/X-4'
              })
              setPdfxReport(report)
            } catch (err) {
              showToast(`PDF/X-4検証エラー: ${err}`)
            }
          }}
          style={{ background: 'var(--bg-2)', color: 'var(--text)', border: '1px solid var(--border)', fontSize: 11 }}
        >
          X-4 適合検証
        </AccentBtn>
      </div>

      {pdfxReport && (
        <div style={{
          padding: 12,
          borderRadius: 8,
          background: 'var(--bg-1)',
          border: `1px solid ${pdfxReport.is_compliant ? 'var(--green)' : 'var(--danger, #e5484d)'}`,
          marginBottom: 14,
          fontSize: 11
        }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8 }}>
            <span style={{ fontWeight: 600, color: pdfxReport.is_compliant ? 'var(--green)' : 'var(--danger, #e5484d)' }}>
              {pdfxReport.is_compliant ? `ISO準拠合格: ${pdfxReport.standard}` : `ISO不適合: ${pdfxReport.standard}`}
            </span>
            <button
              onClick={() => setPdfxReport(null)}
              style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer', fontSize: 12 }}
            >
              [x]
            </button>
          </div>
          <div style={{ color: 'var(--text-muted)', marginBottom: 6 }}>
            出力インテント: {pdfxReport.output_condition}
          </div>
          {pdfxReport.violations.length > 0 && (
            <div style={{ marginBottom: 6 }}>
              <div style={{ color: 'var(--danger, #e5484d)', fontWeight: 600, marginBottom: 2 }}>不適合項目:</div>
              {pdfxReport.violations.map((v, i) => (
                <div key={i} style={{ color: 'var(--danger, #e5484d)', marginLeft: 8, lineHeight: 1.4 }}>
                  - {v}
                </div>
              ))}
            </div>
          )}
          <div>
            <div style={{ color: 'var(--green)', fontWeight: 600, marginBottom: 2 }}>合格項目:</div>
            {pdfxReport.passed_checks.map((p, i) => (
              <div key={i} style={{ color: 'var(--text-muted)', marginLeft: 8, lineHeight: 1.4 }}>
                [OK] {p}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}
