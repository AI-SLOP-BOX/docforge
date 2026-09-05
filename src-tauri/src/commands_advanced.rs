use super::*;

// ===== PRO SIGNATURE, TOKEN & EXTENDED WORKFLOW TAURI COMMANDS =====

#[tauri::command]
pub fn add_digital_signature(
    data: Vec<u8>,
    page_index: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    signer_name: String,
    reason: String,
) -> Result<Vec<u8>, String> {
    pdf_engine::add_digital_signature(
        &data,
        page_index,
        x,
        y,
        width,
        height,
        &signer_name,
        &reason,
        None,
    )
}

#[tauri::command]
pub fn verify_signature(
    data: Vec<u8>,
    signature_index: usize,
) -> Result<serde_json::Value, String> {
    pdf_engine::verify_signature(&data, signature_index)
}

#[tauri::command]
pub fn embed_font(data: Vec<u8>, page_index: usize, font_path: String) -> Result<Vec<u8>, String> {
    pdf_engine::embed_font(&data, page_index, &font_path)
}

#[tauri::command]
pub fn add_form_field(
    data: Vec<u8>,
    page_index: usize,
    field_name: String,
    field_type: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    default_value: String,
) -> Result<Vec<u8>, String> {
    pdf_engine::add_form_field(
        &data,
        page_index,
        &field_name,
        &field_type,
        x,
        y,
        width,
        height,
        &default_value,
    )
}

#[tauri::command]
pub fn add_calculated_field(
    data: Vec<u8>,
    page_index: usize,
    field_name: String,
    formula: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<Vec<u8>, String> {
    pdf_engine::add_calculated_field(
        &data,
        page_index,
        &field_name,
        &formula,
        x,
        y,
        width,
        height,
    )
}

#[tauri::command]
pub fn export_xfdf(data: Vec<u8>) -> Result<String, String> {
    pdf_engine::export_xfdf(&data)
}

#[tauri::command]
pub fn import_xfdf(data: Vec<u8>, xfdf_content: String) -> Result<Vec<u8>, String> {
    pdf_engine::import_xfdf(&data, &xfdf_content)
}

#[tauri::command]
pub fn detect_hardware_tokens() -> Result<Vec<serde_json::Value>, String> {
    let tokens = pdf_engine::detect_hardware_tokens()?;
    let result: Vec<serde_json::Value> = tokens
        .iter()
        .map(|t| {
            serde_json::json!({
                "slot_id": t.slot_id,
                "label": t.label,
                "manufacturer": t.manufacturer,
                "serial": t.serial,
                "initialized": t.initialized,
            })
        })
        .collect();
    Ok(result)
}

#[tauri::command]
pub fn sign_with_hardware_token(
    data: Vec<u8>,
    slot_id: u32,
    pin: String,
    page_index: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    signer_name: String,
    reason: String,
) -> Result<Vec<u8>, String> {
    pdf_engine::sign_with_hardware_token(
        &data,
        slot_id,
        &pin,
        page_index,
        x,
        y,
        width,
        height,
        &signer_name,
        &reason,
    )
}

#[tauri::command]
pub fn verify_hardware_token_signature(
    data: Vec<u8>,
    slot_id: u32,
) -> Result<serde_json::Value, String> {
    pdf_engine::verify_hardware_token_signature(&data, slot_id)
}

#[tauri::command]
pub fn repair_pdf(data: Vec<u8>) -> Result<Vec<u8>, String> {
    pdf_engine::repair_pdf(&data)
}

#[tauri::command]
pub fn unlock_pdf(data: Vec<u8>, password: String) -> Result<Vec<u8>, String> {
    pdf_engine::unlock_pdf(&data, &password)
}

#[tauri::command]
pub fn compress_pdf_quality(data: Vec<u8>, quality: u8) -> Result<Vec<u8>, String> {
    pdf_engine::compress_pdf_quality(&data, quality)
}

#[tauri::command]
pub fn add_page_numbers(
    data: Vec<u8>,
    position: String,
    font_size: f32,
    start_number: usize,
) -> Result<Vec<u8>, String> {
    pdf_engine::add_page_numbers(&data, &position, font_size, start_number)
}

#[tauri::command]
pub fn create_action_wizard(name: String, steps: Vec<serde_json::Value>) -> Result<String, String> {
    let action_steps: Vec<pdf_engine::ActionStep> = steps
        .iter()
        .map(|s| pdf_engine::ActionStep {
            action_type: s
                .get("action_type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            params: s.get("params").cloned().unwrap_or(serde_json::Value::Null),
        })
        .collect();
    pdf_engine::create_action_wizard(&name, &action_steps)
}

#[tauri::command]
pub fn execute_action_wizard(data: Vec<u8>, wizard_json: String) -> Result<Vec<u8>, String> {
    pdf_engine::execute_action_wizard(&data, &wizard_json)
}

#[tauri::command]
pub fn aggregate_form_data(pdf_paths: Vec<String>) -> Result<serde_json::Value, String> {
    pdf_engine::aggregate_form_data(&pdf_paths)
}

#[tauri::command]
pub fn embed_javascript(data: Vec<u8>, script: String) -> Result<Vec<u8>, String> {
    pdf_engine::embed_javascript(&data, &script)
}

#[tauri::command]
pub fn add_bookmark_tree(
    data: Vec<u8>,
    bookmarks: Vec<serde_json::Value>,
) -> Result<Vec<u8>, String> {
    pdf_engine::add_bookmark_tree(&data, &bookmarks)
}

#[tauri::command]
pub fn visual_diff(data1: Vec<u8>, data2: Vec<u8>, output_path: String) -> Result<(), String> {
    pdf_engine::visual_diff(&data1, &data2, &output_path)
}

#[tauri::command]
pub fn list_digital_ids() -> Result<Vec<pdf_engine::DigitalID>, String> {
    pdf_engine::list_digital_ids()
}

