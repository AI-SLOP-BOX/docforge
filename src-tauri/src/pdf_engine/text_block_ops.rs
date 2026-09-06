use super::common::*;
use super::reflow::get_char_metric_width;
use lopdf::{Dictionary, Document, Object, Stream};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct TextBlock {
    pub id: usize,
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub font_name: String,
    pub font_size: f32,
    pub color: String,
    pub page_index: usize,
}

// Get all text blocks on a page for direct editing
pub fn get_text_blocks_from_doc(
    doc: &Document,
    page_index: usize,
) -> Result<Vec<TextBlock>, String> {
    let page_ids = get_page_ids(doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let page_id = page_ids[page_index];
    let mut blocks = Vec::new();
    let mut block_id = 0;

    if let Some(Object::Dictionary(ref dict)) = doc.objects.get(&page_id) {
        if let Ok(Object::Reference(content_id)) = dict.get(b"Contents") {
            if let Some(Object::Stream(stream)) = doc.objects.get(content_id) {
                if let Ok(content) = lopdf::content::Content::decode(&stream.content) {
                    let mut current_x = 0.0f32;
                    let mut current_y = 0.0f32;
                    let mut current_font = String::new();
                    let mut current_size = 12.0f32;
                    let mut current_color = "#000000".to_string();
                    let mut in_text = false;
                    let mut text_buffer = String::new();
                    let mut text_start_x = 0.0f32;
                    let mut text_start_y = 0.0f32;

                    for op in &content.operations {
                        match op.operator.as_str() {
                            "BT" => {
                                in_text = true;
                                text_buffer.clear();
                                text_start_x = current_x;
                                text_start_y = current_y;
                            }
                            "ET" => {
                                if in_text && !text_buffer.is_empty() {
                                    let calc_width: f32 = text_buffer
                                        .chars()
                                        .map(|c| get_char_metric_width(c, current_size))
                                        .sum();
                                    blocks.push(TextBlock {
                                        id: block_id,
                                        text: text_buffer.clone(),
                                        x: text_start_x,
                                        y: text_start_y - current_size,
                                        width: calc_width,
                                        height: current_size * 1.2,
                                        font_name: current_font.clone(),
                                        font_size: current_size,
                                        color: current_color.clone(),
                                        page_index,
                                    });
                                    block_id += 1;
                                }
                                in_text = false;
                                text_buffer.clear();
                            }
                            "Tf" => {
                                if let (Some(Object::Name(font)), Some(Object::Real(size))) =
                                    (op.operands.first(), op.operands.get(1))
                                {
                                    current_font = String::from_utf8_lossy(font).to_string();
                                    current_size = *size;
                                }
                            }
                            "Tm" => {
                                if op.operands.len() >= 6 {
                                    if let Object::Real(x) = &op.operands[4] {
                                        current_x = *x;
                                    }
                                    if let Object::Real(y) = &op.operands[5] {
                                        current_y = *y;
                                    }
                                }
                            }
                            "Td" | "TD" => {
                                if let (Some(Object::Real(dx)), Some(Object::Real(dy))) =
                                    (op.operands.first(), op.operands.get(1))
                                {
                                    current_x += dx;
                                    current_y += dy;
                                }
                            }
                            "rg" => {
                                if op.operands.len() >= 3 {
                                    if let (Object::Real(r), Object::Real(g), Object::Real(b)) =
                                        (&op.operands[0], &op.operands[1], &op.operands[2])
                                    {
                                        let ri = (r * 255.0) as u8;
                                        let gi = (g * 255.0) as u8;
                                        let bi = (b * 255.0) as u8;
                                        current_color = format!("#{:02x}{:02x}{:02x}", ri, gi, bi);
                                    }
                                }
                            }
                            "Tj" => {
                                if in_text {
                                    if let Some(Object::String(bytes, _)) = op.operands.first() {
                                        text_buffer.push_str(&String::from_utf8_lossy(bytes));
                                    }
                                }
                            }
                            "TJ" => {
                                if in_text {
                                    if let Some(Object::Array(arr)) = op.operands.first() {
                                        for item in arr {
                                            if let Object::String(bytes, _) = item {
                                                text_buffer
                                                    .push_str(&String::from_utf8_lossy(bytes));
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    Ok(blocks)
}

pub fn get_text_blocks(data: &[u8], page_index: usize) -> Result<Vec<TextBlock>, String> {
    let doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    get_text_blocks_from_doc(&doc, page_index)
}

// Edit a specific text block
pub fn edit_text_block(
    data: &[u8],
    page_index: usize,
    block_id: usize,
    new_text: &str,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let page_id = page_ids[page_index];

    let content_id = if let Some(Object::Dictionary(ref dict)) = doc.objects.get(&page_id) {
        dict.get(b"Contents")
            .ok()
            .and_then(|o| o.as_reference().ok())
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
    let mut current_block = 0;
    let mut in_text = false;
    let mut text_buffer = String::new();
    let mut in_target_block = false;
    let mut block_replaced = false;

    for op in &operations {
        match op.operator.as_str() {
            "BT" => {
                in_text = true;
                text_buffer.clear();
                in_target_block = current_block == block_id;
                new_operations.push(op.clone());
            }
            "ET" => {
                if in_text && current_block == block_id {
                    if !block_replaced {
                        new_operations.push(lopdf::content::Operation::new(
                            "Tj",
                            vec![Object::String(
                                new_text.as_bytes().to_vec(),
                                lopdf::StringFormat::Literal,
                            )],
                        ));
                        block_replaced = true;
                    }
                    current_block += 1;
                } else if in_text && !text_buffer.is_empty() {
                    current_block += 1;
                }
                in_text = false;
                in_target_block = false;
                text_buffer.clear();
                new_operations.push(op.clone());
            }
            "Tj" => {
                if in_text {
                    if let Some(Object::String(bytes, _)) = op.operands.first() {
                        text_buffer.push_str(&String::from_utf8_lossy(bytes));
                    }
                }
                if in_target_block {
                    if !block_replaced {
                        new_operations.push(lopdf::content::Operation::new(
                            "Tj",
                            vec![Object::String(
                                new_text.as_bytes().to_vec(),
                                lopdf::StringFormat::Literal,
                            )],
                        ));
                        block_replaced = true;
                    }
                } else {
                    new_operations.push(op.clone());
                }
            }
            "TJ" => {
                if in_text {
                    if let Some(Object::Array(arr)) = op.operands.first() {
                        for item in arr {
                            if let Object::String(bytes, _) = item {
                                text_buffer.push_str(&String::from_utf8_lossy(bytes));
                            }
                        }
                    }
                }
                if in_target_block {
                    if !block_replaced {
                        new_operations.push(lopdf::content::Operation::new(
                            "Tj",
                            vec![Object::String(
                                new_text.as_bytes().to_vec(),
                                lopdf::StringFormat::Literal,
                            )],
                        ));
                        block_replaced = true;
                    }
                } else {
                    new_operations.push(op.clone());
                }
            }
            _ => {
                new_operations.push(op.clone());
            }
        }
    }

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

    save_doc(&mut doc)
}

// Move a text block to new position
pub fn move_text_block(
    data: &[u8],
    page_index: usize,
    block_id: usize,
    new_x: f32,
    new_y: f32,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let page_id = page_ids[page_index];

    let content_id = if let Some(Object::Dictionary(ref dict)) = doc.objects.get(&page_id) {
        dict.get(b"Contents")
            .ok()
            .and_then(|o| o.as_reference().ok())
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
    let mut current_block = 0;
    let mut in_text = false;

    for op in &operations {
        match op.operator.as_str() {
            "BT" => {
                in_text = true;
                new_operations.push(op.clone());
            }
            "ET" => {
                in_text = false;
                if current_block == block_id {
                    current_block += 1;
                }
                new_operations.push(op.clone());
            }
            "Tm" => {
                if in_text && current_block == block_id {
                    new_operations.push(lopdf::content::Operation::new(
                        "Tm",
                        vec![
                            Object::Real(1.0),
                            Object::Real(0.0),
                            Object::Real(0.0),
                            Object::Real(1.0),
                            Object::Real(new_x),
                            Object::Real(new_y),
                        ],
                    ));
                } else {
                    new_operations.push(op.clone());
                }
            }
            "Td" | "TD" => {
                if in_text && current_block == block_id {
                    new_operations.push(lopdf::content::Operation::new(
                        "Tm",
                        vec![
                            Object::Real(1.0),
                            Object::Real(0.0),
                            Object::Real(0.0),
                            Object::Real(1.0),
                            Object::Real(new_x),
                            Object::Real(new_y),
                        ],
                    ));
                } else {
                    new_operations.push(op.clone());
                }
            }
            "Tj" | "TJ" => {
                if in_text && current_block == block_id {
                    current_block += 1;
                }
                new_operations.push(op.clone());
            }
            _ => {
                new_operations.push(op.clone());
            }
        }
    }

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

    save_doc(&mut doc)
}

// Delete a text block
pub fn delete_text_block(
    data: &[u8],
    page_index: usize,
    block_id: usize,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let page_id = page_ids[page_index];

    let content_id = if let Some(Object::Dictionary(ref dict)) = doc.objects.get(&page_id) {
        dict.get(b"Contents")
            .ok()
            .and_then(|o| o.as_reference().ok())
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
    let mut current_block = 0;
    let mut skip_block = false;

    for op in &operations {
        match op.operator.as_str() {
            "BT" => {
                skip_block = current_block == block_id;
                if !skip_block {
                    new_operations.push(op.clone());
                }
            }
            "ET" => {
                if !skip_block {
                    new_operations.push(op.clone());
                }
                current_block += 1;
            }
            _ => {
                if !skip_block {
                    new_operations.push(op.clone());
                }
            }
        }
    }

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

    save_doc(&mut doc)
}
