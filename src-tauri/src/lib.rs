pub mod pdf_engine;
pub mod image_engine;
pub mod ocr_engine;
pub mod commands_text;
pub mod commands_io;
pub mod commands_prod;
pub use commands_text::*;
pub use commands_io::*;
pub use commands_prod::*;

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
fn add_text(data: Vec<u8>, page_index: usize, text: String, x: f64, y: f64, size: f64, color: String) -> Result<Vec<u8>, String> {
    pdf_engine::add_text(&data, page_index, &text, x, y, size, &color)
}

#[tauri::command]
fn add_image_to_page(data: Vec<u8>, page_index: usize, image_data: Vec<u8>, x: f64, y: f64, width: f64, height: f64) -> Result<Vec<u8>, String> {
    pdf_engine::add_image_to_page(&data, page_index, &image_data, x, y, width, height)
}

#[tauri::command]
fn crop_page(data: Vec<u8>, page_index: usize, x: f64, y: f64, width: f64, height: f64) -> Result<Vec<u8>, String> {
    pdf_engine::crop_page(&data, page_index, x, y, width, height)
}

// ===== WATERMARK =====

#[tauri::command]
fn add_watermark(data: Vec<u8>, text: String, opacity: f32, rotation: f32, font_size: f32, color: String, all_pages: bool, page_indices: Vec<usize>) -> Result<Vec<u8>, String> {
    pdf_engine::add_watermark(&data, &text, opacity, rotation, font_size, &color, all_pages, &page_indices)
}

#[tauri::command]
fn remove_watermarks(data: Vec<u8>) -> Result<Vec<u8>, String> {
    pdf_engine::remove_watermarks(&data)
}

// ===== ANNOTATIONS =====

#[tauri::command]
fn add_highlight(data: Vec<u8>, page_index: usize, x: f64, y: f64, width: f64, height: f64, color: String) -> Result<Vec<u8>, String> {
    pdf_engine::add_highlight(&data, page_index, x, y, width, height, &color)
}

#[tauri::command]
fn add_underline(data: Vec<u8>, page_index: usize, x: f64, y: f64, width: f64, color: String) -> Result<Vec<u8>, String> {
    pdf_engine::add_underline(&data, page_index, x, y, width, &color)
}

#[tauri::command]
fn add_sticky_note(data: Vec<u8>, page_index: usize, x: f64, y: f64, text: String, color: String) -> Result<Vec<u8>, String> {
    pdf_engine::add_sticky_note(&data, page_index, x, y, &text, &color)
}

#[tauri::command]
fn add_rectangle(data: Vec<u8>, page_index: usize, x: f64, y: f64, width: f64, height: f64, stroke_color: String, fill_color: String, stroke_width: f32) -> Result<Vec<u8>, String> {
    pdf_engine::add_rectangle(&data, page_index, x, y, width, height, &stroke_color, &fill_color, stroke_width)
}

#[tauri::command]
fn add_line(data: Vec<u8>, page_index: usize, x1: f64, y1: f64, x2: f64, y2: f64, color: String, width: f32) -> Result<Vec<u8>, String> {
    pdf_engine::add_line(&data, page_index, x1, y1, x2, y2, &color, width)
}

// ===== REDACTION =====

#[tauri::command]
fn redact_area(data: Vec<u8>, page_index: usize, x: f64, y: f64, width: f64, height: f64, color: String) -> Result<Vec<u8>, String> {
    pdf_engine::redact_area(&data, page_index, x, y, width, height, &color)
}

#[tauri::command]
fn redact_text(data: Vec<u8>, search_text: String, replacement: String) -> Result<Vec<u8>, String> {
    pdf_engine::redact_text(&data, &search_text, &replacement)
}

// ===== HEADERS & FOOTERS =====

#[tauri::command]
fn add_header_footer(data: Vec<u8>, header_text: String, footer_text: String, font_size: f32, margin: f32) -> Result<Vec<u8>, String> {
    pdf_engine::add_header_footer(&data, &header_text, &footer_text, font_size, margin)
}

// ===== BOOKMARKS =====

