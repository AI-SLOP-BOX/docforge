use super::*;

// ===== ADVANCED TEXT EDITING TAURI COMMANDS =====

#[tauri::command]
pub fn get_text_blocks(data: Vec<u8>, page_index: usize) -> Result<Vec<pdf_engine::TextBlock>, String> {
    pdf_engine::get_text_blocks(&data, page_index)
}

#[tauri::command]
pub fn edit_text_block(data: Vec<u8>, page_index: usize, block_id: usize, new_text: String) -> Result<Vec<u8>, String> {
    pdf_engine::edit_text_block(&data, page_index, block_id, &new_text)
}

#[tauri::command]
pub fn move_text_block(data: Vec<u8>, page_index: usize, block_id: usize, new_x: f32, new_y: f32) -> Result<Vec<u8>, String> {
    pdf_engine::move_text_block(&data, page_index, block_id, new_x, new_y)
}

#[tauri::command]
pub fn delete_text_block(data: Vec<u8>, page_index: usize, block_id: usize) -> Result<Vec<u8>, String> {
    pdf_engine::delete_text_block(&data, page_index, block_id)
}

#[tauri::command]
pub fn get_fonts(data: Vec<u8>) -> Result<Vec<serde_json::Value>, String> {
    pdf_engine::get_fonts(&data)
}

#[tauri::command]
pub fn replace_font(data: Vec<u8>, old_font: String, new_font: String) -> Result<Vec<u8>, String> {
    pdf_engine::replace_font(&data, &old_font, &new_font)
}

#[tauri::command]
pub fn change_text_color(data: Vec<u8>, page_index: usize, old_color: String, new_color: String) -> Result<Vec<u8>, String> {
    pdf_engine::change_text_color(&data, page_index, &old_color, &new_color)
}

#[tauri::command]
pub fn change_font_size(data: Vec<u8>, page_index: usize, old_size: f32, new_size: f32) -> Result<Vec<u8>, String> {
    pdf_engine::change_font_size(&data, page_index, old_size, new_size)
}

#[tauri::command]
pub fn edit_text(
    data: Vec<u8>,
    page_index: usize,
    search_text: String,
    replacement: String,
    font_name: String,
    font_size: f32,
    color: String,
) -> Result<Vec<u8>, String> {
    pdf_engine::edit_text(&data, page_index, &search_text, &replacement, &font_name, font_size, &color)
}

#[tauri::command]
pub fn get_text_positions(data: Vec<u8>, page_index: usize) -> Result<Vec<serde_json::Value>, String> {
    pdf_engine::get_text_positions(&data, page_index)
}

#[tauri::command]
pub fn reflow_text(
    data: Vec<u8>,
    page_index: usize,
    new_text: String,
    start_x: f64,
    start_y: f64,
    max_width: f64,
    font_size: f32,
    line_height: f32,
    color: String,
) -> Result<Vec<u8>, String> {
    pdf_engine::reflow_text(&data, page_index, &new_text, start_x, start_y, max_width, font_size, line_height, &color)
}
