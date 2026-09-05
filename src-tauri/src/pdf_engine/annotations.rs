use super::common::*;
use lopdf::{Dictionary, Document, Object, Stream};

// ===== WATERMARK =====

pub fn add_watermark(
    data: &[u8],
    text: &str,
    _opacity: f32,
    rotation: f32,
    font_size: f32,
    color: &str,
    all_pages: bool,
    page_indices: &[usize],
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);

    let (r, g, b) = parse_hex_color(color, (0.5, 0.5, 0.5));

    let rad = rotation * std::f32::consts::PI / 180.0;
    let cos_r = rad.cos();
    let sin_r = rad.sin();

    for (i, &page_id) in page_ids.iter().enumerate() {
        if !all_pages && !page_indices.contains(&i) {
            continue;
        }

        let (pw, ph) = get_page_dimensions(&doc, page_id);

        let cx = pw / 2.0;
        let cy = ph / 2.0;

        let operations = vec![
            lopdf::content::Operation::new("q", vec![]),
            lopdf::content::Operation::new("cs", vec![Object::Name("DeviceRGB".into())]),
            lopdf::content::Operation::new(
                "sc",
                vec![Object::Real(r), Object::Real(g), Object::Real(b)],
            ),
            lopdf::content::Operation::new("gs", vec![Object::Name("GState".into())]),
            lopdf::content::Operation::new("BT", vec![]),
            lopdf::content::Operation::new(
                "Tf",
                vec![Object::Name("Helvetica".into()), Object::Real(font_size)],
            ),
            lopdf::content::Operation::new("Td", vec![Object::Real(cx - 80.0), Object::Real(cy)]),
            lopdf::content::Operation::new(
                "Tm",
                vec![
                    Object::Real(cos_r),
                    Object::Real(sin_r),
                    Object::Real(-sin_r),
                    Object::Real(cos_r),
                    Object::Real(cx),
                    Object::Real(cy),
                ],
            ),
            lopdf::content::Operation::new(
                "Tj",
                vec![Object::String(
                    text.as_bytes().to_vec(),
                    lopdf::StringFormat::Literal,
                )],
            ),
            lopdf::content::Operation::new("ET", vec![]),
            lopdf::content::Operation::new("Q", vec![]),
        ];

        let content = lopdf::content::Content { operations };
        let content_bytes = content.encode().map_err(|e| format!("Encode error: {e}"))?;

        let mut stream = Stream::new(Dictionary::new(), content_bytes);
        stream.dict.set("Type", Object::Name("Content".into()));
        let content_id = doc.add_object(stream);

        if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
            dict.set("Contents", Object::Reference(content_id));
        }
    }

    save_doc(&mut doc)
}

pub fn remove_watermarks(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

    for (_, obj) in doc.objects.iter_mut() {
        if let Object::Stream(ref mut stream) = obj {
            if let Ok(content) = lopdf::content::Content::decode(&stream.content) {
                let orig_len = content.operations.len();
                let filtered: Vec<_> = content
                    .operations
                    .into_iter()
                    .filter(|op| {
                        let lower = op.operator.to_lowercase();
                        !lower.contains("watermark")
                    })
                    .collect();
                if filtered.len() != orig_len {
                    let new_content = lopdf::content::Content {
                        operations: filtered,
                    };
                    if let Ok(encoded) = new_content.encode() {
                        stream.content = encoded;
                    }
                }
            }
        }
    }

    save_doc(&mut doc)
}

// ===== ANNOTATIONS =====

pub fn add_highlight(
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

    let (r, g, b) = parse_hex_color(color, (1.0, 1.0, 0.0));

    let mut annot_dict = Dictionary::new();
    annot_dict.set("Type", Object::Name("Annot".into()));
    annot_dict.set("Subtype", Object::Name("Highlight".into()));
    annot_dict.set(
        "Rect",
        Object::Array(vec![
            Object::Real(x as f32),
            Object::Real(y as f32),
            Object::Real((x + width) as f32),
            Object::Real((y + height) as f32),
        ]),
    );
    annot_dict.set(
        "C",
        Object::Array(vec![Object::Real(r), Object::Real(g), Object::Real(b)]),
    );
    annot_dict.set("F", Object::Integer(4));

    let annot_id = doc.add_object(Object::Dictionary(annot_dict));

    let page_id = page_ids[page_index];
    if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
        let mut annots = match dict.get(b"Annots") {
            Ok(Object::Array(a)) => a.clone(),
            _ => Vec::new(),
        };
        annots.push(Object::Reference(annot_id));
        dict.set("Annots", Object::Array(annots));
    }

    save_doc(&mut doc)
}

