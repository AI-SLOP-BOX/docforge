use lopdf::{Document, Object, Stream, Dictionary};
use super::common::*;

// ===== TEXT EDITING & REFLOW =====

pub fn edit_text(
    data: &[u8],
    page_index: usize,
    search_text: &str,
    replacement: &str,
    _font_name: &str,
    _font_size: f32,
    color: &str,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let (_r, _g, _b) = parse_hex_color(color, (0.0, 0.0, 0.0));

    let page_id = page_ids[page_index];

    // Get existing content stream
    let mut existing_operations = Vec::new();
    if let Some(Object::Dictionary(ref dict)) = doc.objects.get(&page_id) {
        if let Ok(Object::Reference(contents_id)) = dict.get(b"Contents") {
            if let Some(Object::Stream(stream)) = doc.objects.get(contents_id) {
                if let Ok(content) = lopdf::content::Content::decode(&stream.content) {
                    existing_operations = content.operations;
                }
            }
        }
    }

    // Find and replace text in content stream
    let mut new_operations = Vec::new();
    let mut skip_next = false;

    for (_i, op) in existing_operations.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }

        match op.operator.as_str() {
            "Tj" => {
                // Single text string
                if let Some(Object::String(bytes, _)) = op.operands.first() {
                    let text = String::from_utf8_lossy(bytes);
                    if text.contains(search_text) {
                        let new_text = text.replace(search_text, replacement);
                        new_operations.push(lopdf::content::Operation::new(
                            "Tj",
                            vec![Object::String(new_text.as_bytes().to_vec(), lopdf::StringFormat::Literal)],
                        ));
                        continue;
                    }
                }
                new_operations.push(op.clone());
            }
            "TJ" => {
                // Array of text strings
                if let Some(Object::Array(arr)) = op.operands.first() {
                    let mut new_arr = Vec::new();
                    let mut combined = String::new();

                    for item in arr {
                        match item {
                            Object::String(bytes, _) => {
                                combined.push_str(&String::from_utf8_lossy(bytes));
                            }
                            Object::Integer(kerning) => {
                                // Apply kerning
                                if !combined.is_empty() {
                                    if combined.contains(search_text) {
                                        let new_text = combined.replace(search_text, replacement);
                                        new_arr.push(Object::String(new_text.as_bytes().to_vec(), lopdf::StringFormat::Literal));
                                    } else {
                                        new_arr.push(Object::String(combined.as_bytes().to_vec(), lopdf::StringFormat::Literal));
                                    }
                                    combined.clear();
                                }
                                new_arr.push(Object::Integer(*kerning));
                            }
                            _ => new_arr.push(item.clone()),
                        }
                    }

                    if !combined.is_empty() {
                        if combined.contains(search_text) {
                            let new_text = combined.replace(search_text, replacement);
                            new_arr.push(Object::String(new_text.as_bytes().to_vec(), lopdf::StringFormat::Literal));
                        } else {
                            new_arr.push(Object::String(combined.as_bytes().to_vec(), lopdf::StringFormat::Literal));
                        }
                    }

                    new_operations.push(lopdf::content::Operation::new("TJ", vec![Object::Array(new_arr)]));
                    continue;
                }
                new_operations.push(op.clone());
            }
            "Tf" => {
                // Font change - check if we need to update
                new_operations.push(op.clone());
            }
            _ => {
                new_operations.push(op.clone());
            }
        }
    }

    // Create new content stream
    let content = lopdf::content::Content { operations: new_operations };
    let content_bytes = content.encode().map_err(|e| format!("Encode error: {e}"))?;

    let mut stream = Stream::new(Dictionary::new(), content_bytes);
    stream.dict.set("Type", Object::Name("Content".into()));
    let content_id = doc.add_object(stream);

    if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
        dict.set("Contents", Object::Reference(content_id));
    }

    save_doc(&mut doc)
}

