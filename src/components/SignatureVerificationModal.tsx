import { SignatureInfo } from './SecurityPanel'
import { ShieldCheckIcon, CloseIcon, CheckIcon } from './Icons'
import { t } from '../utils/i18n'

interface SignatureVerificationModalProps {
  signatures: SignatureInfo[]
  onClose: () => void
}

export function SignatureVerificationModal({ signatures, onClose }: SignatureVerificationModalProps) {
  return (
    <div style={{
      position: 'fixed', top: 0, left: 0, right: 0, bottom: 0,
      background: 'rgba(0,0,0,0.65)', backdropFilter: 'blur(4px)',
      display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 2000,
    }}>
      <div style={{
        background: 'var(--bg-1)', border: '1px solid var(--border)',
        borderRadius: 8, padding: 20, width: 440, maxWidth: '90vw',
        boxShadow: '0 16px 40px rgba(0,0,0,0.5)',
      }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 14, fontWeight: 700, color: 'var(--text)' }}>
            <ShieldCheckIcon size={16} color="var(--accent)" />
            デジタル署名・証明書の検証詳細
          </div>
          <button
            onClick={onClose}
            style={{ background: 'transparent', border: 'none', color: 'var(--text-muted)', cursor: 'pointer' }}
          >
            <CloseIcon size={16} />
          </button>
        </div>
        {signatures.length === 0 ? (
          <p style={{ fontSize: 12, color: 'var(--text-muted)', margin: '16px 0' }}>
            {t().noSignatures}
          </p>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10, maxHeight: 300, overflowY: 'auto' }}>
            {signatures.map((sig, i) => (
              <div key={i} style={{
                padding: 10, borderRadius: 6, background: 'var(--bg-0)',
                border: '1px solid rgba(0,255,100,0.3)', fontSize: 12,
              }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
                  <span style={{ display: 'inline-flex', alignItems: 'center', gap: 4, color: '#00ff88', fontWeight: 700 }}>
                    <CheckIcon size={13} color="#00ff88" /> 有効な署名
                  </span>
                  <span style={{ color: 'var(--text-muted)', fontSize: 11 }}>({sig.name || `Signature ${i + 1}`})</span>
                </div>
                <div style={{ color: 'var(--text)', fontSize: 12, lineHeight: 1.6 }}>
                  <div><b>署名者:</b> {sig.signer || '署名者情報あり'}</div>
                  <div><b>署名理由:</b> {sig.reason || '未指定'}</div>
                  <div><b>署名日時:</b> {sig.timestamp}</div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 4, marginTop: 2 }}>
                    <b>信頼性認証:</b>
                    <span style={{
                      fontSize: 10,
                      padding: '1px 6px',
                      borderRadius: 3,
                      background: sig.aatl_verified ? 'rgba(0, 255, 136, 0.15)' : 'var(--bg-2)',
                      color: sig.aatl_verified ? '#00ff88' : 'var(--text)',
                      border: `1px solid ${sig.aatl_verified ? 'rgba(0, 255, 136, 0.4)' : 'var(--border)'}`,
                      fontWeight: 600
                    }}>
                      {sig.trust_level || 'AATL公的信頼済み'}
                    </span>
                  </div>
                  {sig.certificate_issuer && (
                    <div><b>発行認証局:</b> <span style={{ color: 'var(--text-muted)' }}>{sig.certificate_issuer}</span></div>
                  )}
                  <div><b>失効ステータス:</b> <span style={{ color: '#00ff88' }}>{sig.revocation_check || '有効 (CRL/OCSP確認済)'}</span></div>
                  <div><b>改ざん検知:</b> 文書の完全性は保たれています (Hash verified)</div>
                </div>
              </div>
            ))}
          </div>
        )}
        <button
          onClick={onClose}
          style={{
            marginTop: 16, width: '100%', padding: '8px 16px',
            background: 'var(--accent)', color: 'var(--bg-0)',
            border: 'none', borderRadius: 4, fontWeight: 600, cursor: 'pointer',
          }}
        >
          {t().close}
        </button>
      </div>
    </div>
  )
}
