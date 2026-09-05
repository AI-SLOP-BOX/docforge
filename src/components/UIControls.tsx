import React from 'react'

export function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <div style={{
      fontSize: 10, fontWeight: 700, color: 'var(--text-muted)', textTransform: 'uppercase',
      letterSpacing: '0.08em', marginTop: 14, marginBottom: 8, paddingBottom: 3,
      borderBottom: '1px solid var(--border-subtle)', display: 'flex', alignItems: 'center', gap: 6,
    }}>{children}</div>
  )
}

export function Input({ value, onChange, placeholder }: { value: string; onChange: (v: string) => void; placeholder: string }) {
  return (
    <input
      value={value}
      onChange={e => onChange(e.target.value)}
      placeholder={placeholder}
      style={{
        width: '100%', padding: '6px 10px', background: 'var(--bg-0)',
        border: '1px solid var(--border)', borderRadius: 'var(--radius-sm)',
        color: 'var(--text)', fontSize: 12, marginBottom: 6,
        boxShadow: 'inset 0 1px 3px rgba(0,0,0,0.25)',
      }}
    />
  )
}

export function NumInput({ value, onChange, label }: { value: number; onChange: (v: number) => void; label: string }) {
  return (
    <div style={{ flex: 1, marginBottom: 6 }}>
      <label style={{ fontSize: 10, fontWeight: 600, color: 'var(--text-muted)', display: 'block', marginBottom: 2 }}>
        {label}
      </label>
      <div style={{ position: 'relative', display: 'flex', alignItems: 'center' }}>
        <input
          type="number"
          value={value}
          onChange={e => onChange(Number(e.target.value))}
          style={{
            width: '100%', padding: '5px 8px', background: 'var(--bg-0)',
            border: '1px solid var(--border)', borderRadius: 'var(--radius-sm)',
            color: 'var(--text)', fontSize: 12, fontWeight: 500,
            boxShadow: 'inset 0 1px 3px rgba(0,0,0,0.25)',
          }}
        />
      </div>
    </div>
  )
}

export function ColorInput({ value, onChange, label }: { value: string; onChange: (v: string) => void; label: string }) {
  return (
    <div style={{ flex: 1, marginBottom: 6 }}>
      <label style={{ fontSize: 10, fontWeight: 600, color: 'var(--text-muted)', display: 'block', marginBottom: 2 }}>
        {label}
      </label>
      <div style={{
        display: 'flex', alignItems: 'center', gap: 6, padding: '3px 6px',
        background: 'var(--bg-0)', border: '1px solid var(--border)', borderRadius: 'var(--radius-sm)',
      }}>
        <input
          type="color"
          value={value}
          onChange={e => onChange(e.target.value)}
          style={{
            width: 22, height: 22, border: 'none', borderRadius: 4, cursor: 'pointer',
            padding: 0, background: 'transparent',
          }}
        />
        <span style={{ fontSize: 11, fontFamily: 'monospace', color: 'var(--text-dim)' }}>
          {value.toUpperCase()}
        </span>
      </div>
    </div>
  )
}

export function SliderInput({ value, onChange, label, min, max, step }: {
  value: number; onChange: (v: number) => void; label: string; min: number; max: number; step: number
}) {
  return (
    <div style={{ marginBottom: 8 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 3 }}>
        <span style={{ fontSize: 10, fontWeight: 600, color: 'var(--text-muted)' }}>{label}</span>
        <span style={{ fontSize: 10, fontFamily: 'monospace', color: 'var(--accent)' }}>{value.toFixed(1)}</span>
      </div>
      <input
        type="range" min={min} max={max} step={step} value={value}
        onChange={e => onChange(Number(e.target.value))}
        style={{
          width: '100%', accentColor: 'var(--accent)', cursor: 'pointer', height: 4,
        }}
      />
    </div>
  )
}

export function AccentBtn({ children, onClick, disabled, style }: {
  children: React.ReactNode; onClick: () => void; disabled?: boolean; style?: React.CSSProperties
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      style={{
        width: '100%', padding: '6px 12px', marginTop: 5,
        background: disabled ? 'rgba(255,255,255,0.05)' : style?.background || 'var(--accent)',
        color: disabled ? 'var(--text-muted)' : (style?.color || '#ffffff'),
        border: '1px solid ' + (disabled ? 'transparent' : 'rgba(255,255,255,0.1)'),
        borderRadius: 'var(--radius-sm)', fontSize: 12, fontWeight: 600,
        boxShadow: disabled ? 'none' : '0 1px 3px rgba(0,0,0,0.3)',
        cursor: disabled ? 'not-allowed' : 'pointer',
        display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 6,
        ...style,
      }}
    >
      {children}
    </button>
  )
}

export function ToolBtn({ icon, onClick, disabled, title, shortcut }: {
  icon: React.ReactNode; onClick: () => void; disabled?: boolean; title?: string; shortcut?: string;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      title={shortcut ? `${title} (${shortcut})` : title}
      style={{
        height: 28, padding: '0 8px', display: 'flex', alignItems: 'center', gap: 5,
        border: '1px solid transparent', borderRadius: 'var(--radius-xs)',
        cursor: disabled ? 'not-allowed' : 'pointer',
        background: 'transparent', color: disabled ? 'var(--text-muted)' : 'var(--text)',
        opacity: disabled ? 0.35 : 1,
        transition: 'all 0.12s ease',
      }}
      onMouseEnter={(e) => {
        if (!disabled) {
          e.currentTarget.style.background = 'var(--bg-hover)'
          e.currentTarget.style.borderColor = 'var(--border)'
        }
      }}
      onMouseLeave={(e) => {
        if (!disabled) {
          e.currentTarget.style.background = 'transparent'
          e.currentTarget.style.borderColor = 'transparent'
        }
      }}
    >
      <span style={{ display: 'flex', alignItems: 'center', color: disabled ? 'inherit' : 'var(--text-dim)' }}>
        {icon}
      </span>
      {shortcut && (
        <span className="kbd-badge">{shortcut}</span>
      )}
    </button>
  )
}
