use super::common::*;
use lopdf::{Dictionary, Document, Object, Stream};

// ===== REDACTION =====

pub fn redact_area(
    data: &[u8],
    page_index: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    color: &str,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let (r, g, b) = parse_hex_color(color, (0.0, 0.0, 0.0));

    let operations = vec![
        lopdf::content::Operation::new("q", vec![]),
        lopdf::content::Operation::new(
            "rg",
            vec![Object::Real(r), Object::Real(g), Object::Real(b)],
        ),
        lopdf::content::Operation::new(
            "re",
            vec![
                Object::Real(x as f32),
                Object::Real(y as f32),
                Object::Real(width as f32),
                Object::Real(height as f32),
            ],
        ),
        lopdf::content::Operation::new("f", vec![]),
        lopdf::content::Operation::new("Q", vec![]),
    ];

    let content = lopdf::content::Content { operations };
    let content_bytes = content.encode().map_err(|e| format!("Encode error: {e}"))?;

    let mut stream = Stream::new(Dictionary::new(), content_bytes);
    stream.dict.set("Type", Object::Name("Content".into()));
    let content_id = doc.add_object(stream);

    let page_id = page_ids[page_index];
    if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
        dict.set("Contents", Object::Reference(content_id));
    }

    save_doc(&mut doc)
}

