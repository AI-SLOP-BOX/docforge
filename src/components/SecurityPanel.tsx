import { useState } from 'react'
import { DocumentService } from '../services/documentService'
import { Input, AccentBtn } from './UIControls'

export interface SignatureInfo {
  name: string
  signer: string
  reason: string
  status: string
  timestamp: string
  aatl_verified?: boolean
  trust_level?: string
  certificate_issuer?: string
  revocation_check?: string
  integrity_verified?: boolean
}

export function SecurityPanel({
  exec,
  showToast,
  onInspectSignatures,
  pdfData,
  docId,
}: {
  exec: Function
  showToast: (msg: string) => void
  onInspectSignatures?: (sigs: SignatureInfo[]) => void
  pdfData: number[] | null
  docId?: string | null
}) {
  const [password, setPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [signerName, setSignerName] = useState('')
  const [signReason, setSignReason] = useState('')

  const handleVerify = async () => {
    const target = docId || pdfData
    if (!target) return
    try {
      const result = await DocumentService.verifySignatures(target)
      if (result && result.signatures) {
        onInspectSignatures?.(result.signatures)
        showToast(`${result.signatures.length}件の署名を検証しました`)
      } else {
        onInspectSignatures?.([])
        showToast('署名は検出されませんでした')
      }
    } catch (err) {
      showToast(`検証エラー: ${err}`)
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      {/* Card 1: Password Encryption */}
      <div className="inspector-card">
        <div className="inspector-card-header">
          <span>パスワード暗号化</span>
          <span style={{ fontSize: 9, color: 'var(--red)', fontWeight: 600 }}>AES-128</span>
        </div>
        <div className="inspector-card-desc">PDF閲覧にパスワード保護を設定</div>
        <input
          type="password"
          value={password}
          onChange={e => setPassword(e.target.value)}
          placeholder="パスワードを入力"
          style={{
            width: '100%', padding: '6px 10px', background: 'var(--bg-0)',
            border: '1px solid var(--border)', borderRadius: 'var(--radius-sm)',
            color: 'var(--text)', fontSize: 12, marginBottom: 6,
            boxShadow: 'inset 0 1px 3px rgba(0,0,0,0.25)'
          }}
        />
        <input
          type="password"
          value={confirmPassword}
          onChange={e => setConfirmPassword(e.target.value)}
          placeholder="確認のため再入力"
          style={{
            width: '100%', padding: '6px 10px', background: 'var(--bg-0)',
            border: '1px solid var(--border)', borderRadius: 'var(--radius-sm)',
            color: 'var(--text)', fontSize: 12, marginBottom: 6,
            boxShadow: 'inset 0 1px 3px rgba(0,0,0,0.25)'
          }}
        />
        <AccentBtn
          onClick={() => {
            if (password !== confirmPassword) {
              showToast('パスワードが一致しません')
              return
            }
            exec('protect_pdf', { password })
          }}
          disabled={!password || password !== confirmPassword}
        >
          暗号化を実行
        </AccentBtn>
      </div>

      {/* Card 2: Digital Signatures */}
      <div className="inspector-card">
        <div className="inspector-card-header">
          <span>PKCS#7 デジタル署名</span>
          <span style={{ fontSize: 9, color: 'var(--green)', fontWeight: 600 }}>AUTH</span>
        </div>
        <div className="inspector-card-desc">電子署名フィールドと理由を付与</div>
        <Input value={signerName} onChange={setSignerName} placeholder="署名者名 (例: Taro Yamada)" />
        <Input value={signReason} onChange={setSignReason} placeholder="署名理由 (例: 承認済み)" />
        <AccentBtn
          onClick={() => exec('add_digital_signature', { page_index: 0, x: 400, y: 50, width: 150, height: 60, signer_name: signerName, reason: signReason })}
          disabled={!signerName}
          style={{ marginTop: 6 }}
        >
          デジタル署名を追加
        </AccentBtn>
        <AccentBtn
          onClick={handleVerify}
          style={{ marginTop: 6, background: 'rgba(46, 160, 67, 0.15)', color: '#2ea043', border: '1px solid rgba(46, 160, 67, 0.4)' }}
        >
          署名検証インスペクターを開く
        </AccentBtn>
      </div>
    </div>
  )
}
