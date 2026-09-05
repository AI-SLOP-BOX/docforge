import { useState, useCallback, useEffect, useMemo, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import PDFViewer, { InteractiveMode, TextBlock } from '../components/PDFViewer'
import {
  TypeIcon,
  EditIcon, AnnotateIcon, FormIcon, OrganizeIcon, FileIcon,
  LockIcon, ToolsIcon, FolderOpenIcon, SaveIcon, HighlightIcon, RedactIcon, ZapIcon, VectorPathIcon, ShieldCheckIcon
} from '../components/Icons'
import { CommandPalette, CommandItem } from '../components/CommandPalette'
import { EditPanel } from '../components/EditPanel'
import { AnnotatePanel } from '../components/AnnotatePanel'
import { FormCreatorPanel } from '../components/FormCreatorPanel'
import { OrganizePanel } from '../components/OrganizePanel'
import { PagesPanel } from '../components/PagesPanel'
import { SecurityPanel, SignatureInfo } from '../components/SecurityPanel'
import { TextEditPanel } from '../components/TextEditPanel'
import { ToolsPanel } from '../components/ToolsPanel'
import { EditorHeader } from '../components/EditorHeader'
import { SignatureVerificationModal } from '../components/SignatureVerificationModal'
import { useHistory } from '../hooks/useHistory'
import { useToast } from '../hooks/useToast'
import { formatError } from '../utils/errorHandler'
import { t } from '../utils/i18n'

import { DocumentService } from '../services/documentService'
import { buildEditorCommandItems } from '../components/editorCommands'

type Tab = 'edit' | 'annotate' | 'forms' | 'organize' | 'pages' | 'security' | 'text' | 'tools'

export default function PDFEditorView() {
  const { data: pdfData, setData: setPdfData, pushHistory, undo: fallbackUndo, redo: fallbackRedo, canUndo: fallbackCanUndo, canRedo: fallbackCanRedo } = useHistory(null, 30)
  const { toast, toastType, showToast, showError, showSuccess } = useToast(2800)

  const [docId, setDocId] = useState<string | null>(null)
  const docIdRef = useRef<string | null>(null)
  docIdRef.current = docId

  const [revision, setRevision] = useState(0)
  const [sessionCanUndo, setSessionCanUndo] = useState(false)
  const [sessionCanRedo, setSessionCanRedo] = useState(false)

  const [fileName, setFileName] = useState('')
  const [pageCount, setPageCount] = useState(0)
  const [currentPage, setCurrentPage] = useState(0)
  const [activeTab, setActiveTab] = useState<Tab | null>('edit')

  // Refresh history capability status whenever revision or docId changes
  const refreshHistoryStatus = useCallback(async (activeId: string) => {
    try {
      const status = await DocumentService.getHistoryStatus(activeId)
      setSessionCanUndo(status.can_undo)
      setSessionCanRedo(status.can_redo)
    } catch {
      setSessionCanUndo(false)
      setSessionCanRedo(false)
    }
  }, [])

  // Cleanup session ONLY when component truly unmounts
  useEffect(() => {
    return () => {
      if (docIdRef.current) {
        DocumentService.closeSession(docIdRef.current).catch(() => {})
      }
    }
  }, [])

  // Interactive canvas state
  const [interactiveMode, setInteractiveMode] = useState<InteractiveMode>('view')
  const [selectedTextBlock, setSelectedTextBlock] = useState<TextBlock | null>(null)

  // Signature verification inspection modal
  const [verifiedSignatures, setVerifiedSignatures] = useState<SignatureInfo[] | null>(null)

  // Watermark state
  const [watermarkText, setWatermarkText] = useState('CONFIDENTIAL')
  const [watermarkOpacity, setWatermarkOpacity] = useState(0.3)
  const [watermarkRotation, setWatermarkRotation] = useState(-45)
  const [watermarkFontSize, setWatermarkFontSize] = useState(48)
  const [watermarkColor, setWatermarkColor] = useState('#808080')

  // Annotation state
  const [annotationColor, setAnnotationColor] = useState('#FF0000')
  const [stickyNoteText, setStickyNoteText] = useState('')
  const [strokeWidth, setStrokeWidth] = useState(2)

  // Header/Footer state
  const [headerText, setHeaderText] = useState('')
  const [footerText, setFooterText] = useState('Page {page} of {total}')
  const [hfFontSize, setHfFontSize] = useState(10)

  // Bates state
  const [batesPrefix, setBatesPrefix] = useState('DOC')
  const [batesStart, setBatesStart] = useState(1)
  const [batesFontSize, setBatesFontSize] = useState(10)

  // Redact state
  const [redactColor, setRedactColor] = useState('#000000')
  const [redactSearchText, setRedactSearchText] = useState('')
  const [redactReplacement, setRedactReplacement] = useState('')

  const handleOpen = useCallback(async () => {
    try {
      const res = await DocumentService.openFileDialog()
      if (!res) return

      // If previous session exists, close it
      if (docIdRef.current) {
        await DocumentService.closeSession(docIdRef.current).catch(() => {})
      }

      // Initialize DocumentSession in Rust backend with per-doc lock
      const newDocId = await DocumentService.createSession(res.bytes)
      setDocId(newDocId)
      setRevision(r => r + 1)
      setPdfData(res.bytes)
      setFileName(res.name)
      pushHistory(res.bytes)

      await refreshHistoryStatus(newDocId)
      showSuccess(t().pdfLoaded)
    } catch (err) {
      showError(formatError(err, 'PDFの読み込みに失敗しました'))
    }
  }, [pushHistory, refreshHistoryStatus, setPdfData, showError, showSuccess])

  const handleSave = useCallback(async () => {
    if (!docId && !pdfData) return
    try {
      // If docId is active, serialize directly from backend session
      const bytesToSave = docId
        ? await DocumentService.getSessionBytes(docId)
        : (pdfData as number[])

      const path = await DocumentService.saveFileDialog(fileName, bytesToSave)
      if (!path) return
      showSuccess(t().savedSuccess)
    } catch (err) {
      showError(formatError(err, 'PDFの保存に失敗しました'))
    }
  }, [docId, pdfData, fileName, showError, showSuccess])

  const handleUndo = useCallback(async () => {
    if (docId) {
      try {
        const ok = await DocumentService.undo(docId)
        if (ok) {
          const currentBytes = await DocumentService.getSessionBytes(docId).catch(() => null)
          if (currentBytes) setPdfData(currentBytes)
          setRevision(r => r + 1)
          await refreshHistoryStatus(docId)
          showSuccess(t().undo)
        }
      } catch (err) {
        showError(formatError(err, 'Undoに失敗しました'))
      }
    } else {
      fallbackUndo()
    }
  }, [docId, fallbackUndo, refreshHistoryStatus, setPdfData, showError, showSuccess])

  const handleRedo = useCallback(async () => {
    if (docId) {
      try {
        const ok = await DocumentService.redo(docId)
        if (ok) {
          const currentBytes = await DocumentService.getSessionBytes(docId).catch(() => null)
          if (currentBytes) setPdfData(currentBytes)
          setRevision(r => r + 1)
          await refreshHistoryStatus(docId)
          showSuccess(t().redo)
        }
      } catch (err) {
        showError(formatError(err, 'Redoに失敗しました'))
      }
    } else {
      fallbackRedo()
    }
  }, [docId, fallbackRedo, refreshHistoryStatus, setPdfData, showError, showSuccess])

  const canUndo = docId ? sessionCanUndo : fallbackCanUndo
  const canRedo = docId ? sessionCanRedo : fallbackCanRedo

  const [isCommandPaletteOpen, setIsCommandPaletteOpen] = useState(false)

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey) {
        if (e.key === 'k' || e.key === 'K') {
          e.preventDefault()
          setIsCommandPaletteOpen(prev => !prev)
          return
        }
        if (e.key === 'o') { e.preventDefault(); handleOpen() }
        if (e.key === 's') { e.preventDefault(); handleSave() }
        if (e.key === 'z' && !e.shiftKey) { e.preventDefault(); handleUndo() }
        if ((e.key === 'z' && e.shiftKey) || e.key === 'y') { e.preventDefault(); handleRedo() }
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [handleOpen, handleSave, handleUndo, handleRedo])

  const exec = useCallback(async (cmd: string, args: Record<string, unknown>) => {
    if (!docId && !pdfData) return
    try {
      if (docId) {
        // Fast paths for rotation and page deletion using DocumentSession
        if (cmd === 'rotate_page') {
          const pIdx = (args.page_index as number) ?? currentPage
          const deg = (args.degrees as number) ?? 90
          await DocumentService.rotatePage(docId, pIdx, deg)
          const currentBytes = await DocumentService.getSessionBytes(docId).catch(() => null)
          if (currentBytes) setPdfData(currentBytes)
          setRevision(r => r + 1)
          await refreshHistoryStatus(docId)
          showSuccess(t().completed)
          return
        } else if (cmd === 'delete_page') {
          const pIdx = (args.page_index as number) ?? currentPage
          await DocumentService.deletePage(docId, pIdx)
          const currentBytes = await DocumentService.getSessionBytes(docId).catch(() => null)
          if (currentBytes) setPdfData(currentBytes)
          setRevision(r => r + 1)
          await refreshHistoryStatus(docId)
          showSuccess(t().completed)
          return
        }
      }

      // For standard command tools, run against current session bytes and update session in-place
      const currentBytes = docId ? await DocumentService.getSessionBytes(docId) : (pdfData as number[])
      const result = await invoke<number[]>(cmd, { data: currentBytes, ...args })

      if (docId) {
        // Update session in-place with FullSnapshot so Undo/Redo stack is preserved!
        await DocumentService.updateSessionBytes(docId, `Command ${cmd}`, result)
        setRevision(r => r + 1)
        await refreshHistoryStatus(docId)
      }
      pushHistory(result)
      showSuccess(t().completed)
    } catch (err) {
      showError(formatError(err, 'コマンド実行に失敗しました'))
    }
  }, [docId, pdfData, currentPage, pushHistory, refreshHistoryStatus, showError, showSuccess])

  // Canvas Rect draw handler
  const handleDrawRectComplete = useCallback(async (rect: { x: number; y: number; width: number; height: number; page: number }) => {
    if (!pdfData) return
    try {
      if (interactiveMode === 'draw-redact') {
        await exec('redact_area', {
          page_index: rect.page,
          x: rect.x,
          y: rect.y,
          width: rect.width,
          height: rect.height,
          color: redactColor,
        })
        setInteractiveMode('view')
        showSuccess(t().redactApplied)
      } else if (interactiveMode === 'draw-highlight') {
        await exec('add_highlight', {
          page_index: rect.page,
          x: rect.x,
          y: rect.y,
          width: rect.width,
          height: rect.height,
          color: annotationColor,
        })
        setInteractiveMode('view')
        showSuccess(t().highlightAdded)
      } else if (interactiveMode === 'draw-rect') {
        await exec('add_rectangle', {
          page_index: rect.page,
          x: rect.x,
          y: rect.y,
          width: rect.width,
          height: rect.height,
          stroke_color: annotationColor,
          fill_color: '#FFFFFF00',
          stroke_width: strokeWidth,
        })
        setInteractiveMode('view')
        showSuccess(t().rectAdded)
      }
    } catch (err) {
      showError(formatError(err, '注釈の追加に失敗しました'))
    }
  }, [pdfData, interactiveMode, redactColor, annotationColor, strokeWidth, exec, showError, showSuccess])

  // Move text block handler from canvas drag with full Session sync & Undo preservation
  const handleMoveTextBlock = useCallback(async (blockId: number, newX: number, newY: number) => {
    if (!docId && !pdfData) return
    try {
      const currentBytes = docId ? await DocumentService.getSessionBytes(docId) : (pdfData as number[])
      const result = await invoke<number[]>('move_text_block', {
        data: currentBytes,
        page_index: currentPage,
        block_id: blockId,
        new_x: newX,
        new_y: newY,
      })
      if (docId) {
        await DocumentService.updateSessionBytes(docId, `Move text block #${blockId}`, result)
        setRevision(r => r + 1)
        await refreshHistoryStatus(docId)
      }
      pushHistory(result)
      showSuccess(t().textMoved(blockId, newX, newY))
    } catch (err) {
      showError(formatError(err, 'テキスト移動に失敗しました'))
    }
  }, [docId, pdfData, currentPage, pushHistory, refreshHistoryStatus, showError, showSuccess])

  // Switch interactiveMode when changing tabs (toggle to collapse if already active)
  const handleSelectTab = (tab: Tab) => {
    if (activeTab === tab) {
      setActiveTab(null)
      setInteractiveMode('view')
      return
    }
    setActiveTab(tab)
    if (tab === 'text') {
      setInteractiveMode('select-text')
    } else {
      setInteractiveMode('view')
    }
  }

  // Handle PDF byte update from child components (Forms/TextEdit/Tools/Overlay) with session sync
  const handlePdfUpdate = useCallback(async (data: number[]) => {
    if (docId) {
      try {
        await DocumentService.updateSessionBytes(docId, 'Edit Document', data)
        setRevision(r => r + 1)
        await refreshHistoryStatus(docId)
      } catch (err) {
        console.error('Failed to sync updated bytes to session:', err)
        showError(formatError(err, 'セッションの同期に失敗しました'))
        throw err
      }
    }
    pushHistory(data)
  }, [docId, pushHistory, refreshHistoryStatus, showError])

  const handlePrint = useCallback(async () => {
    const target = docId || pdfData
    if (!target) return
    try {
      await DocumentService.printPdf(target)
    } catch (err) {
      showError(formatError(err, '印刷に失敗しました'))
    }
  }, [docId, pdfData, showError])

  // Command items for ⌘K Quick Launcher
  const commandItems = useMemo(() => {
    return buildEditorCommandItems({
      onOpen: handleOpen,
      onSave: handleSave,
      onSelectTab: handleSelectTab,
      onSetInteractiveMode: setInteractiveMode,
      onExec: exec,
      hasPdf: !!pdfData,
    })
  }, [handleOpen, handleSave, exec, pdfData])

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', background: 'var(--bg-0)' }}>
      {toast && (
        <div style={{
          position: 'fixed', top: 16, right: 16, zIndex: 1000,
          background: 'var(--bg-2)',
          border: `1px solid ${toastType === 'error' ? 'var(--red, #f85149)' : 'var(--accent)'}`,
          color: toastType === 'error' ? 'var(--red, #f85149)' : 'var(--accent)',
          padding: '10px 20px', borderRadius: 'var(--radius)',
          boxShadow: 'var(--shadow)', fontSize: 13,
          maxWidth: 420,
        }}>
          {toast}
        </div>
      )}

      {/* Signature Verification Dialog */}
      {verifiedSignatures && (
        <SignatureVerificationModal
          signatures={verifiedSignatures}
          onClose={() => setVerifiedSignatures(null)}
        />
      )}

      {/* Top Toolbar */}
      <EditorHeader
        onOpen={handleOpen}
        onSave={handleSave}
        onPrint={handlePrint}
        canSave={!!pdfData}
        undo={handleUndo}
        redo={handleRedo}
        canUndo={canUndo}
        canRedo={canRedo}
        interactiveMode={interactiveMode}
        setInteractiveMode={setInteractiveMode}
        fileName={fileName}
        pageCount={pageCount}
        currentPage={currentPage}
        onOpenCommandPalette={() => setIsCommandPaletteOpen(true)}
      />

      <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
        {/* Tab Sidebar */}
        <div style={{
          width: 124, background: 'var(--bg-1)', borderRight: '1px solid var(--border)',
          display: 'flex', flexDirection: 'column', flexShrink: 0, padding: '8px 0',
        }}>
          {([
            ['edit', <EditIcon size={14} />, t().tabEdit],
            ['annotate', <AnnotateIcon size={14} />, t().tabAnnotate],
            ['forms', <FormIcon size={14} />, t().tabForms],
            ['organize', <OrganizeIcon size={14} />, t().tabOrganize],
            ['pages', <FileIcon size={14} />, t().tabPages],
            ['security', <LockIcon size={14} />, t().tabSecurity],
            ['text', <TypeIcon size={14} />, t().tabText],
            ['tools', <ToolsIcon size={14} />, t().tabTools],
          ] as const).map(([tab, icon, label]) => (
            <button
              key={tab}
              onClick={() => handleSelectTab(tab)}
              className={`editor-tab-btn ${activeTab === tab ? 'active' : ''}`}
            >
              <span style={{ display: 'flex', alignItems: 'center' }}>{icon}</span>
              <span>{label}</span>
            </button>
          ))}
        </div>

        {/* Tool Panel (Contextual Drawer) */}
        {activeTab && (
          <div style={{
            width: 290, background: 'var(--bg-1)', borderRight: '1px solid var(--border)',
            padding: 12, overflowY: 'auto', flexShrink: 0,
            transition: 'width 0.2s ease',
          }}>
            {activeTab === 'edit' && (
              <EditPanel exec={exec} pdfData={pdfData} showToast={showToast} currentPage={currentPage} />
            )}
            {activeTab === 'annotate' && (
              <AnnotatePanel
                exec={exec}
                pdfData={pdfData}
                docId={docId}
                annotationColor={annotationColor}
                setAnnotationColor={setAnnotationColor}
                stickyNoteText={stickyNoteText}
                setStickyNoteText={setStickyNoteText}
                strokeWidth={strokeWidth}
                setStrokeWidth={setStrokeWidth}
                onActivateDraw={(mode) => setInteractiveMode(mode)}
                currentPage={currentPage}
              />
            )}
            {activeTab === 'forms' && (
              <FormCreatorPanel
                pdfData={pdfData}
                docId={docId}
                currentPage={currentPage}
                exec={exec}
                showToast={showToast}
                onPdfUpdate={handlePdfUpdate}
              />
            )}
            {activeTab === 'organize' && (
              <OrganizePanel exec={exec} />
            )}
            {activeTab === 'pages' && (
              <PagesPanel
                watermarkText={watermarkText} setWatermarkText={setWatermarkText}
                watermarkOpacity={watermarkOpacity} setWatermarkOpacity={setWatermarkOpacity}
                watermarkRotation={watermarkRotation} setWatermarkRotation={setWatermarkRotation}
                watermarkFontSize={watermarkFontSize} setWatermarkFontSize={setWatermarkFontSize}
                watermarkColor={watermarkColor} setWatermarkColor={setWatermarkColor}
                headerText={headerText} setHeaderText={setHeaderText}
                footerText={footerText} setFooterText={setFooterText}
                hfFontSize={hfFontSize} setHfFontSize={setHfFontSize}
                batesPrefix={batesPrefix} setBatesPrefix={setBatesPrefix}
                batesStart={batesStart} setBatesStart={setBatesStart}
                batesFontSize={batesFontSize} setBatesFontSize={setBatesFontSize}
                exec={exec}
              />
            )}
            {activeTab === 'security' && (
              <SecurityPanel
                exec={exec}
                pdfData={pdfData}
                docId={docId}
                showToast={showToast}
                onInspectSignatures={(sigs: SignatureInfo[]) => setVerifiedSignatures(sigs)}
              />
            )}
            {activeTab === 'text' && (
              <TextEditPanel
                pdfData={pdfData}
                docId={docId}
                exec={exec}
                showToast={showToast}
                onPdfUpdate={handlePdfUpdate}
                selectedBlockFromCanvas={selectedTextBlock}
                currentPage={currentPage}
              />
            )}
            {activeTab === 'tools' && (
              <ToolsPanel
                exec={exec}
                pdfData={pdfData}
                docId={docId}
                redactColor={redactColor} setRedactColor={setRedactColor}
                redactSearchText={redactSearchText} setRedactSearchText={setRedactSearchText}
                redactReplacement={redactReplacement} setRedactReplacement={setRedactReplacement}
                showToast={showToast}
                onActivateDrawRedact={() => setInteractiveMode('draw-redact')}
                onPdfUpdate={handlePdfUpdate}
              />
            )}
          </div>
        )}

        {/* PDF Viewer */}
        <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
          {pdfData ? (
            <PDFViewer
              pdfData={pdfData}
              docId={docId}
              revision={revision}
              currentPage={currentPage}
              onPageCountChange={setPageCount}
              onPageChange={setCurrentPage}
              interactiveMode={interactiveMode}
              selectedTextBlockId={selectedTextBlock?.id}
              onSelectTextBlock={setSelectedTextBlock}
              onMoveTextBlock={handleMoveTextBlock}
              onDrawRectComplete={handleDrawRectComplete}
              onPdfUpdate={handlePdfUpdate}
            />
          ) : (
            <div style={{
              flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center',
              background: '#1a1a2e',
            }}>
              <div style={{ textAlign: 'center', color: '#666' }}>
                <div style={{ marginBottom: 24, opacity: 0.25, display: 'flex', justifyContent: 'center' }}>
                  <FileIcon size={64} color="var(--text-dim)" />
                </div>
                <p style={{ fontSize: 16 }}>{t().noFileLoaded}</p>
                <button onClick={handleOpen} style={{
                  marginTop: 16, padding: '12px 32px', background: 'var(--accent)',
                  color: 'var(--bg-0)', border: 'none', borderRadius: 'var(--radius)',
                  fontSize: 14, fontWeight: 600, cursor: 'pointer',
                }}>
                  {t().openFileBtn}
                </button>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* ⌘K Command Palette Modal */}
      <CommandPalette
        isOpen={isCommandPaletteOpen}
        onClose={() => setIsCommandPaletteOpen(false)}
        commands={commandItems}
      />
    </div>
  )
}