pub fn get_text_positions(
    data: &[u8],
    page_index: usize,
) -> Result<Vec<serde_json::Value>, String> {
    let doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let page_id = page_ids[page_index];
    let mut positions = Vec::new();

    if let Some(Object::Dictionary(ref dict)) = doc.objects.get(&page_id) {
        if let Ok(Object::Reference(contents_id)) = dict.get(b"Contents") {
            if let Some(Object::Stream(stream)) = doc.objects.get(contents_id) {
                if let Ok(content) = lopdf::content::Content::decode(&stream.content) {
                    let mut current_x = 0.0;
                    let mut current_y = 0.0;
                    let mut current_font_size = 12.0;

                    for op in &content.operations {
                        match op.operator.as_str() {
                            "Tf" => {
                                if op.operands.len() >= 2 {
                                    if let Object::Real(size) = op.operands[1] {
                                        current_font_size = size;
                                    }
                                }
                            }
                            "Td" | "TD" => {
                                if op.operands.len() >= 2 {
                                    if let Object::Real(dx) = op.operands[0] {
                                        if let Object::Real(dy) = op.operands[1] {
                                            current_x += dx as f64;
                                            current_y += dy as f64;
                                        }
                                    }
                                }
                            }
                            "Tm" => {
                                if op.operands.len() >= 6 {
                                    if let Object::Real(x) = op.operands[4] {
                                        if let Object::Real(y) = op.operands[5] {
                                            current_x = x as f64;
                                            current_y = y as f64;
                                        }
                                    }
                                }
                            }
                            "Tj" | "TJ" => {
                                let text = match &op.operands[0] {
                                    Object::String(bytes, _) => String::from_utf8_lossy(bytes).to_string(),
                                    Object::Array(arr) => {
                                        let mut s = String::new();
                                        for item in arr {
                                            if let Object::String(bytes, _) = item {
                                                s.push_str(&String::from_utf8_lossy(bytes));
                                            }
                                        }
                                        s
                                    }
                                    _ => continue,
                                };

                                if !text.is_empty() {
                                    positions.push(serde_json::json!({
                                        "text": text,
                                        "x": current_x,
                                        "y": current_y,
                                        "font_size": current_font_size,
                                        "width": text.len() as f64 * current_font_size as f64 * 0.6,
                                        "height": current_font_size as f64,
                                    }));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    Ok(positions)
}


// ===== FONT EMBEDDING & SUBSETTING =====

pub fn embed_font(
    data: &[u8],
    page_index: usize,
    font_path: &str,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    // Read font file
    let font_data = std::fs::read(font_path)
        .map_err(|e| format!("Failed to read font: {e}"))?;

    // Create font descriptor
    let mut font_dict = Dictionary::new();
    font_dict.set("Type", Object::Name("Font".into()));
    font_dict.set("Subtype", Object::Name("TrueType".into()));
    font_dict.set("BaseFont", Object::Name("CustomFont".into()));
    font_dict.set("Encoding", Object::Name("WinAnsiEncoding".into()));

    let font_id = doc.add_object(Object::Dictionary(font_dict));

    // Add font to page resources
    let page_id = page_ids[page_index];
    if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
        let mut resources = match dict.get(b"Resources") {
            Ok(Object::Dictionary(r)) => r.clone(),
            _ => Dictionary::new(),
        };
        let mut fonts = match resources.get(b"Font") {
            Ok(Object::Dictionary(f)) => f.clone(),
            _ => Dictionary::new(),
        };
        fonts.set("CustomFont", Object::Reference(font_id));
        resources.set("Font", Object::Dictionary(fonts));
        dict.set("Resources", Object::Dictionary(resources));
    }

    // Store font data as a stream
    let mut font_stream_dict = Dictionary::new();
    font_stream_dict.set("Length", Object::Integer(font_data.len() as i64));
    let font_stream = Stream::new(font_stream_dict, font_data);
    let _font_stream_id = doc.add_object(font_stream);

    save_doc(&mut doc)
}

// ===== REFLOW (Paragraph Re-layout) =====
pub use super::reflow::*;



// ===== ADVANCED TEXT EDITING (Separated to text_block_ops.rs) =====
pub use super::text_block_ops::*;

// ===== FONT & STYLING MANAGEMENT (Separated to font_style.rs) =====
pub use super::font_style::*;

