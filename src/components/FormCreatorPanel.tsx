import { useState, useCallback, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { DocumentService } from '../services/documentService'
import { SectionTitle, Input, NumInput, AccentBtn } from './UIControls'
import { PlusIcon, ZapIcon } from './Icons'

export function FormCreatorPanel({
  pdfData,
  docId,
  currentPage,
  showToast,
  onPdfUpdate,
}: {
  pdfData: number[] | null
  docId?: string | null
  currentPage: number
  exec?: Function
  showToast: (msg: string) => void
  onPdfUpdate: (data: number[]) => void
}) {
  const [fieldName, setFieldName] = useState('text_field_1')
  const [fieldType, setFieldType] = useState<'Tx' | 'Btn' | 'Ch'>('Tx')
  const [defaultValue, setDefaultValue] = useState('')
  const [posX, setPosX] = useState(100)
  const [posY, setPosY] = useState(700)
  const [width, setWidth] = useState(150)
  const [height, setHeight] = useState(25)

  // Calculated field
  const [calcFieldName, setCalcFieldName] = useState('total_price')
  const [calcFormula, setCalcFormula] = useState('this.getField("qty").value * this.getField("price").value')
  const [calcX, setCalcX] = useState(100)
  const [calcY, setCalcY] = useState(650)

  // Field list
  const [existingFields, setExistingFields] = useState<Array<{ name: string; type: string; value: string }>>([])

  const getCurrentBytes = useCallback(async (): Promise<number[] | null> => {
    if (docId) {
      return DocumentService.getSessionBytes(docId)
    }
    return pdfData
  }, [docId, pdfData])

  const loadFields = useCallback(async () => {
    const target = docId || pdfData
    if (!target) return
    try {
      const fields = await DocumentService.getFormFields(target)
      setExistingFields(fields || [])
    } catch (err) {
      showToast(`フォーム取得エラー: ${err}`)
    }
  }, [docId, pdfData, showToast])

  useEffect(() => {
    loadFields()
  }, [loadFields])

  const handleCreateField = async () => {
    const bytes = await getCurrentBytes()
    if (!bytes) return
    try {
      const result = await invoke<number[]>('add_form_field', {
        data: bytes,
        page_index: currentPage,
        field_name: fieldName,
        field_type: fieldType,
        x: posX,
        y: posY,
        width,
        height,
        default_value: defaultValue,
      })
      await onPdfUpdate(result)
      await loadFields()
      showToast(`フィールド「${fieldName}」を追加しました`)
    } catch (err) {
      showToast(`エラー: ${err}`)
    }
  }

  const handleCreateCalculated = async () => {
    const bytes = await getCurrentBytes()
    if (!bytes) return
    try {
      const result = await invoke<number[]>('add_calculated_field', {
        data: bytes,
        page_index: currentPage,
        field_name: calcFieldName,
        formula: calcFormula,
        x: calcX,
        y: calcY,
        width: 150,
        height: 25,
      })
      await onPdfUpdate(result)
      await loadFields()
      showToast(`計算フィールド「${calcFieldName}」を追加しました`)
    } catch (err) {
      showToast(`エラー: ${err}`)
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      {/* Card 1: Form Field Creator */}
      <div className="inspector-card">
        <div className="inspector-card-header">
          <span>フォームフィールド作成</span>
          <span style={{ fontSize: 9, color: 'var(--accent)', fontWeight: 600 }}>INTERACTIVE</span>
        </div>
        <div className="inspector-card-desc">PDF上に直接入力可能なフィールドを配置</div>

        <div style={{ marginBottom: 6 }}>
          <label style={{ fontSize: 10, fontWeight: 600, color: 'var(--text-muted)', display: 'block', marginBottom: 2 }}>
            フィールド種類
          </label>
          <select
            value={fieldType}
            onChange={e => setFieldType(e.target.value as any)}
            style={{
              width: '100%', padding: '6px 8px', background: 'var(--bg-0)',
              border: '1px solid var(--border)', borderRadius: 'var(--radius-sm)',
              color: 'var(--text)', fontSize: 12,
            }}
          >
            <option value="Tx">テキスト入力枠 (Text)</option>
            <option value="Btn">チェックボックス / ボタン (Button)</option>
            <option value="Ch">ドロップダウン / 選択肢 (Choice)</option>
          </select>
        </div>

        <Input value={fieldName} onChange={setFieldName} placeholder="フィールド名 (例: customer_name)" />
        <Input value={defaultValue} onChange={setDefaultValue} placeholder="初期値" />

        <div style={{ display: 'flex', gap: 8, marginTop: 4 }}>
          <NumInput value={posX} onChange={setPosX} label="X座標 (pt)" />
          <NumInput value={posY} onChange={setPosY} label="Y座標 (pt)" />
        </div>
        <div style={{ display: 'flex', gap: 8, marginTop: 4 }}>
          <NumInput value={width} onChange={setWidth} label="横幅 (pt)" />
          <NumInput value={height} onChange={setHeight} label="高さ (pt)" />
        </div>

        <AccentBtn onClick={handleCreateField} style={{ background: 'var(--accent)', marginTop: 8 }}>
          <PlusIcon size={14} /> フォーム追加 (p{currentPage + 1})
        </AccentBtn>
      </div>

      {/* Card 2: Calculated Field (JavaScript) */}
      <div className="inspector-card">
        <div className="inspector-card-header">
          <span>自動計算フィールド (JS式)</span>
          <span style={{ fontSize: 9, color: 'var(--purple)', fontWeight: 600 }}>SCRIPT</span>
        </div>
        <div className="inspector-card-desc">他フィールド値を参照して自動計算を行う高度機能</div>

        <Input value={calcFieldName} onChange={setCalcFieldName} placeholder="計算フィールド名 (例: total)" />
        <Input value={calcFormula} onChange={setCalcFormula} placeholder="JS式: this.getField('a').value * 1.1" />

        <div style={{ display: 'flex', gap: 8, marginTop: 4 }}>
          <NumInput value={calcX} onChange={setCalcX} label="X座標" />
          <NumInput value={calcY} onChange={setCalcY} label="Y座標" />
        </div>

        <AccentBtn onClick={handleCreateCalculated} style={{ background: 'var(--purple)', marginTop: 8 }}>
          <ZapIcon size={14} /> 計算フィールドを追加
        </AccentBtn>
      </div>

      {/* Card 3: Existing Fields */}
      <div className="inspector-card">
        <div className="inspector-card-header">
          <span>既存フィールド一覧</span>
          <span style={{ fontSize: 9, color: 'var(--text-muted)' }}>{existingFields.length} 件</span>
        </div>
        <AccentBtn onClick={loadFields} style={{ background: 'var(--bg-2)', color: 'var(--text-dim)', border: '1px solid var(--border)', marginBottom: 6 }}>
          一覧を再読込
        </AccentBtn>

        {existingFields.length === 0 ? (
          <div style={{ fontSize: 11, color: 'var(--text-muted)', textAlign: 'center', padding: '12px 0' }}>
            検出されたフィールドはありません
          </div>
        ) : (
          <div style={{ maxHeight: 180, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 4 }}>
            {existingFields.map((f, i) => (
              <div key={i} style={{
                padding: '6px 8px', background: 'var(--bg-0)', border: '1px solid var(--border)',
                borderRadius: 'var(--radius-sm)', fontSize: 11,
              }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', color: 'var(--text)' }}>
                  <span style={{ fontWeight: 600 }}>{f.name}</span>
                  <span style={{ color: 'var(--accent)', fontSize: 10 }}>[{f.type}]</span>
                </div>
                {f.value && (
                  <div style={{ color: 'var(--text-dim)', marginTop: 2, fontSize: 10 }}>
                    値: {f.value}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