pub fn redact_text(data: &[u8], search_text: &str, _replacement: &str) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc).clone();

    // Collect references first to avoid borrow issues
    let mut refs_to_modify: Vec<OID> = Vec::new();
    let mut replacement_text = String::new();

    for &page_id in &page_ids {
        if let Some(Object::Dictionary(ref dict)) = doc.objects.get(&page_id) {
            if let Ok(Object::Array(annots)) = dict.get(b"Annots") {
                for annot_ref in annots {
                    if let Object::Reference(ref_id) = annot_ref {
                        if let Some(Object::Dictionary(ref annot_dict)) = doc.objects.get(ref_id) {
                            if let Ok(Object::String(bytes, _)) = annot_dict.get(b"Contents") {
                                let content_str = String::from_utf8_lossy(bytes);
                                if content_str.contains(search_text) {
                                    refs_to_modify.push(*ref_id);
                                    replacement_text = _replacement.to_string();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Now modify the collected references
    for ref_id in refs_to_modify {
        if let Some(Object::Dictionary(ref mut annot_dict)) = doc.objects.get_mut(&ref_id) {
            annot_dict.set(
                "Contents",
                Object::String(
                    replacement_text.as_bytes().to_vec(),
                    lopdf::StringFormat::Literal,
                ),
            );
        }
    }

    save_doc(&mut doc)
}

// ===== DEEP REDACTION (Complete Data Purging - Permanent Removal) =====

pub fn deep_redact(
    data: &[u8],
    page_index: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    color: &str,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let (r, g, b) = parse_hex_color(color, (0.0, 0.0, 0.0));

    let page_id = page_ids[page_index];

    // Step 1: Get existing content stream and remove text in redacted area
    let content_id = if let Some(Object::Dictionary(ref dict)) = doc.objects.get(&page_id) {
        dict.get(b"Contents")
            .ok()
            .and_then(|o| o.as_reference().ok())
    } else {
        None
    };

    let mut new_operations = Vec::new();

    if let Some(content_id) = content_id {
        if let Some(Object::Stream(stream)) = doc.objects.get(&content_id) {
            if let Ok(content) = lopdf::content::Content::decode(&stream.content) {
                let mut current_x = 0.0f32;
                let mut current_y = 0.0f32;
                let mut in_text = false;

                for op in &content.operations {
                    match op.operator.as_str() {
                        "BT" => {
                            in_text = true;
                            new_operations.push(op.clone());
                        }
                        "ET" => {
                            in_text = false;
                            new_operations.push(op.clone());
                        }
                        "Tm" => {
                            // Text matrix
                            if op.operands.len() >= 6 {
                                if let (
                                    Object::Real(_a),
                                    Object::Real(_b),
                                    Object::Real(_c),
                                    Object::Real(_d),
                                    Object::Real(e),
                                    Object::Real(f),
                                ) = (
                                    &op.operands[0],
                                    &op.operands[1],
                                    &op.operands[2],
                                    &op.operands[3],
                                    &op.operands[4],
                                    &op.operands[5],
                                ) {
                                    current_x = *e;
                                    current_y = *f;
                                }
                            }
                            new_operations.push(op.clone());
                        }
                        "Td" | "TD" => {
                            if let (Some(Object::Real(dx)), Some(Object::Real(dy))) =
                                (op.operands.first(), op.operands.get(1))
                            {
                                current_x += dx;
                                current_y += dy;
                            }
                            new_operations.push(op.clone());
                        }
                        "Tj" | "TJ" => {
                            if in_text {
                                // Check if text is in redacted area
                                let text_in_area = current_x >= x as f32
                                    && current_x <= (x + width) as f32
                                    && current_y >= y as f32
                                    && current_y <= (y + height) as f32;

                                if text_in_area {
                                    // Skip this text operation (remove it completely)
                                    continue;
                                }
                            }
                            new_operations.push(op.clone());
                        }
                        _ => new_operations.push(op.clone()),
                    }
                }
            }
        }
    }

    // Step 2: Draw opaque redaction box
    new_operations.push(lopdf::content::Operation::new("q", vec![]));
    new_operations.push(lopdf::content::Operation::new(
        "rg",
        vec![Object::Real(r), Object::Real(g), Object::Real(b)],
    ));
    new_operations.push(lopdf::content::Operation::new(
        "re",
        vec![
            Object::Real(x as f32),
            Object::Real(y as f32),
            Object::Real(width as f32),
            Object::Real(height as f32),
        ],
    ));
    new_operations.push(lopdf::content::Operation::new("f", vec![]));
    new_operations.push(lopdf::content::Operation::new("Q", vec![]));

    let content = lopdf::content::Content {
        operations: new_operations,
    };
    let content_bytes = content.encode().map_err(|e| format!("Encode error: {e}"))?;

    let mut stream = Stream::new(Dictionary::new(), content_bytes);
    stream.dict.set("Type", Object::Name("Content".into()));
    let new_content_id = doc.add_object(stream);

    if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
        dict.set("Contents", Object::Reference(new_content_id));
    }

    // Step 3: Remove all annotations in the redacted area
    let mut annots_to_keep = Vec::new();
    let mut annots_to_remove = Vec::new();

    // First pass: collect which annotations to remove
    if let Some(Object::Dictionary(ref dict)) = doc.objects.get(&page_id) {
        if let Ok(Object::Array(annots)) = dict.get(b"Annots") {
            for annot_ref in annots {
                if let Object::Reference(ref_id) = annot_ref {
                    let mut should_remove = false;
                    if let Some(Object::Dictionary(annot_dict)) = doc.objects.get(ref_id) {
                        if let Ok(Object::Array(rect)) = annot_dict.get(b"Rect") {
                            if rect.len() >= 4 {
                                let ax = match &rect[0] {
                                    Object::Real(v) => *v as f64,
                                    Object::Integer(v) => *v as f64,
                                    _ => 0.0,
                                };
                                let ay = match &rect[1] {
                                    Object::Real(v) => *v as f64,
                                    Object::Integer(v) => *v as f64,
                                    _ => 0.0,
                                };
                                let aw = match &rect[2] {
                                    Object::Real(v) => *v as f64,
                                    Object::Integer(v) => *v as f64,
                                    _ => 0.0,
                                };
                                let ah = match &rect[3] {
                                    Object::Real(v) => *v as f64,
                                    Object::Integer(v) => *v as f64,
                                    _ => 0.0,
                                };

                                // Check if annotation overlaps with redaction area
                                if ax < x + width && ax + aw > x && ay < y + height && ay + ah > y {
                                    should_remove = true;
                                    annots_to_remove.push(*ref_id);
                                }
                            }
                        }
                    }
                    if !should_remove {
                        annots_to_keep.push(annot_ref.clone());
                    }
                }
            }
        }
    }

    // Second pass: remove the annotation objects
    for ref_id in annots_to_remove {
        doc.objects.remove(&ref_id);
    }

    // Update annotations array
    if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
        dict.set("Annots", Object::Array(annots_to_keep));
    }

    // Step 4: Remove metadata that might contain sensitive info
    doc.trailer.remove(b"Info");

    // Step 5: Remove any embedded files
    if let Ok(root_id) = doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        if let Some(Object::Dictionary(ref mut root_dict)) = doc.objects.get_mut(&root_id) {
            root_dict.remove(b"Names");
            root_dict.remove(b"Outlines");
        }
    }

    save_doc(&mut doc)
}

// ===== REDACTION WITH TEXT SEARCH =====

pub fn redact_text_deep(data: &[u8], search_text: &str, color: &str) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

    let (_r, _g, _b) = parse_hex_color(color, (0.0, 0.0, 0.0));

    let page_ids = get_page_ids(&doc).clone();

    for &page_id in &page_ids {
        // Get content stream
        let content_id = if let Some(Object::Dictionary(ref dict)) = doc.objects.get(&page_id) {
            dict.get(b"Contents")
                .ok()
                .and_then(|o| o.as_reference().ok())
        } else {
            None
        };

        let content_id = match content_id {
            Some(id) => id,
            None => continue,
        };

        let operations = if let Some(Object::Stream(stream)) = doc.objects.get(&content_id) {
            lopdf::content::Content::decode(&stream.content)
                .map(|c| c.operations)
                .unwrap_or_default()
        } else {
            continue;
        };

        let mut new_operations = Vec::new();
        let mut in_text = false;
        let mut current_x = 0.0f32;
        let mut current_y = 0.0f32;

        for op in &operations {
            match op.operator.as_str() {
                "BT" => {
                    in_text = true;
                    new_operations.push(op.clone());
                }
                "ET" => {
                    in_text = false;
                    new_operations.push(op.clone());
                }
                "Tm" => {
                    if op.operands.len() >= 6 {
                        if let Object::Real(e) = &op.operands[4] {
                            current_x = *e;
                        }
                        if let Object::Real(f) = &op.operands[5] {
                            current_y = *f;
                        }
                    }
                    new_operations.push(op.clone());
                }
                "Td" | "TD" => {
                    if let (Some(Object::Real(dx)), Some(Object::Real(dy))) =
                        (op.operands.first(), op.operands.get(1))
                    {
                        current_x += dx;
                        current_y += dy;
                    }
                    new_operations.push(op.clone());
                }
                "Tj" => {
                    if in_text {
                        if let Some(Object::String(bytes, _)) = op.operands.first() {
                            let text = String::from_utf8_lossy(bytes);
                            if text.contains(search_text) {
                                // Remove this text completely
                                continue;
                            }
                        }
                    }
                    new_operations.push(op.clone());
                }
                "TJ" => {
                    if in_text {
                        if let Some(Object::Array(arr)) = op.operands.first() {
                            let mut combined = String::new();
                            for item in arr {
                                if let Object::String(bytes, _) = item {
                                    combined.push_str(&String::from_utf8_lossy(bytes));
                                }
                            }
                            if combined.contains(search_text) {
                                continue;
                            }
                        }
                    }
                    new_operations.push(op.clone());
                }
                _ => new_operations.push(op.clone()),
            }
        }

        // Create new content stream
        let content = lopdf::content::Content {
            operations: new_operations,
        };
        let content_bytes = content.encode().map_err(|e| format!("Encode error: {e}"))?;

        let mut stream = Stream::new(Dictionary::new(), content_bytes);
        stream.dict.set("Type", Object::Name("Content".into()));
        let new_content_id = doc.add_object(stream);

        if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
            dict.set("Contents", Object::Reference(new_content_id));
        }
    }

    save_doc(&mut doc)
}
