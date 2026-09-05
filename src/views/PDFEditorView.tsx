import { useState, useCallback, useEffect } from 'react'
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

type Tab = 'edit' | 'annotate' | 'forms' | 'organize' | 'pages' | 'security' | 'text' | 'tools'

export default function PDFEditorView() {
  const { data: pdfData, setData: setPdfData, pushHistory, undo, redo, canUndo, canRedo } = useHistory(null, 30)
  const { toast, toastType, showToast, showError, showSuccess } = useToast(2800)

  const [fileName, setFileName] = useState('')
  const [pageCount, setPageCount] = useState(0)
  const [currentPage, setCurrentPage] = useState(0)
  const [activeTab, setActiveTab] = useState<Tab | null>('edit')

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
      const path = await open({
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
        multiple: false,
      })
      if (!path) return
      const bytes = await invoke<number[]>('read_file_bytes', { path: path as string })
      setPdfData(bytes)
      setFileName((path as string).split(/[/\\]/).pop() || 'document.pdf')
      pushHistory(bytes)
      showSuccess(t().pdfLoaded)
    } catch (err) {
      showError(formatError(err, 'PDFの読み込みに失敗しました'))
    }
  }, [pushHistory, setPdfData, showError, showSuccess])

  const handleSave = useCallback(async () => {
    if (!pdfData) return
    try {
      const path = await save({
        defaultPath: fileName || 'document.pdf',
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
      })
      if (!path) return
      await invoke('write_file_bytes', { path, data: pdfData })
      showSuccess(t().savedSuccess)
    } catch (err) {
      showError(formatError(err, 'PDFの保存に失敗しました'))
    }
  }, [pdfData, fileName, showError, showSuccess])

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
        if (e.key === 'z' && !e.shiftKey) { e.preventDefault(); undo() }
        if ((e.key === 'z' && e.shiftKey) || e.key === 'y') { e.preventDefault(); redo() }
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [handleOpen, handleSave, undo, redo])

  const exec = useCallback(async (cmd: string, args: Record<string, unknown>) => {
    if (!pdfData) return
    try {
      const result = await invoke<number[]>(cmd, { data: pdfData, ...args })
      pushHistory(result)
      showSuccess(t().completed)
    } catch (err) {
      showError(formatError(err, 'コマンド実行に失敗しました'))
    }
  }, [pdfData, pushHistory, showError, showSuccess])

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

  // Move text block handler from canvas drag
  const handleMoveTextBlock = useCallback(async (blockId: number, newX: number, newY: number) => {
    if (!pdfData) return
    try {
      const result = await invoke<number[]>('move_text_block', {
        data: pdfData,
        page_index: currentPage,
        block_id: blockId,
        new_x: newX,
        new_y: newY,
      })
      pushHistory(result)
      showSuccess(t().textMoved(blockId, newX, newY))
    } catch (err) {
      showError(formatError(err, 'テキスト移動に失敗しました'))
    }
  }, [pdfData, currentPage, pushHistory, showError, showSuccess])

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

  const handlePdfUpdate = (data: number[]) => {
    pushHistory(data)
  }

  // Command items for ⌘K Quick Launcher
  const commandItems: CommandItem[] = [
    {
      id: 'open',
      title: 'PDFを開く',
      subtitle: 'ローカルのPDFドキュメントを選択して読み込み',
      category: 'edit',
      icon: <FolderOpenIcon size={14} />,
      shortcut: '⌘O',
      action: handleOpen,
    },
    {
      id: 'save',
      title: 'PDFを保存',
      subtitle: '編集結果をPDF形式でローカルディスクに書き出し',
      category: 'edit',
      icon: <SaveIcon size={14} />,
      shortcut: '⌘S',
      action: handleSave,
    },
    {
      id: 'tab-edit',
      title: '編集タブ（テキスト・組版リフロー）',
      subtitle: 'インプレース文字置換、JIS X 4051禁則組版リフロー',
      category: 'edit',
      icon: <EditIcon size={14} />,
      action: () => handleSelectTab('edit'),
    },
    {
      id: 'tab-annotate',
      title: '注釈・マークアップ',
      subtitle: 'ハイライト、付箋、描画、図形ツール',
      category: 'annotate',
      icon: <AnnotateIcon size={14} />,
      action: () => handleSelectTab('annotate'),
    },
    {
      id: 'tab-forms',
      title: 'インタラクティブフォーム作成',
      subtitle: 'テキストフィールド、チェックボックス、電子署名枠の配置',
      category: 'forms',
      icon: <FormIcon size={14} />,
      action: () => handleSelectTab('forms'),
    },
    {
      id: 'tab-organize',
      title: 'ページ整理・分割・結合',
      subtitle: '回転、削除、抽出、順序並び替え、複数ファイル結合',
      category: 'organize',
      icon: <OrganizeIcon size={14} />,
      action: () => handleSelectTab('organize'),
    },
    {
      id: 'tab-pages',
      title: 'ページ装飾（透かし・ヘッダー・ベイツ）',
      subtitle: '裁判所証拠用Bates番号付与、セキュリティ透かし',
      category: 'pages',
      icon: <FileIcon size={14} />,
      action: () => handleSelectTab('pages'),
    },
    {
      id: 'tab-security',
      title: 'セキュリティ・電子署名（AATL検証）',
      subtitle: 'パスワード暗号化、AATL認証局信頼チェーン検証',
      category: 'security',
      icon: <LockIcon size={14} />,
      action: () => handleSelectTab('security'),
    },
    {
      id: 'tab-tools',
      title: 'プロダクションツール（PDF/X・プリフライト・最適化）',
      subtitle: 'PDF/X-1a/X-4検証、アクセシビリティ自動修復、フォントアウトライン化',
      category: 'tools',
      icon: <ToolsIcon size={14} />,
      action: () => handleSelectTab('tools'),
    },
    {
      id: 'mode-text',
      title: 'テキスト選択・インプレース直接編集モード',
      subtitle: 'テキストブロックをダブルクリックしてその場でタイプ入力',
      category: 'edit',
      icon: <TypeIcon size={14} />,
      action: () => setInteractiveMode('select-text'),
    },
    {
      id: 'mode-redact',
      title: 'ドラッグ黒塗り墨消しモード',
      subtitle: '矩形ドラッグで機密情報を恒久的に抹消',
      category: 'edit',
      icon: <RedactIcon size={14} />,
      action: () => setInteractiveMode('draw-redact'),
    },
    {
      id: 'mode-highlight',
      title: 'ハイライト描画モード',
      subtitle: '矩形ドラッグで指定箇所を蛍光ペンハイライト',
      category: 'annotate',
      icon: <HighlightIcon size={14} />,
      action: () => setInteractiveMode('draw-highlight'),
    },
    {
      id: 'optimize',
      title: 'PDF最適化・データ削減',
      subtitle: '重複オブジェクト削除とストリーム再圧縮によるファイル縮小',
      category: 'tools',
      icon: <ZapIcon size={14} />,
      action: () => {
        handleSelectTab('tools')
        if (pdfData) exec('optimize_pdf', {})
      },
    },
    {
      id: 'create-outlines',
      title: 'テキストのアウトライン化（全フォントベクター化）',
      subtitle: '印刷・入稿用に全テキストをベクターパスへ変換',
      category: 'tools',
      icon: <VectorPathIcon size={14} />,
      action: () => handleSelectTab('tools'),
    },
    {
      id: 'pdfx-validate',
      title: 'PDF/X 入稿プリフライト検証',
      subtitle: 'PDF/X-1a および PDF/X-4 規格適合性の詳細診断',
      category: 'tools',
      icon: <ShieldCheckIcon size={14} />,
      action: () => handleSelectTab('tools'),
    },
    {
      id: 'pdf-repair',
      title: '破損PDFの自動修復・再構築',
      subtitle: '不正XRef・オフセットズレ・構文エラーの全走査サルベージ',
      category: 'tools',
      icon: <ShieldCheckIcon size={14} />,
      action: () => handleSelectTab('tools'),
    },
    {
      id: 'scan-enhance',
      title: 'スキャン書類の美化・傾き補正（Deskew）',
      subtitle: '斜めスキャン回転補正と裏写り・影・黄ばみの純白化',
      category: 'tools',
      icon: <ZapIcon size={14} />,
      action: () => handleSelectTab('tools'),
    },
    {
      id: 'compare-pdf',
      title: '2文書の差分比較（Compare Files）',
      subtitle: '別バージョンPDFとのテキスト追加・削除・変更を照合検出',
      category: 'tools',
      icon: <ToolsIcon size={14} />,
      action: () => handleSelectTab('tools'),
    },
  ]

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
        onPrint={() => pdfData && invoke('print_pdf', { data: pdfData })}
        canSave={!!pdfData}
        undo={undo}
        redo={redo}
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
                showToast={showToast}
                onInspectSignatures={(sigs: SignatureInfo[]) => setVerifiedSignatures(sigs)}
              />
            )}
            {activeTab === 'text' && (
              <TextEditPanel
                pdfData={pdfData}
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


