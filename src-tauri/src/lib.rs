pub mod commands_advanced;
pub mod commands_io;
pub mod commands_prod;
pub mod commands_session;
pub mod commands_text;
pub mod image_engine;
pub mod ocr_engine;
pub mod pdf_engine;
pub mod session;
pub use commands_advanced::*;
pub use commands_io::*;
pub use commands_prod::*;
pub use commands_session::*;
pub use commands_text::*;

// ===== PDF CORE =====

#[tauri::command]
fn merge_pdfs(paths: Vec<String>) -> Result<Vec<u8>, String> {
    pdf_engine::merge_pdfs(&paths)
}

#[tauri::command]
fn delete_page(data: Vec<u8>, page_index: usize) -> Result<Vec<u8>, String> {
    pdf_engine::delete_page(&data, page_index)
}

#[tauri::command]
fn rotate_page(data: Vec<u8>, page_index: usize, degrees: i32) -> Result<Vec<u8>, String> {
    pdf_engine::rotate_page(&data, page_index, degrees)
}

#[tauri::command]
fn reorder_pages(data: Vec<u8>, from_index: usize, to_index: usize) -> Result<Vec<u8>, String> {
    pdf_engine::reorder_pages(&data, from_index, to_index)
}

#[tauri::command]
fn extract_pages(data: Vec<u8>, indices: Vec<usize>) -> Result<Vec<u8>, String> {
    pdf_engine::extract_pages(&data, &indices)
}

#[tauri::command]
fn duplicate_page(data: Vec<u8>, page_index: usize) -> Result<Vec<u8>, String> {
    pdf_engine::duplicate_page(&data, page_index)
}

#[tauri::command]
fn create_blank_pdf(width: f64, height: f64, page_count: usize) -> Result<Vec<u8>, String> {
    pdf_engine::create_blank_pdf(width, height, page_count)
}

// ===== TEXT & IMAGES =====

#[tauri::command]
fn add_text(
    data: Vec<u8>,
    page_index: usize,
    text: String,
    x: f64,
    y: f64,
    size: f64,
    color: String,
) -> Result<Vec<u8>, String> {
    pdf_engine::add_text(&data, page_index, &text, x, y, size, &color)
}

