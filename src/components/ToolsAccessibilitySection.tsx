import React from 'react'
import { invoke } from '@tauri-apps/api/core'
import { AccentBtn } from './UIControls'

export interface AccessibilityReport {
  score: number
  issues: Array<{ severity: string; message: string }>
  has_tags: boolean
  has_title: boolean
  has_language: boolean
}

interface AccessibilitySectionProps {
  pdfData: number[] | null
  accessReport: AccessibilityReport | null
  setAccessReport: (report: AccessibilityReport | null) => void
  onPdfUpdate?: (data: number[]) => void
  showToast: (msg: string) => void
}

export const AccessibilitySection: React.FC<AccessibilitySectionProps> = ({
  pdfData,
  accessReport,
  setAccessReport,
  onPdfUpdate,
  showToast,
}) => {
  return (
    <>
      <AccentBtn
        onClick={async () => {
          if (!pdfData) return
          try {
            const result = await invoke<AccessibilityReport>('check_accessibility', { data: pdfData })
            setAccessReport(result)
          } catch (err) {
            showToast(`エラー: ${err}`)
          }
        }}
      >
        アクセシビリティ検査・診断
      </AccentBtn>

      {accessReport && (
        <div
          style={{
            padding: 12,
            borderRadius: 8,
            background: 'var(--bg-1)',
            border: `1px solid ${accessReport.score >= 80 ? 'var(--green)' : 'var(--danger, #e5484d)'}`,
            marginBottom: 12,
            fontSize: 11,
          }}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 6 }}>
            <span
              style={{
                fontWeight: 700,
                color: accessReport.score >= 80 ? 'var(--green)' : 'var(--danger, #e5484d)',
              }}
            >
              アクセシビリティ適合スコア: {accessReport.score} / 100点
            </span>
            <button
              onClick={() => setAccessReport(null)}
              style={{
                background: 'none',
                border: 'none',
                color: 'var(--text-muted)',
                cursor: 'pointer',
                fontSize: 12,
              }}
            >
              [x]
            </button>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: 2, marginBottom: 8, color: 'var(--text-muted)' }}>
            <div>タグ付き構造 (Marked): {accessReport.has_tags ? '[OK] 適合' : '[NG] 未設定 (タグなし)'}</div>
            <div>文書タイトル (Title): {accessReport.has_title ? '[OK] 設定済' : '[NG] 未設定'}</div>
            <div>文書言語 (Lang): {accessReport.has_language ? '[OK] 設定済' : '[NG] 未設定'}</div>
          </div>

          {accessReport.issues.length > 0 && (
            <div style={{ marginBottom: 10 }}>
              <div style={{ color: 'var(--danger, #e5484d)', fontWeight: 600, marginBottom: 2 }}>検出された問題:</div>
              {accessReport.issues.map((issue, idx) => (
                <div key={idx} style={{ color: 'var(--danger, #e5484d)', marginLeft: 8, lineHeight: 1.4 }}>
                  • {issue.message}
                </div>
              ))}
            </div>
          )}

          <AccentBtn
            onClick={async () => {
              if (!pdfData) return
              try {
                const fixed = await invoke<number[]>('fix_accessibility_issues', {
                  data: pdfData,
                  default_title: 'Accessible Document',
                  default_lang: 'ja-JP',
                })
                onPdfUpdate?.(fixed)
                setAccessReport(null)
                showToast('アクセシビリティ補正完了: 言語・タイトル・フォームツールチップを設定しました')
              } catch (err) {
                showToast(`修復エラー: ${err}`)
              }
            }}
            style={{ width: '100%', background: 'var(--green)', color: '#fff', fontSize: 11, padding: '5px 8px' }}
          >
            メタデータ・言語属性・ツールチップ補正
          </AccentBtn>
        </div>
      )}
    </>
  )
}
