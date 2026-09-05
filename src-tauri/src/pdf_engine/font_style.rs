use lopdf::{Document, Object, Dictionary};
use super::common::*;

// ===== FONT & STYLING MANAGEMENT =====

// Get font information from PDF
pub fn get_fonts(data: &[u8]) -> Result<Vec<serde_json::Value>, String> {
    let doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;

    let mut fonts = Vec::new();

    // Search all objects for font dictionaries
    for (_, obj) in &doc.objects {
        if let Object::Dictionary(dict) = obj {
            if let Ok(Object::Name(font_type)) = dict.get(b"Type") {
                if font_type == b"Font" {
                    let mut font_info = serde_json::Map::new();
                    
                    if let Ok(Object::Name(subtype)) = dict.get(b"Subtype") {
                        font_info.insert("type".into(), serde_json::Value::String(
                            String::from_utf8_lossy(subtype).to_string()
                        ));
                    }
                    
                    if let Ok(Object::Name(base_font)) = dict.get(b"BaseFont") {
                        font_info.insert("name".into(), serde_json::Value::String(
                            String::from_utf8_lossy(base_font).to_string()
                        ));
                    }
                    
                    if let Ok(Object::Integer(encoding)) = dict.get(b"Encoding") {
                        font_info.insert("encoding".into(), serde_json::Value::Number((*encoding).into()));
                    }

                    fonts.push(serde_json::Value::Object(font_info));
                }
            }
        }
    }

    Ok(fonts)
}

// Replace font in entire document
pub fn replace_font(
    data: &[u8],
    old_font: &str,
    new_font: &str,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;

    // Update all font references
    for (_, obj) in doc.objects.iter_mut() {
        if let Object::Dictionary(dict) = obj {
            if let Ok(Object::Name(base_font)) = dict.get(b"BaseFont") {
                if String::from_utf8_lossy(base_font) == old_font {
                    dict.set("BaseFont", Object::Name(new_font.as_bytes().to_vec()));
                }
            }
        }
    }

    save_doc(&mut doc)
}

// Change text color across entire page
pub fn change_text_color(
    data: &[u8],
    page_index: usize,
    _old_color: &str,
    new_color: &str,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let page_id = page_ids[page_index];

    // Get content stream
    let content_id = if let Some(Object::Dictionary(ref dict)) = doc.objects.get(&page_id) {
        dict.get(b"Contents").ok().and_then(|o| o.as_reference().ok())
    } else {
        None
    };

    let content_id = match content_id {
        Some(id) => id,
        None => return Err("No content stream".into()),
    };

    let operations = if let Some(Object::Stream(stream)) = doc.objects.get(&content_id) {
        lopdf::content::Content::decode(&stream.content)
            .map(|c| c.operations)
            .unwrap_or_default()
    } else {
        return Err("Invalid content stream".into());
    };

    // Parse new color
    let (r, g, b) = parse_hex_color(new_color, (0.0, 0.0, 0.0));

    let mut new_operations = Vec::new();

    for op in &operations {
        match op.operator.as_str() {
            "rg" => {
                // Replace color operation
                new_operations.push(lopdf::content::Operation::new("rg", vec![
                    Object::Real(r), Object::Real(g), Object::Real(b)
                ]));
            }
            _ => {
                new_operations.push(op.clone());
            }
        }
    }

    // Create new content stream
    let content = lopdf::content::Content { operations: new_operations };
    let content_bytes = content.encode().map_err(|e| format!("Encode error: {e}"))?;

    let mut stream = lopdf::Stream::new(Dictionary::new(), content_bytes);
    stream.dict.set("Type", Object::Name("Content".into()));
    let new_content_id = doc.add_object(stream);

    if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
        dict.set("Contents", Object::Reference(new_content_id));
    }

    save_doc(&mut doc)
}

// Change font size for entire page
pub fn change_font_size(
    data: &[u8],
    page_index: usize,
    old_size: f32,
    new_size: f32,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let page_id = page_ids[page_index];

    // Get content stream
    let content_id = if let Some(Object::Dictionary(ref dict)) = doc.objects.get(&page_id) {
        dict.get(b"Contents").ok().and_then(|o| o.as_reference().ok())
    } else {
        None
    };

    let content_id = match content_id {
        Some(id) => id,
        None => return Err("No content stream".into()),
    };

    let operations = if let Some(Object::Stream(stream)) = doc.objects.get(&content_id) {
        lopdf::content::Content::decode(&stream.content)
            .map(|c| c.operations)
            .unwrap_or_default()
    } else {
        return Err("Invalid content stream".into());
    };

    let mut new_operations = Vec::new();

    for op in &operations {
        match op.operator.as_str() {
            "Tf" => {
                // Update font size if it matches
                if op.operands.len() >= 2 {
                    if let Object::Real(size) = &op.operands[1] {
                        if (*size - old_size).abs() < 0.1 {
                            new_operations.push(lopdf::content::Operation::new("Tf", vec![
                                op.operands[0].clone(),
                                Object::Real(new_size),
                            ]));
                            continue;
                        }
                    }
                }
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

    let mut stream = lopdf::Stream::new(Dictionary::new(), content_bytes);
    stream.dict.set("Type", Object::Name("Content".into()));
    let new_content_id = doc.add_object(stream);

    if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
        dict.set("Contents", Object::Reference(new_content_id));
    }

    save_doc(&mut doc)
}
