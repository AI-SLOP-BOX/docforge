use super::*;

// ===== SESSION LIFECYCLE & EDITING TAURI COMMANDS =====

#[tauri::command]
pub fn session_open_pdf(
    data: Vec<u8>,
    manager: tauri::State<'_, crate::session::SessionManager>,
) -> Result<String, String> {
    manager.create_session(&data)
}

#[tauri::command]
pub fn session_close(
    doc_id: String,
    manager: tauri::State<'_, crate::session::SessionManager>,
) -> Result<bool, String> {
    Ok(manager.close_session(&doc_id))
}

#[tauri::command]
pub fn session_get_bytes(
    doc_id: String,
    manager: tauri::State<'_, crate::session::SessionManager>,
) -> Result<Vec<u8>, String> {
    let session_arc = manager.get_session(&doc_id)?;
    let mut session = session_arc.write().map_err(|e| e.to_string())?;
    session.save_to_bytes()
}

#[tauri::command]
pub fn session_rotate_page(
    doc_id: String,
    page_index: usize,
    degrees: i32,
    manager: tauri::State<'_, crate::session::SessionManager>,
) -> Result<(), String> {
    let session_arc = manager.get_session(&doc_id)?;
    let mut session = session_arc.write().map_err(|e| e.to_string())?;

    let page_ids = crate::pdf_engine::get_page_ids(&session.doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }
    let page_id = page_ids[page_index];

    let current_rot = if let Some(lopdf::Object::Dictionary(ref dict)) = session.doc.objects.get(&page_id) {
        match dict.get(b"Rotate") {
            Ok(lopdf::Object::Integer(r)) => *r as i32,
            _ => 0,
        }
    } else {
        0
    };

    let new_rot = (current_rot + degrees).rem_euclid(360);
    session.mutate(|doc| crate::pdf_engine::rotate_page_in_doc(doc, page_index, degrees))?;

    session.push_undo(crate::session::EditCommand::RotatePage {
        page: page_index,
        from_degrees: current_rot,
        to_degrees: new_rot,
    });

    Ok(())
}

#[tauri::command]
pub fn session_delete_page(
    doc_id: String,
    page_index: usize,
    manager: tauri::State<'_, crate::session::SessionManager>,
) -> Result<(), String> {
    let session_arc = manager.get_session(&doc_id)?;
    let mut session = session_arc.write().map_err(|e| e.to_string())?;

    // 1. Take snapshot before modification
    let snapshot = session.save_to_bytes()?;

    // 2. Perform deletion on document model via mutate (auto-invalidates cache)
    session.mutate(|doc| crate::pdf_engine::delete_page_in_doc(doc, page_index))?;

    // 3. Only if deletion succeeded, push undo snapshot
    session.push_undo(crate::session::EditCommand::FullSnapshot {
        description: format!("Delete page {}", page_index + 1),
        data: snapshot,
    });

    Ok(())
}

#[tauri::command]
pub fn session_undo(
    doc_id: String,
    manager: tauri::State<'_, crate::session::SessionManager>,
) -> Result<bool, String> {
    let session_arc = manager.get_session(&doc_id)?;
    let mut session = session_arc.write().map_err(|e| e.to_string())?;
    session.undo()
}

#[tauri::command]
pub fn session_redo(
    doc_id: String,
    manager: tauri::State<'_, crate::session::SessionManager>,
) -> Result<bool, String> {
    let session_arc = manager.get_session(&doc_id)?;
    let mut session = session_arc.write().map_err(|e| e.to_string())?;
    session.redo()
}

#[tauri::command]
pub fn session_update_bytes(
    doc_id: String,
    description: String,
    data: Vec<u8>,
    manager: tauri::State<'_, crate::session::SessionManager>,
) -> Result<(), String> {
    let session_arc = manager.get_session(&doc_id)?;
    let mut session = session_arc.write().map_err(|e| e.to_string())?;

    // 1. Take snapshot of current state before applying new bytes
    let snapshot = session.save_to_bytes()?;

    // 2. Parse new document
    let new_doc = lopdf::Document::load_mem(&data).map_err(|e| format!("Failed to parse updated PDF: {e}"))?;

    // 3. Push undo snapshot and update doc in place
    session.push_undo(crate::session::EditCommand::FullSnapshot {
        description,
        data: snapshot,
    });
    session.doc = new_doc;
    session.invalidate_cache();
    Ok(())
}

