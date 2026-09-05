use super::*;

// ===== PRO PRODUCTION, COLOR, AND COMPLIANCE TAURI COMMANDS =====

#[tauri::command]
pub fn convert_to_pdfx(data: Vec<u8>, output_intent: String) -> Result<Vec<u8>, String> {
    pdf_engine::convert_to_pdfx(&data, &output_intent)
}

#[tauri::command]
pub fn convert_to_pdfx_standard(data: Vec<u8>, standard: String, output_intent: String) -> Result<Vec<u8>, String> {
    pdf_engine::convert_to_pdfx_standard(&data, &standard, &output_intent)
}

#[tauri::command]
pub fn validate_pdfx_compliance(data: Vec<u8>, target_standard: String) -> Result<pdf_engine::PdfxValidationReport, String> {
    pdf_engine::validate_pdfx_compliance(&data, &target_standard)
}

#[tauri::command]
pub fn check_accessibility(data: Vec<u8>) -> Result<serde_json::Value, String> {
    pdf_engine::check_accessibility(&data)
}

#[tauri::command]
pub fn fix_accessibility_issues(data: Vec<u8>, default_title: String, default_lang: String) -> Result<Vec<u8>, String> {
    pdf_engine::fix_accessibility_issues(&data, &default_title, &default_lang)
}

#[tauri::command]
pub fn preview_color_separations(data: Vec<u8>) -> Result<serde_json::Value, String> {
    pdf_engine::preview_color_separations(&data)
}

#[tauri::command]
pub fn render_color_separation(
    data: Vec<u8>,
    page_index: usize,
    dpi: u32,
    show_c: bool,
    show_m: bool,
    show_y: bool,
    show_k: bool,
    highlight_tac: bool,
    tac_limit: u32,
) -> Result<Vec<u8>, String> {
    pdf_engine::render_color_separation(
        &data,
        page_index,
        dpi,
        show_c,
        show_m,
        show_y,
        show_k,
        highlight_tac,
        tac_limit,
    )
}

#[tauri::command]
pub fn convert_to_cmyk(data: Vec<u8>) -> Result<Vec<u8>, String> {
    pdf_engine::convert_to_cmyk(&data)
}

#[tauri::command]
pub fn embed_icc_profile(data: Vec<u8>, profile_name: String) -> Result<Vec<u8>, String> {
    pdf_engine::embed_icc_profile(&data, &profile_name)
}

#[tauri::command]
pub fn rgb_to_cmyk(r: u8, g: u8, b: u8) -> Result<serde_json::Value, String> {
    let (c, m, y, k) = pdf_engine::rgb_to_cmyk(r, g, b);
    Ok(serde_json::json!({"c": c, "m": m, "y": y, "k": k}))
}

#[tauri::command]
pub fn cmyk_to_rgb(c: u8, m: u8, y: u8, k: u8) -> Result<serde_json::Value, String> {
    let (r, g, b) = pdf_engine::cmyk_to_rgb(c, m, y, k);
    Ok(serde_json::json!({"r": r, "g": g, "b": b}))
}

#[tauri::command]
pub fn flatten_transparency(data: Vec<u8>) -> Result<Vec<u8>, String> {
    pdf_engine::flatten_transparency(&data)
}

#[tauri::command]
pub fn flatten_content(data: Vec<u8>) -> Result<Vec<u8>, String> {
    pdf_engine::flatten_content(&data)
}

#[tauri::command]
pub fn downsample_images(data: Vec<u8>, target_dpi: u32, quality: u8) -> Result<Vec<u8>, String> {
    pdf_engine::downsample_images(&data, target_dpi, quality)
}

#[tauri::command]
pub fn remove_metadata(data: Vec<u8>,) -> Result<Vec<u8>, String> {
    pdf_engine::remove_metadata(&data)
}

#[tauri::command]
pub fn repair_corrupt_pdf(data: Vec<u8>) -> Result<Vec<u8>, String> {
    pdf_engine::repair_corrupt_pdf(&data)
}

#[tauri::command]
pub fn enhance_scanned_pdf(data: Vec<u8>, options: pdf_engine::ScanEnhanceOptions) -> Result<Vec<u8>, String> {
    pdf_engine::enhance_scanned_pdf(&data, &options)
}

#[tauri::command]
pub fn compare_pdf_documents(original: Vec<u8>, revised: Vec<u8>) -> Result<pdf_engine::CompareReport, String> {
    pdf_engine::compare_pdf_documents(&original, &revised)
}
