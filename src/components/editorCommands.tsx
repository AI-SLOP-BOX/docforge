import React from 'react'
import { CommandItem } from './CommandPalette'
import {
  FolderOpenIcon, SaveIcon, EditIcon, AnnotateIcon, FormIcon,
  OrganizeIcon, LockIcon, ToolsIcon, RedactIcon, HighlightIcon,
  ZapIcon, VectorPathIcon, ShieldCheckIcon
} from './Icons'

interface CommandItemsOptions {
  onOpen: () => void
  onSave: () => void
  onSelectTab: (tab: any) => void
  onSetInteractiveMode: (mode: any) => void
  onExec: (cmd: string, args: Record<string, unknown>) => void
  hasPdf: boolean
}

export function buildEditorCommandItems(opts: CommandItemsOptions): CommandItem[] {
  return [
    {
      id: 'open',
      title: 'PDFを開く',
      subtitle: 'ローカルのPDFドキュメントを選択して読み込み',
      category: 'edit',
      icon: <FolderOpenIcon size={14} />,
      shortcut: '⌘O',
      action: opts.onOpen,
    },
    {
      id: 'save',
      title: 'PDFを保存',
      subtitle: '編集結果をPDF形式でローカルディスクに書き出し',
      category: 'edit',
      icon: <SaveIcon size={14} />,
      shortcut: '⌘S',
      action: opts.onSave,
    },
    {
      id: 'tab-edit',
      title: '編集タブ（テキスト・組版リフロー）',
      subtitle: 'インプレース文字置換、JIS X 4051禁則組版リフロー',
      category: 'edit',
      icon: <EditIcon size={14} />,
      action: () => opts.onSelectTab('edit'),
    },
    {
      id: 'tab-annotate',
      title: '注釈・マークアップ',
      subtitle: 'ハイライト、付箋、描画、図形ツール',
      category: 'annotate',
      icon: <AnnotateIcon size={14} />,
      action: () => opts.onSelectTab('annotate'),
    },
    {
      id: 'tab-forms',
      title: 'フォームフィールド作成',
      subtitle: 'テキスト入力欄、チェックボックス、電子署名枠の配置',
      category: 'forms',
      icon: <FormIcon size={14} />,
      action: () => opts.onSelectTab('forms'),
    },
    {
      id: 'tab-organize',
      title: 'ページ整理・結合・分割',
      subtitle: '回転、削除、抽出、逆順、結合、ページ番号付与',
      category: 'organize',
      icon: <OrganizeIcon size={14} />,
      action: () => opts.onSelectTab('organize'),
    },
    {
      id: 'tab-security',
      title: 'セキュリティ・電子署名・墨消し',
      subtitle: '暗号化、パスワード保護、X.509電子署名、完全抹消',
      category: 'security',
      icon: <LockIcon size={14} />,
      action: () => opts.onSelectTab('security'),
    },
    {
      id: 'tab-tools',
      title: 'プロフェッショナル印刷・プリフライト・変換ツール',
      subtitle: 'PDF/A、PDF/X検証、アウトライン化、CMYK分版、TAC診断',
      category: 'tools',
      icon: <ToolsIcon size={14} />,
      action: () => opts.onSelectTab('tools'),
    },
    {
      id: 'mode-redact',
      title: 'ドラッグ黒塗り墨消しモード',
      subtitle: '矩形ドラッグで機密情報を恒久的に抹消',
      category: 'edit',
      icon: <RedactIcon size={14} />,
      action: () => opts.onSetInteractiveMode('draw-redact'),
    },
    {
      id: 'mode-highlight',
      title: 'ハイライト描画モード',
      subtitle: '矩形ドラッグで指定箇所を蛍光ペンハイライト',
      category: 'annotate',
      icon: <HighlightIcon size={14} />,
      action: () => opts.onSetInteractiveMode('draw-highlight'),
    },
    {
      id: 'optimize',
      title: 'PDF最適化・データ削減',
      subtitle: '重複オブジェクト削除とストリーム再圧縮によるファイル縮小',
      category: 'tools',
      icon: <ZapIcon size={14} />,
      action: () => {
        opts.onSelectTab('tools')
        if (opts.hasPdf) opts.onExec('optimize_pdf', {})
      },
    },
    {
      id: 'create-outlines',
      title: 'テキストのアウトライン化（全フォントベクター化）',
      subtitle: '印刷・入稿用に全テキストをベクターパスへ変換',
      category: 'tools',
      icon: <VectorPathIcon size={14} />,
      action: () => opts.onSelectTab('tools'),
    },
    {
      id: 'pdfx-validate',
      title: 'PDF/X 入稿プリフライト検証',
      subtitle: 'PDF/X-1a および PDF/X-4 規格適合性の詳細診断',
      category: 'tools',
      icon: <ShieldCheckIcon size={14} />,
      action: () => opts.onSelectTab('tools'),
    },
    {
      id: 'pdf-repair',
      title: '破損PDFの自動修復・再構築',
      subtitle: '不正XRef・オフセットズレ・構文エラーの全走査サルベージ',
      category: 'tools',
      icon: <ShieldCheckIcon size={14} />,
      action: () => opts.onSelectTab('tools'),
    },
    {
      id: 'scan-enhance',
      title: 'スキャン書類の美化・傾き補正（Deskew）',
      subtitle: '斜めスキャン回転補正と裏写り・影・黄ばみの純白化',
      category: 'tools',
      icon: <ZapIcon size={14} />,
      action: () => opts.onSelectTab('tools'),
    },
    {
      id: 'cmyk-preview',
      title: 'CMYK分版・インキ総量(TAC)プレビュー',
      subtitle: '版別分解表示とTAC超過アラート表示',
      category: 'tools',
      icon: <ToolsIcon size={14} />,
      action: () => opts.onSelectTab('tools'),
    },
  ]
}