#[tauri::command]
pub fn session_get_history_status(
    doc_id: String,
    manager: tauri::State<'_, crate::session::SessionManager>,
) -> Result<serde_json::Value, String> {
    let session_arc = manager.get_session(&doc_id)?;
    let session = session_arc.read().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "can_undo": !session.undo_stack.is_empty(),
        "can_redo": !session.redo_stack.is_empty(),
        "undo_count": session.undo_stack.len(),
        "redo_count": session.redo_stack.len(),
        "history_bytes": session.total_history_bytes,
    }))
}

// ===== SESSION QUERY & RENDER TAURI COMMANDS =====

#[tauri::command]
pub fn session_get_page_count(
    doc_id: String,
    manager: tauri::State<'_, crate::session::SessionManager>,
) -> Result<usize, String> {
    let session_arc = manager.get_session(&doc_id)?;
    let session = session_arc.read().map_err(|e| e.to_string())?;
    Ok(pdf_engine::get_page_ids(&session.doc).len())
}

#[tauri::command]
pub fn session_get_page_dimensions(
    doc_id: String,
    page_index: usize,
    manager: tauri::State<'_, crate::session::SessionManager>,
) -> Result<serde_json::Value, String> {
    let session_arc = manager.get_session(&doc_id)?;
    let session = session_arc.read().map_err(|e| e.to_string())?;
    let page_ids = pdf_engine::get_page_ids(&session.doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }
    let (w, h) = pdf_engine::get_page_dimensions(&session.doc, page_ids[page_index]);
    Ok(serde_json::json!({ "width": w, "height": h }))
}

#[tauri::command]
pub fn session_get_text_blocks(
    doc_id: String,
    page_index: usize,
    manager: tauri::State<'_, crate::session::SessionManager>,
) -> Result<Vec<pdf_engine::TextBlock>, String> {
    let session_arc = manager.get_session(&doc_id)?;
    let session = session_arc.read().map_err(|e| e.to_string())?;
    pdf_engine::get_text_blocks_from_doc(&session.doc, page_index)
}

#[tauri::command]
pub fn session_get_metadata(
    doc_id: String,
    manager: tauri::State<'_, crate::session::SessionManager>,
) -> Result<serde_json::Value, String> {
    let session_arc = manager.get_session(&doc_id)?;
    let session = session_arc.read().map_err(|e| e.to_string())?;
    pdf_engine::get_pdf_metadata_from_doc(&session.doc)
}

#[tauri::command]
pub fn session_get_bookmarks(
    doc_id: String,
    manager: tauri::State<'_, crate::session::SessionManager>,
) -> Result<Vec<serde_json::Value>, String> {
    let session_arc = manager.get_session(&doc_id)?;
    let session = session_arc.read().map_err(|e| e.to_string())?;
    pdf_engine::get_bookmarks_from_doc(&session.doc)
}

#[tauri::command]
pub fn session_get_form_fields(
    doc_id: String,
    manager: tauri::State<'_, crate::session::SessionManager>,
) -> Result<Vec<serde_json::Value>, String> {
    let session_arc = manager.get_session(&doc_id)?;
    let session = session_arc.read().map_err(|e| e.to_string())?;
    pdf_engine::get_form_fields_from_doc(&session.doc)
}

#[tauri::command]
pub fn session_search_text(
    doc_id: String,
    query: String,
    manager: tauri::State<'_, crate::session::SessionManager>,
) -> Result<Vec<serde_json::Value>, String> {
    let session_arc = manager.get_session(&doc_id)?;
    let session = session_arc.read().map_err(|e| e.to_string())?;
    pdf_engine::search_text_in_doc(&session.doc, &query)
}

#[tauri::command]
pub fn session_render_page_to_png(
    doc_id: String,
    page_index: usize,
    dpi: u32,
    manager: tauri::State<'_, crate::session::SessionManager>,
) -> Result<Vec<u8>, String> {
    let session_arc = manager.get_session(&doc_id)?;
    let mut session = session_arc.write().map_err(|e| e.to_string())?;
    let bytes = session.save_to_bytes()?;
    pdf_engine::render_page_to_png(&bytes, page_index, dpi)
}

#[tauri::command]
pub fn session_render_color_separation(
    doc_id: String,
    page_index: usize,
    dpi: u32,
    show_c: bool,
    show_m: bool,
    show_y: bool,
    show_k: bool,
    highlight_tac: bool,
    tac_limit: u32,
    manager: tauri::State<'_, crate::session::SessionManager>,
) -> Result<Vec<u8>, String> {
    let session_arc = manager.get_session(&doc_id)?;
    let mut session = session_arc.write().map_err(|e| e.to_string())?;
    let bytes = session.save_to_bytes()?;
    pdf_engine::render_color_separation(
        &bytes,
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
