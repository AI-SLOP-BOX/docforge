import { useState } from 'react'
import type { View } from './types'
import Sidebar from './components/Sidebar'
import PDFEditorView from './views/PDFEditorView'
import ScannerView from './views/ScannerView'
import OCRView from './views/OCRView'
import { FileIcon, CameraIcon, TypeIcon } from './components/Icons'

export default function App() {
  const [currentView, setCurrentView] = useState<View>('pdf-editor')

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', width: '100vw', overflow: 'hidden' }}>
      <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
        <Sidebar currentView={currentView} onNavigate={setCurrentView} />
        <main style={{ flex: 1, overflow: 'hidden', display: 'flex', flexDirection: 'column' }}>
          {currentView === 'pdf-editor' && <PDFEditorView />}
          {currentView === 'scanner' && <ScannerView />}
          {currentView === 'ocr' && <OCRView />}
        </main>
      </div>

      {/* Mobile Bottom Navigation Bar (iPhone / Android Phone <= 640px) */}
      <nav
        className="mobile-bottom-nav"
        style={{
          display: 'none',
          height: 'calc(52px + var(--safe-bottom, 0px))',
          paddingBottom: 'var(--safe-bottom, 0px)',
          background: 'var(--bg-1)',
          borderTop: '1px solid var(--border)',
          alignItems: 'center',
          justifyContent: 'space-around',
          flexShrink: 0,
          zIndex: 100,
        }}
      >
        {([
          { view: 'pdf-editor', label: 'PDF編集', icon: <FileIcon size={20} /> },
          { view: 'scanner', label: 'スキャン', icon: <CameraIcon size={20} /> },
          { view: 'ocr', label: 'OCR変換', icon: <TypeIcon size={20} /> },
        ] as const).map(item => {
          const active = currentView === item.view
          return (
            <button
              key={item.view}
              onClick={() => setCurrentView(item.view)}
              style={{
                flex: 1,
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                gap: 3,
                background: 'transparent',
                color: active ? 'var(--accent)' : 'var(--text-muted)',
                padding: '6px 0',
              }}
            >
              {item.icon}
              <span style={{ fontSize: 10, fontWeight: active ? 600 : 500 }}>{item.label}</span>
            </button>
          )
        })}
      </nav>
    </div>
  )
}