pub fn add_underline(
    data: &[u8],
    page_index: usize,
    x: f64,
    y: f64,
    width: f64,
    color: &str,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let (r, g, b) = parse_hex_color(color, (1.0, 0.0, 0.0));

    let mut annot_dict = Dictionary::new();
    annot_dict.set("Type", Object::Name("Annot".into()));
    annot_dict.set("Subtype", Object::Name("Underline".into()));
    annot_dict.set(
        "Rect",
        Object::Array(vec![
            Object::Real(x as f32),
            Object::Real(y as f32),
            Object::Real((x + width) as f32),
            Object::Real((y + 2.0) as f32),
        ]),
    );
    annot_dict.set(
        "C",
        Object::Array(vec![Object::Real(r), Object::Real(g), Object::Real(b)]),
    );
    annot_dict.set("F", Object::Integer(4));

    let annot_id = doc.add_object(Object::Dictionary(annot_dict));

    let page_id = page_ids[page_index];
    if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
        let mut annots = match dict.get(b"Annots") {
            Ok(Object::Array(a)) => a.clone(),
            _ => Vec::new(),
        };
        annots.push(Object::Reference(annot_id));
        dict.set("Annots", Object::Array(annots));
    }

    save_doc(&mut doc)
}

pub fn add_sticky_note(
    data: &[u8],
    page_index: usize,
    x: f64,
    y: f64,
    text: &str,
    color: &str,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let (r, g, b) = parse_hex_color(color, (1.0, 1.0, 0.0));

    let mut annot_dict = Dictionary::new();
    annot_dict.set("Type", Object::Name("Annot".into()));
    annot_dict.set("Subtype", Object::Name("Text".into()));
    annot_dict.set(
        "Rect",
        Object::Array(vec![
            Object::Real(x as f32),
            Object::Real(y as f32),
            Object::Real((x + 20.0) as f32),
            Object::Real((y + 20.0) as f32),
        ]),
    );
    annot_dict.set(
        "C",
        Object::Array(vec![Object::Real(r), Object::Real(g), Object::Real(b)]),
    );
    annot_dict.set(
        "Contents",
        Object::String(text.as_bytes().to_vec(), lopdf::StringFormat::Literal),
    );
    annot_dict.set("Open", Object::Boolean(true));

    let annot_id = doc.add_object(Object::Dictionary(annot_dict));

    let page_id = page_ids[page_index];
    if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
        let mut annots = match dict.get(b"Annots") {
            Ok(Object::Array(a)) => a.clone(),
            _ => Vec::new(),
        };
        annots.push(Object::Reference(annot_id));
        dict.set("Annots", Object::Array(annots));
    }

    save_doc(&mut doc)
}

pub fn add_rectangle(
    data: &[u8],
    page_index: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    stroke_color: &str,
    fill_color: &str,
    stroke_width: f32,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let (sr, sg, sb) = parse_hex_color(stroke_color, (0.0, 0.0, 0.0));
    let (fr, fg, fb) = parse_hex_color(fill_color, (1.0, 1.0, 1.0));

    let operations = vec![
        lopdf::content::Operation::new("q", vec![]),
        lopdf::content::Operation::new("w", vec![Object::Real(stroke_width)]),
        lopdf::content::Operation::new(
            "RG",
            vec![Object::Real(sr), Object::Real(sg), Object::Real(sb)],
        ),
        lopdf::content::Operation::new(
            "rg",
            vec![Object::Real(fr), Object::Real(fg), Object::Real(fb)],
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
        lopdf::content::Operation::new("B", vec![]),
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

pub fn add_line(
    data: &[u8],
    page_index: usize,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    color: &str,
    width: f32,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let (r, g, b) = parse_hex_color(color, (0.0, 0.0, 0.0));

    let operations = vec![
        lopdf::content::Operation::new("q", vec![]),
        lopdf::content::Operation::new("w", vec![Object::Real(width)]),
        lopdf::content::Operation::new(
            "RG",
            vec![Object::Real(r), Object::Real(g), Object::Real(b)],
        ),
        lopdf::content::Operation::new("m", vec![Object::Real(x1 as f32), Object::Real(y1 as f32)]),
        lopdf::content::Operation::new("l", vec![Object::Real(x2 as f32), Object::Real(y2 as f32)]),
        lopdf::content::Operation::new("S", vec![]),
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

// ===== REDACTION (Separated to redact.rs) =====
pub use super::redact::*;

// ===== ANNOTATION MANAGEMENT (Separated to annot_manage.rs) =====
pub use super::annot_manage::*;