#[tauri::command]
fn add_bookmark(data: Vec<u8>, title: String, page_index: usize) -> Result<Vec<u8>, String> {
    pdf_engine::add_bookmark(&data, &title, page_index)
}

// ===== BATES NUMBERING =====

#[tauri::command]
fn add_bates_number(data: Vec<u8>, prefix: String, start_number: usize, font_size: f32, margin: f32) -> Result<Vec<u8>, String> {
    pdf_engine::add_bates_number(&data, &prefix, start_number, font_size, margin)
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
    Ok(serde_json::to_value(result).unwrap())
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
fn add_stamp(data: Vec<u8>, page_index: usize, text: String, x: f64, y: f64, rotation: f32, color: String, font_size: f32) -> Result<Vec<u8>, String> {
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
fn process_scanned_images(paths: Vec<String>, remove_shadow: bool, correct_perspective: bool, dpi: u32) -> Result<Vec<u8>, String> {
    image_engine::process_scanned_images(&paths, remove_shadow, correct_perspective, dpi)
}

// ===== OCR =====

#[tauri::command]
fn ocr_files(paths: Vec<String>, language: String) -> Result<serde_json::Value, String> {
    let result = ocr_engine::ocr_files(&paths, &language)?;
    Ok(serde_json::to_value(result).unwrap())
}

#[tauri::command]
fn create_epub(text: String, output_path: String, title: String) -> Result<(), String> {
    ocr_engine::create_epub(&text, &output_path, &title)
}

#[tauri::command]
fn create_searchable_pdf(original_paths: Vec<String>, ocr_text: String, output_path: String) -> Result<(), String> {
    ocr_engine::create_searchable_pdf(&original_paths, &ocr_text, &output_path)
}

// ===== DEEP REDACTION =====

#[tauri::command]
fn deep_redact(data: Vec<u8>, page_index: usize, x: f64, y: f64, width: f64, height: f64, color: String) -> Result<Vec<u8>, String> {
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
fn add_annotation_reply(data: Vec<u8>, annotation_id: (u32, u16), author: String, contents: String) -> Result<Vec<u8>, String> {
    pdf_engine::add_annotation_reply(&data, annotation_id, &author, &contents)
}

#[tauri::command]
fn set_annotation_status(data: Vec<u8>, annotation_id: (u32, u16), status: String) -> Result<Vec<u8>, String> {
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

#[tauri::command]
fn add_digital_signature(data: Vec<u8>, page_index: usize, x: f64, y: f64, width: f64, height: f64, signer_name: String, reason: String) -> Result<Vec<u8>, String> {
    pdf_engine::add_digital_signature(&data, page_index, x, y, width, height, &signer_name, &reason, None)
}

#[tauri::command]
fn verify_signature(data: Vec<u8>, signature_index: usize) -> Result<serde_json::Value, String> {
    pdf_engine::verify_signature(&data, signature_index)
}

// ===== FONT =====

#[tauri::command]
fn embed_font(data: Vec<u8>, page_index: usize, font_path: String) -> Result<Vec<u8>, String> {
    pdf_engine::embed_font(&data, page_index, &font_path)
}

// ===== FORMS =====

#[tauri::command]
fn add_form_field(data: Vec<u8>, page_index: usize, field_name: String, field_type: String, x: f64, y: f64, width: f64, height: f64, default_value: String) -> Result<Vec<u8>, String> {
    pdf_engine::add_form_field(&data, page_index, &field_name, &field_type, x, y, width, height, &default_value)
}

#[tauri::command]
fn add_calculated_field(data: Vec<u8>, page_index: usize, field_name: String, formula: String, x: f64, y: f64, width: f64, height: f64) -> Result<Vec<u8>, String> {
    pdf_engine::add_calculated_field(&data, page_index, &field_name, &formula, x, y, width, height)
}

// ===== XFDF/FDF =====

#[tauri::command]
fn export_xfdf(data: Vec<u8>) -> Result<String, String> {
    pdf_engine::export_xfdf(&data)
}

#[tauri::command]
fn import_xfdf(data: Vec<u8>, xfdf_content: String) -> Result<Vec<u8>, String> {
    pdf_engine::import_xfdf(&data, &xfdf_content)
}

// ===== HARDWARE TOKEN =====

#[tauri::command]
fn detect_hardware_tokens() -> Result<Vec<serde_json::Value>, String> {
    let tokens = pdf_engine::detect_hardware_tokens()?;
    let result: Vec<serde_json::Value> = tokens.iter().map(|t| {
        serde_json::json!({
            "slot_id": t.slot_id,
            "label": t.label,
            "manufacturer": t.manufacturer,
            "serial": t.serial,
            "initialized": t.initialized,
        })
    }).collect();
    Ok(result)
}

#[tauri::command]
fn sign_with_hardware_token(data: Vec<u8>, slot_id: u32, pin: String, page_index: usize, x: f64, y: f64, width: f64, height: f64, signer_name: String, reason: String) -> Result<Vec<u8>, String> {
    pdf_engine::sign_with_hardware_token(&data, slot_id, &pin, page_index, x, y, width, height, &signer_name, &reason)
}

#[tauri::command]
fn verify_hardware_token_signature(data: Vec<u8>, slot_id: u32) -> Result<serde_json::Value, String> {
    pdf_engine::verify_hardware_token_signature(&data, slot_id)
}

// ===== PDF REPAIR =====

#[tauri::command]
fn repair_pdf(data: Vec<u8>) -> Result<Vec<u8>, String> {
    pdf_engine::repair_pdf(&data)
}

// ===== PDF UNLOCK =====

#[tauri::command]
fn unlock_pdf(data: Vec<u8>, password: String) -> Result<Vec<u8>, String> {
    pdf_engine::unlock_pdf(&data, &password)
}

// ===== QUALITY COMPRESSION =====

#[tauri::command]
fn compress_pdf_quality(data: Vec<u8>, quality: u8) -> Result<Vec<u8>, String> {
    pdf_engine::compress_pdf_quality(&data, quality)
}

// ===== PAGE NUMBERS =====

#[tauri::command]
fn add_page_numbers(data: Vec<u8>, position: String, font_size: f32, start_number: usize) -> Result<Vec<u8>, String> {
    pdf_engine::add_page_numbers(&data, &position, font_size, start_number)
}

// ===== ACROBAT PRO EXCLUSIVE FEATURES =====

#[tauri::command]
fn create_action_wizard(name: String, steps: Vec<serde_json::Value>) -> Result<String, String> {
    let action_steps: Vec<pdf_engine::ActionStep> = steps.iter().map(|s| {
        pdf_engine::ActionStep {
            action_type: s.get("action_type").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            params: s.get("params").cloned().unwrap_or(serde_json::Value::Null),
        }
    }).collect();
    pdf_engine::create_action_wizard(&name, &action_steps)
}

#[tauri::command]
fn execute_action_wizard(data: Vec<u8>, wizard_json: String) -> Result<Vec<u8>, String> {
    pdf_engine::execute_action_wizard(&data, &wizard_json)
}

#[tauri::command]
fn aggregate_form_data(pdf_paths: Vec<String>) -> Result<serde_json::Value, String> {
    pdf_engine::aggregate_form_data(&pdf_paths)
}

#[tauri::command]
fn embed_javascript(data: Vec<u8>, script: String) -> Result<Vec<u8>, String> {
    pdf_engine::embed_javascript(&data, &script)
}

#[tauri::command]
fn add_bookmark_tree(data: Vec<u8>, bookmarks: Vec<serde_json::Value>) -> Result<Vec<u8>, String> {
    pdf_engine::add_bookmark_tree(&data, &bookmarks)
}

#[tauri::command]
fn visual_diff(data1: Vec<u8>, data2: Vec<u8>, output_path: String) -> Result<(), String> {
    pdf_engine::visual_diff(&data1, &data2, &output_path)
}

#[tauri::command]
fn list_digital_ids() -> Result<Vec<pdf_engine::DigitalID>, String> {
    pdf_engine::list_digital_ids()
}



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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
