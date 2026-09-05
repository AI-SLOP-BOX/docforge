use super::*;

// ===== FILE I/O & BATCH PROCESSING TAURI COMMANDS =====

fn validate_safe_path(path_str: &str, for_write: bool) -> Result<std::path::PathBuf, String> {
    let path = std::path::Path::new(path_str);
    if path_str.trim().is_empty() {
        return Err("File path cannot be empty".to_string());
    }

    // Disallow null bytes
    if path_str.contains('\0') {
        return Err("Invalid path containing null bytes".to_string());
    }

    if for_write {
        // For writing, ensure parent directory exists or is valid
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && parent.exists() {
                let canonical_parent = parent
                    .canonicalize()
                    .map_err(|e| format!("Invalid target directory: {e}"))?;
                return Ok(canonical_parent.join(path.file_name().ok_or("Invalid filename")?));
            }
        }
        Ok(path.to_path_buf())
    } else {
        // For reading, canonicalize to prevent ../ directory traversal
        let canonical = path
            .canonicalize()
            .map_err(|e| format!("File not found or inaccessible: {e}"))?;
        Ok(canonical)
    }
}

#[tauri::command]
pub fn read_file_bytes(path: String) -> Result<Vec<u8>, String> {
    let safe_path = validate_safe_path(&path, false)?;
    std::fs::read(&safe_path).map_err(|e| format!("Failed to read file: {e}"))
}

#[tauri::command]
pub fn write_file_bytes(path: String, data: Vec<u8>) -> Result<(), String> {
    let safe_path = validate_safe_path(&path, true)?;
    std::fs::write(&safe_path, &data).map_err(|e| format!("Failed to write file: {e}"))
}

#[tauri::command]
pub fn write_text_file(path: String, content: String) -> Result<(), String> {
    let safe_path = validate_safe_path(&path, true)?;
    std::fs::write(&safe_path, &content).map_err(|e| format!("Failed to write file: {e}"))
}

#[tauri::command]
pub fn batch_merge_pdfs(paths: Vec<String>, output_path: String) -> Result<(), String> {
    pdf_engine::batch_merge_pdfs(&paths, &output_path)
}

#[tauri::command]
pub fn batch_add_watermark(
    paths: Vec<String>,
    text: String,
    opacity: f32,
    rotation: f32,
    font_size: f32,
    color: String,
) -> Result<Vec<Vec<u8>>, String> {
    pdf_engine::batch_add_watermark(&paths, &text, opacity, rotation, font_size, &color)
}

#[tauri::command]
pub fn batch_protect(paths: Vec<String>, password: String) -> Result<Vec<Vec<u8>>, String> {
    pdf_engine::batch_protect(&paths, &password)
}

#[tauri::command]
pub fn batch_optimize(paths: Vec<String>) -> Result<Vec<Vec<u8>>, String> {
    pdf_engine::batch_optimize(&paths)
}

// ===== CONVERSIONS & EXPORT TAURI COMMANDS =====

#[tauri::command]
pub fn pdf_to_images(
    data: Vec<u8>,
    output_dir: String,
    format: String,
    dpi: u32,
) -> Result<Vec<String>, String> {
    pdf_engine::pdf_to_images(&data, &output_dir, &format, dpi)
}

#[tauri::command]
pub fn images_to_pdf(image_paths: Vec<String>, output_path: String) -> Result<(), String> {
    pdf_engine::images_to_pdf(&image_paths, &output_path)
}

#[tauri::command]
pub fn html_to_pdf(html_content: String, output_path: String) -> Result<(), String> {
    pdf_engine::html_to_pdf(&html_content, &output_path)
}

#[tauri::command]
pub fn pdf_to_word(data: Vec<u8>, output_path: String) -> Result<(), String> {
    pdf_engine::pdf_to_word(&data, &output_path)
}

#[tauri::command]
pub fn pdf_to_excel(data: Vec<u8>, output_path: String) -> Result<(), String> {
    pdf_engine::pdf_to_excel(&data, &output_path)
}

#[tauri::command]
pub fn pdf_to_powerpoint(data: Vec<u8>, output_path: String) -> Result<(), String> {
    pdf_engine::pdf_to_powerpoint(&data, &output_path)
}

#[tauri::command]
pub fn create_pdf_portfolio(file_paths: Vec<String>, output_path: String) -> Result<(), String> {
    pdf_engine::create_pdf_portfolio(&file_paths, &output_path)
}
