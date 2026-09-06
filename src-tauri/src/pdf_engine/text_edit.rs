use super::common::*;
use lopdf::{Dictionary, Document, Object, Stream};

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
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let (_r, _g, _b) = parse_hex_color(color, (0.0, 0.0, 0.0));

    let page_id = page_ids[page_index];

    // Collect all content stream IDs
    let mut content_ids: Vec<OID> = Vec::new();
    let is_array = if let Some(Object::Dictionary(ref dict)) = doc.objects.get(&page_id) {
        match dict.get(b"Contents") {
            Ok(Object::Reference(id)) => {
                content_ids.push(*id);
                false
            }
            Ok(Object::Array(arr)) => {
                for o in arr {
                    if let Ok(id) = o.as_reference() {
                        content_ids.push(id);
                    }
                }
                true
            }
            _ => false,
        }
    } else {
        false
    };

    let mut modified_any = false;

    for cid in content_ids {
        let stream_bytes = if let Some(Object::Stream(stream)) = doc.objects.get(&cid) {
            stream
                .decompressed_content()
                .unwrap_or_else(|_| stream.content.clone())
        } else {
            continue;
        };

        let content = match lopdf::content::Content::decode(&stream_bytes) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let mut new_operations = Vec::new();
        let mut modified = false;

        for op in content.operations {
            match op.operator.as_str() {
                "Tj" => {
                    if let Some(Object::String(bytes, _)) = op.operands.first() {
                        let text = String::from_utf8_lossy(bytes);
                        if text.contains(search_text) {
                            let new_text = text.replace(search_text, replacement);
                            new_operations.push(lopdf::content::Operation::new(
                                "Tj",
                                vec![Object::String(
                                    new_text.into_bytes(),
                                    lopdf::StringFormat::Literal,
                                )],
                            ));
                            modified = true;
                            continue;
                        }
                    }
                    new_operations.push(op);
                }
                "TJ" => {
                    if let Some(Object::Array(arr)) = op.operands.first() {
                        let mut new_arr = Vec::new();
                        let mut combined = String::new();

                        for item in arr {
                            match item {
                                Object::String(bytes, _) => {
                                    combined.push_str(&String::from_utf8_lossy(bytes));
                                }
                                Object::Integer(kerning) => {
                                    if !combined.is_empty() {
                                        if combined.contains(search_text) {
                                            let new_text =
                                                combined.replace(search_text, replacement);
                                            new_arr.push(Object::String(
                                                new_text.into_bytes(),
                                                lopdf::StringFormat::Literal,
                                            ));
                                            modified = true;
                                        } else {
                                            new_arr.push(Object::String(
                                                combined.as_bytes().to_vec(),
                                                lopdf::StringFormat::Literal,
                                            ));
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
                                new_arr.push(Object::String(
                                    new_text.into_bytes(),
                                    lopdf::StringFormat::Literal,
                                ));
                                modified = true;
                            } else {
                                new_arr.push(Object::String(
                                    combined.into_bytes(),
                                    lopdf::StringFormat::Literal,
                                ));
                            }
                        }

                        new_operations.push(lopdf::content::Operation::new(
                            "TJ",
                            vec![Object::Array(new_arr)],
                        ));
                        continue;
                    }
                    new_operations.push(op);
                }
                _ => new_operations.push(op),
            }
        }

        if modified {
            modified_any = true;
            let updated_content = lopdf::content::Content {
                operations: new_operations,
            };
            if let Ok(encoded) = updated_content.encode() {
                if let Some(Object::Stream(ref mut st)) = doc.objects.get_mut(&cid) {
                    st.set_content(encoded);
                }
            }
        }
    }

    let _ = is_array;
    let _ = modified_any;

    save_doc(&mut doc)
}

pub fn get_text_positions(
    data: &[u8],
    page_index: usize,
) -> Result<Vec<serde_json::Value>, String> {
    let doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
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
                                    Object::String(bytes, _) => {
                                        String::from_utf8_lossy(bytes).to_string()
                                    }
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

pub fn embed_font(data: &[u8], page_index: usize, font_path: &str) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    // Read font file
    let font_data = std::fs::read(font_path).map_err(|e| format!("Failed to read font: {e}"))?;
    let font_file_len = font_data.len();

    let font_name = std::path::Path::new(font_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("CustomEmbeddedFont");

    let parsed = super::font_unicode::ParsedTrueTypeFont::parse(font_data.clone(), font_name).ok();

    // 1. FontFile2 stream
    let mut font_stream_dict = Dictionary::new();
    font_stream_dict.set("Length", Object::Integer(font_file_len as i64));
    font_stream_dict.set("Length1", Object::Integer(font_file_len as i64));
    let font_stream = Stream::new(font_stream_dict, font_data);
    let font_stream_id = doc.add_object(font_stream);

    // 2. FontDescriptor
    let mut descriptor = Dictionary::new();
    descriptor.set("Type", Object::Name("FontDescriptor".into()));
    descriptor.set("FontName", Object::Name(font_name.as_bytes().to_vec()));
    descriptor.set("Flags", Object::Integer(32)); // Non-symbolic

    let (bbox, ascent, descent, cap_height) = if let Some(ref p) = parsed {
        (
            vec![
                Object::Real(p.bbox[0]),
                Object::Real(p.bbox[1]),
                Object::Real(p.bbox[2]),
                Object::Real(p.bbox[3]),
            ],
            p.ascent as f32,
            p.descent as f32,
            p.cap_height as f32,
        )
    } else {
        (
            vec![
                Object::Real(-500.0),
                Object::Real(-300.0),
                Object::Real(1200.0),
                Object::Real(900.0),
            ],
            800.0,
            -200.0,
            700.0,
        )
    };

    descriptor.set("FontBBox", Object::Array(bbox));
    descriptor.set("ItalicAngle", Object::Real(0.0));
    descriptor.set("Ascent", Object::Real(ascent));
    descriptor.set("Descent", Object::Real(descent));
    descriptor.set("CapHeight", Object::Real(cap_height));
    descriptor.set("StemV", Object::Real(70.0));
    descriptor.set("FontFile2", Object::Reference(font_stream_id));

    let descriptor_id = doc.add_object(Object::Dictionary(descriptor));

    // 3. TrueType Font object
    let mut font_dict = Dictionary::new();
    font_dict.set("Type", Object::Name("Font".into()));
    font_dict.set("Subtype", Object::Name("TrueType".into()));
    font_dict.set("BaseFont", Object::Name(font_name.as_bytes().to_vec()));
    font_dict.set("Encoding", Object::Name("WinAnsiEncoding".into()));
    font_dict.set("FontDescriptor", Object::Reference(descriptor_id));
    font_dict.set("FirstChar", Object::Integer(32));
    font_dict.set("LastChar", Object::Integer(255));

    let widths: Vec<Object> = if let Some(ref p) = parsed {
        (32..=255)
            .map(|code| {
                let gid = p.get_gid(code as u8 as char);
                Object::Integer(p.get_glyph_width_1000(gid) as i64)
            })
            .collect()
    } else {
        (32..=255).map(|_| Object::Integer(500)).collect()
    };
    font_dict.set("Widths", Object::Array(widths));

    let font_id = doc.add_object(Object::Dictionary(font_dict));

    // 4. Add font to page resources safely without wiping other resources
    let page_id = page_ids[page_index];
    let mut resources = resolve_page_resources(&doc, page_id);
    let mut fonts = match resources.get(b"Font") {
        Ok(Object::Dictionary(f)) => f.clone(),
        Ok(Object::Reference(f_ref)) => doc
            .objects
            .get(f_ref)
            .and_then(|o| o.as_dict().ok())
            .cloned()
            .unwrap_or_default(),
        _ => Dictionary::new(),
    };
    fonts.set(font_name, Object::Reference(font_id));
    resources.set("Font", Object::Dictionary(fonts));

    if let Some(Object::Dictionary(ref mut page_dict)) = doc.objects.get_mut(&page_id) {
        page_dict.set("Resources", Object::Dictionary(resources));
    }

    save_doc(&mut doc)
}

// ===== REFLOW (Paragraph Re-layout) =====
pub use super::reflow::*;

// ===== ADVANCED TEXT EDITING (Separated to text_block_ops.rs) =====
pub use super::text_block_ops::*;

// ===== FONT & STYLING MANAGEMENT (Separated to font_style.rs) =====
pub use super::font_style::*;