#[tauri::command]
fn add_image_to_page(
    data: Vec<u8>,
    page_index: usize,
    image_data: Vec<u8>,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<Vec<u8>, String> {
    pdf_engine::add_image_to_page(&data, page_index, &image_data, x, y, width, height)
}

#[tauri::command]
fn crop_page(
    data: Vec<u8>,
    page_index: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<Vec<u8>, String> {
    pdf_engine::crop_page(&data, page_index, x, y, width, height)
}

// ===== WATERMARK =====

#[tauri::command]
fn add_watermark(
    data: Vec<u8>,
    text: String,
    opacity: f32,
    rotation: f32,
    font_size: f32,
    color: String,
    all_pages: bool,
    page_indices: Vec<usize>,
) -> Result<Vec<u8>, String> {
    pdf_engine::add_watermark(
        &data,
        &text,
        opacity,
        rotation,
        font_size,
        &color,
        all_pages,
        &page_indices,
    )
}

#[tauri::command]
fn remove_watermarks(data: Vec<u8>) -> Result<Vec<u8>, String> {
    pdf_engine::remove_watermarks(&data)
}

// ===== ANNOTATIONS =====

#[tauri::command]
fn add_highlight(
    data: Vec<u8>,
    page_index: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    color: String,
) -> Result<Vec<u8>, String> {
    pdf_engine::add_highlight(&data, page_index, x, y, width, height, &color)
}

#[tauri::command]
fn add_underline(
    data: Vec<u8>,
    page_index: usize,
    x: f64,
    y: f64,
    width: f64,
    color: String,
) -> Result<Vec<u8>, String> {
    pdf_engine::add_underline(&data, page_index, x, y, width, &color)
}

#[tauri::command]
fn add_sticky_note(
    data: Vec<u8>,
    page_index: usize,
    x: f64,
    y: f64,
    text: String,
    color: String,
) -> Result<Vec<u8>, String> {
    pdf_engine::add_sticky_note(&data, page_index, x, y, &text, &color)
}

#[tauri::command]
fn add_rectangle(
    data: Vec<u8>,
    page_index: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    stroke_color: String,
    fill_color: String,
    stroke_width: f32,
) -> Result<Vec<u8>, String> {
    pdf_engine::add_rectangle(
        &data,
        page_index,
        x,
        y,
        width,
        height,
        &stroke_color,
        &fill_color,
        stroke_width,
    )
}

#[tauri::command]
fn add_line(
    data: Vec<u8>,
    page_index: usize,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    color: String,
    width: f32,
) -> Result<Vec<u8>, String> {
    pdf_engine::add_line(&data, page_index, x1, y1, x2, y2, &color, width)
}

// ===== REDACTION =====

#[tauri::command]
fn redact_area(
    data: Vec<u8>,
    page_index: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    color: String,
) -> Result<Vec<u8>, String> {
    pdf_engine::redact_area(&data, page_index, x, y, width, height, &color)
}

#[tauri::command]
fn redact_text(data: Vec<u8>, search_text: String, replacement: String) -> Result<Vec<u8>, String> {
    pdf_engine::redact_text(&data, &search_text, &replacement)
}



// ===== OPTIMIZE =====

#[tauri::command]
fn optimize_pdf(data: Vec<u8>) -> Result<Vec<u8>, String> {
    pdf_engine::optimize_pdf(&data)
}

// ===== SECURITY =====

#[tauri::command]
fn protect_pdf(data: Vec<u8>, password: String) -> Result<Vec<u8>, String> {
    pdf_engine::protect_pdf(&data, &password)
}

// ===== COMPARE =====

#[tauri::command]
fn compare_pdfs(data1: Vec<u8>, data2: Vec<u8>) -> Result<serde_json::Value, String> {
    let result = pdf_engine::compare_pdfs(&data1, &data2)?;
    serde_json::to_value(result).map_err(|e| e.to_string())
}

// ===== PDF RENDERING =====

#[tauri::command]
fn get_page_count(data: Vec<u8>) -> Result<usize, String> {
    pdf_engine::get_page_count_from_data(&data)
}

#[tauri::command]
fn get_page_dimensions(data: Vec<u8>, page_index: usize) -> Result<serde_json::Value, String> {
    let (w, h) = pdf_engine::get_page_dimensions_from_data(&data, page_index)?;
    Ok(serde_json::json!({"width": w, "height": h}))
}

#[tauri::command]
fn render_page_to_png(data: Vec<u8>, page_index: usize, dpi: u32) -> Result<Vec<u8>, String> {
    pdf_engine::render_page_to_png(&data, page_index, dpi)
}

#[tauri::command]
fn get_page_text(data: Vec<u8>, page_index: usize) -> Result<String, String> {
    pdf_engine::get_page_text(&data, page_index)
}

#[tauri::command]
fn search_text(data: Vec<u8>, query: String) -> Result<Vec<serde_json::Value>, String> {
    let results = pdf_engine::search_text(&data, &query)?;
    Ok(results)
}

#[tauri::command]
fn get_bookmarks(data: Vec<u8>) -> Result<Vec<serde_json::Value>, String> {
    let bookmarks = pdf_engine::get_bookmarks(&data)?;
    Ok(bookmarks)
}

#[tauri::command]
fn add_bookmark_to_pdf(data: Vec<u8>, title: String, page_index: usize) -> Result<Vec<u8>, String> {
    pdf_engine::add_bookmark(&data, &title, page_index)
}

#[tauri::command]
fn get_form_fields(data: Vec<u8>) -> Result<Vec<serde_json::Value>, String> {
    let fields = pdf_engine::get_form_fields(&data)?;
    Ok(fields)
}

#[tauri::command]
fn set_form_field(data: Vec<u8>, field_name: String, value: String) -> Result<Vec<u8>, String> {
    pdf_engine::set_form_field(&data, &field_name, &value)
}

#[tauri::command]
fn flatten_form(data: Vec<u8>) -> Result<Vec<u8>, String> {
    pdf_engine::flatten_form(&data)
}

#[tauri::command]
fn add_stamp(
    data: Vec<u8>,
    page_index: usize,
    text: String,
    x: f64,
    y: f64,
    rotation: f32,
    color: String,
    font_size: f32,
) -> Result<Vec<u8>, String> {
    pdf_engine::add_stamp(&data, page_index, &text, x, y, rotation, &color, font_size)
}

#[tauri::command]
fn print_pdf(data: Vec<u8>) -> Result<(), String> {
    pdf_engine::print_pdf(&data)
}

#[tauri::command]
fn get_pdf_metadata(data: Vec<u8>) -> Result<serde_json::Value, String> {
    let meta = pdf_engine::get_pdf_metadata(&data)?;
    Ok(meta)
}

// ===== IMAGE PROCESSING =====

#[tauri::command]
fn process_scanned_images(
    paths: Vec<String>,
    remove_shadow: bool,
    correct_perspective: bool,
    dpi: u32,
) -> Result<Vec<u8>, String> {
    image_engine::process_scanned_images(&paths, remove_shadow, correct_perspective, dpi)
}

// ===== OCR =====

#[tauri::command]
fn ocr_files(paths: Vec<String>, language: String) -> Result<serde_json::Value, String> {
    let result = ocr_engine::ocr_files(&paths, &language)?;
    serde_json::to_value(result).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_epub(text: String, output_path: String, title: String) -> Result<(), String> {
    ocr_engine::create_epub(&text, &output_path, &title)
}

#[tauri::command]
fn create_searchable_pdf(
    original_paths: Vec<String>,
    ocr_text: String,
    output_path: String,
) -> Result<(), String> {
    ocr_engine::create_searchable_pdf(&original_paths, &ocr_text, &output_path)
}

// ===== DEEP REDACTION =====

#[tauri::command]
fn deep_redact(
    data: Vec<u8>,
    page_index: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    color: String,
) -> Result<Vec<u8>, String> {
    pdf_engine::deep_redact(&data, page_index, x, y, width, height, &color)
}

#[tauri::command]
fn redact_text_deep(data: Vec<u8>, search_text: String, color: String) -> Result<Vec<u8>, String> {
    pdf_engine::redact_text_deep(&data, &search_text, &color)
}

#[tauri::command]
fn sanitize_document(data: Vec<u8>) -> Result<(Vec<u8>, pdf_engine::SanitizeSummary), String> {
    pdf_engine::sanitize_document(&data)
}

#[tauri::command]
fn convert_fonts_to_outlines(data: Vec<u8>) -> Result<Vec<u8>, String> {
    pdf_engine::convert_fonts_to_outlines(&data)
}

// ===== ANNOTATION MANAGEMENT =====

#[tauri::command]
fn get_annotations(data: Vec<u8>) -> Result<Vec<serde_json::Value>, String> {
    pdf_engine::get_annotations(&data)
}

#[tauri::command]
fn add_annotation_reply(
    data: Vec<u8>,
    annotation_id: (u32, u16),
    author: String,
    contents: String,
) -> Result<Vec<u8>, String> {
    pdf_engine::add_annotation_reply(&data, annotation_id, &author, &contents)
}

#[tauri::command]
fn set_annotation_status(
    data: Vec<u8>,
    annotation_id: (u32, u16),
    status: String,
) -> Result<Vec<u8>, String> {
    pdf_engine::set_annotation_status(&data, annotation_id, &status)
}

#[tauri::command]
fn delete_annotation(data: Vec<u8>, annotation_id: (u32, u16)) -> Result<Vec<u8>, String> {
    pdf_engine::delete_annotation(&data, annotation_id)
}

// ===== PDF/A =====

#[tauri::command]
fn convert_to_pdfa(data: Vec<u8>) -> Result<Vec<u8>, String> {
    pdf_engine::convert_to_pdfa(&data)
}

// ===== DIGITAL SIGNATURE =====

// ===== APP =====

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            read_file_bytes,
            write_file_bytes,
            write_text_file,
            merge_pdfs,
            delete_page,
            rotate_page,
            reorder_pages,
            extract_pages,
            duplicate_page,
            add_text,
            protect_pdf,
            create_blank_pdf,
            add_image_to_page,
            crop_page,
            add_watermark,
            remove_watermarks,
            add_highlight,
            add_underline,
            add_sticky_note,
            add_rectangle,
            add_line,
            redact_area,
            redact_text,
            deep_redact,
            redact_text_deep,
            add_header_footer,
            add_bookmark,
            add_bates_number,
            optimize_pdf,
            compare_pdfs,
            get_page_count,
            get_page_dimensions,
            render_page_to_png,
            get_page_text,
            search_text,
            get_bookmarks,
            add_bookmark_to_pdf,
            get_form_fields,
            set_form_field,
            flatten_form,
            add_stamp,
            print_pdf,
            get_pdf_metadata,
            get_annotations,
            add_annotation_reply,
            set_annotation_status,
            delete_annotation,
            batch_merge_pdfs,
            batch_add_watermark,
            batch_protect,
            batch_optimize,
            convert_to_pdfa,
            edit_text,
            get_text_positions,
            reflow_text,
            add_digital_signature,
            verify_signature,
            embed_font,
            add_form_field,
            add_calculated_field,
            export_xfdf,
            import_xfdf,
            convert_to_cmyk,
            embed_icc_profile,
            rgb_to_cmyk,
            cmyk_to_rgb,
            downsample_images,
            remove_metadata,
            flatten_content,
            detect_hardware_tokens,
            sign_with_hardware_token,
            verify_hardware_token_signature,
            pdf_to_images,
            images_to_pdf,
            html_to_pdf,
            repair_pdf,
            unlock_pdf,
            compress_pdf_quality,
            add_page_numbers,
            pdf_to_word,
            pdf_to_excel,
            pdf_to_powerpoint,
            create_pdf_portfolio,
            sanitize_document,
            convert_fonts_to_outlines,
            create_action_wizard,
            execute_action_wizard,
            aggregate_form_data,
            flatten_transparency,
            convert_to_pdfx,
            convert_to_pdfx_standard,
            validate_pdfx_compliance,
            check_accessibility,
            fix_accessibility_issues,
            preview_color_separations,
            render_color_separation,
            embed_javascript,
            add_bookmark_tree,
            visual_diff,
            list_digital_ids,
            get_text_blocks,
            edit_text_block,
            move_text_block,
            delete_text_block,
            get_fonts,
            replace_font,
            change_text_color,
            change_font_size,
            process_scanned_images,
            ocr_files,
            create_epub,
            create_searchable_pdf,
            repair_corrupt_pdf,
            enhance_scanned_pdf,
            compare_pdf_documents,
            session_open_pdf,
            session_close,
            session_get_bytes,
            session_rotate_page,
            session_delete_page,
            session_undo,
            session_redo,
            session_update_bytes,
            session_get_history_status,
            session_get_page_count,
            session_get_page_dimensions,
            session_get_text_blocks,
            session_get_metadata,
            session_get_bookmarks,
            session_get_form_fields,
            session_search_text,
            session_render_page_to_png,
            session_render_color_separation,
            session_verify_signature,
        ])
        .manage(session::SessionManager::new())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
