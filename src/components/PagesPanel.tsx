import { Input, NumInput, ColorInput, SliderInput, AccentBtn } from './UIControls'

export function PagesPanel({
  watermarkText,
  setWatermarkText,
  watermarkOpacity,
  setWatermarkOpacity,
  watermarkRotation,
  setWatermarkRotation,
  watermarkFontSize,
  setWatermarkFontSize,
  watermarkColor,
  setWatermarkColor,
  headerText,
  setHeaderText,
  footerText,
  setFooterText,
  hfFontSize,
  setHfFontSize,
  batesPrefix,
  setBatesPrefix,
  batesStart,
  setBatesStart,
  batesFontSize,
  setBatesFontSize,
  exec,
}: {
  watermarkText: string
  setWatermarkText: (v: string) => void
  watermarkOpacity: number
  setWatermarkOpacity: (v: number) => void
  watermarkRotation: number
  setWatermarkRotation: (v: number) => void
  watermarkFontSize: number
  setWatermarkFontSize: (v: number) => void
  watermarkColor: string
  setWatermarkColor: (v: string) => void
  headerText: string
  setHeaderText: (v: string) => void
  footerText: string
  setFooterText: (v: string) => void
  hfFontSize: number
  setHfFontSize: (v: number) => void
  batesPrefix: string
  setBatesPrefix: (v: string) => void
  batesStart: number
  setBatesStart: (v: number) => void
  batesFontSize: number
  setBatesFontSize: (v: number) => void
  exec: Function
}) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      {/* Card 1: Watermark */}
      <div className="inspector-card">
        <div className="inspector-card-header">
          <span>透かし（ウォーターマーク）</span>
          <span style={{ fontSize: 9, color: 'var(--accent)', fontWeight: 600 }}>WATERMARK</span>
        </div>
        <div className="inspector-card-desc">全ページに透かし文字を角度付きで合成</div>
        <Input value={watermarkText} onChange={setWatermarkText} placeholder="透かし文字 (例: CONFIDENTIAL)" />
        <div style={{ display: 'flex', gap: 8, marginTop: 4 }}>
          <NumInput value={watermarkFontSize} onChange={setWatermarkFontSize} label="フォントサイズ" />
          <NumInput value={watermarkRotation} onChange={setWatermarkRotation} label="回転角度 (°)" />
        </div>
        <ColorInput value={watermarkColor} onChange={setWatermarkColor} label="文字カラー" />
        <SliderInput value={watermarkOpacity} onChange={setWatermarkOpacity} label="不透明度 (Opacity)" min={0} max={1} step={0.05} />
        <AccentBtn onClick={() => exec('add_watermark', { text: watermarkText, opacity: watermarkOpacity, rotation: watermarkRotation, font_size: watermarkFontSize, color: watermarkColor, all_pages: true, page_indices: [] })}>
          透かしを全ページに適用
        </AccentBtn>
        <AccentBtn onClick={() => exec('remove_watermarks', {})} style={{ marginTop: 6, background: 'var(--bg-2)', color: 'var(--text-dim)', border: '1px solid var(--border)' }}>
          透かしを除去
        </AccentBtn>
      </div>

      {/* Card 2: Header / Footer */}
      <div className="inspector-card">
        <div className="inspector-card-header">
          <span>ヘッダー / フッター</span>
          <span style={{ fontSize: 9, color: 'var(--text-muted)' }}>PAGE-NUM</span>
        </div>
        <div className="inspector-card-desc">&#123;page&#125;, &#123;total&#125; 変数展開に対応</div>
        <Input value={headerText} onChange={setHeaderText} placeholder="ヘッダー ({page} / {total})" />
        <Input value={footerText} onChange={setFooterText} placeholder="フッター ({page} / {total})" />
        <NumInput value={hfFontSize} onChange={setHfFontSize} label="文字サイズ (pt)" />
        <AccentBtn onClick={() => exec('add_header_footer', { header_text: headerText, footer_text: footerText, font_size: hfFontSize, margin: 40 })} style={{ marginTop: 6 }}>
          ヘッダー/フッターを適用
        </AccentBtn>
      </div>

      {/* Card 3: Bates Numbering */}
      <div className="inspector-card">
        <div className="inspector-card-header">
          <span>Batesナンバリング</span>
          <span style={{ fontSize: 9, color: 'var(--text-muted)' }}>LEGAL</span>
        </div>
        <div className="inspector-card-desc">法務文書向けの通し番号プレフィックス印字</div>
        <div style={{ display: 'flex', gap: 8 }}>
          <Input value={batesPrefix} onChange={setBatesPrefix} placeholder="接頭辞 (例: DOC-)" />
          <NumInput value={batesStart} onChange={setBatesStart} label="開始番号" />
        </div>
        <NumInput value={batesFontSize} onChange={setBatesFontSize} label="サイズ (pt)" />
        <AccentBtn onClick={() => exec('add_bates_number', { prefix: batesPrefix, start_number: batesStart, font_size: batesFontSize, margin: 40 })} style={{ marginTop: 6 }}>
          Bates番号を全ページ印字
        </AccentBtn>
      </div>
    </div>
  )
}
