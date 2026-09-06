import React from 'react'
import type { View } from '../types'
import { FileIcon, CameraIcon, TypeIcon, ZapIcon } from './Icons'

interface Props {
  currentView: View
  onNavigate: (view: View) => void
}

const navItems: { view: View; label: string; icon: React.ReactNode; desc: string }[] = [
  { view: 'pdf-editor', label: 'PDF Editor', icon: <FileIcon size={18} />, desc: '結合・分割・編集' },
  { view: 'scanner', label: 'Scanner', icon: <CameraIcon size={18} />, desc: 'スキャン補正' },
  { view: 'ocr', label: 'OCR / EPUB', icon: <TypeIcon size={18} />, desc: '文字認識・変換' },
]

export default function Sidebar({ currentView, onNavigate }: Props) {
  return (
    <aside
      className="desktop-sidebar desktop-only tablet-compact"
      style={{
        width: 230,
        background: 'var(--bg-1)',
        borderRight: '1px solid var(--border)',
        display: 'flex',
        flexDirection: 'column',
        flexShrink: 0,
        userSelect: 'none',
      }}
    >
      {/* Brand Header */}
      <div style={{
        padding: '16px 16px 14px',
        borderBottom: '1px solid var(--border)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <div style={{
            width: 32, height: 32, borderRadius: 'var(--radius-sm)',
            overflow: 'hidden',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            boxShadow: '0 3px 12px rgba(47, 129, 247, 0.35)'
          }}>
            <img src="/favicon.png" alt="Nagisa PDF" style={{ width: '100%', height: '100%', objectFit: 'cover' }} />
          </div>
          <div>
            <div style={{ fontSize: 14, fontWeight: 700, color: 'var(--text)', letterSpacing: '-0.02em', display: 'flex', alignItems: 'center', gap: 6 }}>
              Nagisa PDF
              <span style={{ fontSize: 9, fontWeight: 700, padding: '1px 5px', borderRadius: 4, background: 'rgba(47, 129, 247, 0.15)', color: 'var(--accent)' }}>
                PRO
              </span>
            </div>
            <div style={{ fontSize: 10, color: 'var(--text-muted)', letterSpacing: '0.01em' }}>Local Engine</div>
          </div>
        </div>
      </div>

      {/* Navigation List */}
      <nav style={{ padding: '10px 8px', flex: 1, display: 'flex', flexDirection: 'column', gap: 3 }}>
        <div style={{ padding: '4px 10px', fontSize: 10, fontWeight: 600, color: 'var(--text-muted)', textTransform: 'uppercase', letterSpacing: '0.06em' }}>
          Workspace
        </div>
        {navItems.map(({ view, label, icon, desc }) => {
          const active = currentView === view
          return (
            <button
              key={view}
              onClick={() => onNavigate(view)}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 11,
                padding: '8px 12px',
                border: 'none',
                borderRadius: 'var(--radius-sm)',
                background: active ? 'var(--bg-active)' : 'transparent',
                color: active ? 'var(--accent)' : 'var(--text-dim)',
                boxShadow: active ? 'inset 0 0 0 1px rgba(88, 166, 255, 0.3)' : 'none',
                textAlign: 'left',
                transition: 'all 0.12s cubic-bezier(0.16, 1, 0.3, 1)',
                width: '100%',
              }}
              onMouseEnter={(e) => {
                if (!active) e.currentTarget.style.background = 'var(--bg-hover)'
              }}
              onMouseLeave={(e) => {
                if (!active) e.currentTarget.style.background = 'transparent'
              }}
            >
              <span style={{
                width: 22, height: 22, display: 'flex', alignItems: 'center', justifyContent: 'center',
                color: active ? 'var(--accent)' : 'var(--text-muted)'
              }}>
                {icon}
              </span>
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ fontSize: 13, fontWeight: active ? 600 : 500, color: active ? 'var(--text)' : 'inherit' }}>
                  {label}
                </div>
                <div style={{ fontSize: 10, color: 'var(--text-muted)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                  {desc}
                </div>
              </div>
            </button>
          )
        })}
      </nav>

      {/* Footer Status Badge */}
      <div style={{
        padding: '12px 14px',
        borderTop: '1px solid var(--border)',
        fontSize: 11,
        color: 'var(--text-muted)',
        background: 'rgba(0,0,0,0.15)',
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
          <span style={{ width: 6, height: 6, borderRadius: '50%', background: 'var(--green)', boxShadow: '0 0 6px var(--green)' }} />
          <span style={{ fontSize: 11, fontWeight: 600, color: 'var(--text-dim)' }}>Native Rust Core</span>
        </div>
        <div style={{ fontSize: 10, color: 'var(--text-muted)', lineHeight: 1.4 }}>
          Zero Cloud · No Watermark · 100% Local
        </div>
      </div>
    </aside>
  )
}
