use super::common::*;
use lopdf::{Dictionary, Document, Object, Stream};

// ===== BATCH PROCESSING & PAGE FORMATTING (Separated to batch_ops.rs) =====
pub use super::batch_ops::*;

// ===== OPTIMIZE =====

pub fn optimize_pdf(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

    // Remove duplicate objects by comparing their string representations
    let mut seen: std::collections::HashMap<String, OID> = std::collections::HashMap::new();
    let mut duplicates: Vec<OID> = Vec::new();

    for (&id, obj) in doc.objects.iter() {
        let repr = format!("{:?}", obj);
        if let Some(&dup_id) = seen.get(&repr) {
            duplicates.push(id);
            let _ = dup_id;
        } else {
            seen.insert(repr, id);
        }
    }

    for dup_id in &duplicates {
        doc.objects.remove(dup_id);
    }

    save_doc(&mut doc)
}

// ===== COMPARE =====

#[derive(serde::Serialize)]
pub struct CompareResult {
    pub page_count_diff: bool,
    pub pages_same: usize,
    pub pages_different: usize,
    pub size_diff: bool,
    pub original_size: usize,
    pub modified_size: usize,
}

pub fn compare_pdfs(data1: &[u8], data2: &[u8]) -> Result<CompareResult, String> {
    let doc1 = Document::load_mem(data1).map_err(|e| format!("Failed to load PDF1: {e}"))?;
    let doc2 = Document::load_mem(data2).map_err(|e| format!("Failed to load PDF2: {e}"))?;

    let pages1 = get_page_ids(&doc1);
    let pages2 = get_page_ids(&doc2);

    let page_count_diff = pages1.len() != pages2.len();
    let mut pages_same = 0;
    let mut pages_different = 0;

    let min_pages = pages1.len().min(pages2.len());
    for i in 0..min_pages {
        if let (Some(obj1), Some(obj2)) =
            (doc1.objects.get(&pages1[i]), doc2.objects.get(&pages2[i]))
        {
            let repr1 = format!("{:?}", obj1);
            let repr2 = format!("{:?}", obj2);
            if repr1 == repr2 {
                pages_same += 1;
            } else {
                pages_different += 1;
            }
        } else {
            pages_different += 1;
        }
    }

    Ok(CompareResult {
        page_count_diff,
        pages_same,
        pages_different,
        size_diff: data1.len() != data2.len(),
        original_size: data1.len(),
        modified_size: data2.len(),
    })
}

// ===== PDF RENDERING =====

pub fn get_page_count_from_data(data: &[u8]) -> Result<usize, String> {
    let doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    Ok(get_page_ids(&doc).len())
}

pub fn get_page_dimensions_from_data(data: &[u8], page_index: usize) -> Result<(f32, f32), String> {
    let doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }
    Ok(get_page_dimensions(&doc, page_ids[page_index]))
}

pub fn render_page_to_png(data: &[u8], page_index: usize, dpi: u32) -> Result<Vec<u8>, String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();

    let temp_dir = std::env::temp_dir();
    let temp_pdf = temp_dir.join(format!("docforge_{pid}_{id}.pdf"));
    let temp_prefix = temp_dir.join(format!("docforge_page_{pid}_{id}"));

    std::fs::write(&temp_pdf, data).map_err(|e| format!("Failed to write temp PDF: {e}"))?;

    let output = std::process::Command::new("pdftoppm")
        .args([
            "-png",
            "-r",
            &dpi.to_string(),
            "-f",
            &(page_index + 1).to_string(),
            "-l",
            &(page_index + 1).to_string(),
            temp_pdf.to_str().unwrap_or(""),
            temp_prefix.to_str().unwrap_or(""),
        ])
        .output();

    let _ = std::fs::remove_file(&temp_pdf);

    let output = match output {
        Ok(out) => out,
        Err(e) => {
            return Err(format!(
                "Failed to execute pdftoppm: {e}. Ensure poppler is installed."
            ))
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pdftoppm failed: {stderr}"));
    }

    // Match output file (pdftoppm creates format: prefix-1.png, prefix-01.png, or prefix-000001.png)
    let mut found_file = None;
    if let Ok(entries) = std::fs::read_dir(&temp_dir) {
        let prefix_stem = temp_prefix
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(stem) = path.file_stem() {
                if stem.to_string_lossy().starts_with(&*prefix_stem)
                    && path.extension().and_then(|s| s.to_str()) == Some("png")
                {
                    found_file = Some(path);
                    break;
                }
            }
        }
    }

    let png_path = found_file
        .ok_or_else(|| "Failed to locate rendered PNG output from pdftoppm".to_string())?;
    let png_data =
        std::fs::read(&png_path).map_err(|e| format!("Failed to read rendered PNG: {e}"))?;
    let _ = std::fs::remove_file(&png_path);

    Ok(png_data)
}

pub fn get_page_text(data: &[u8], page_index: usize) -> Result<String, String> {
    let doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let page_id = page_ids[page_index];
    let mut text = String::new();

    if let Some(page_obj) = doc.objects.get(&page_id) {
        if let Ok(page_dict) = page_obj.as_dict() {
            if let Ok(Object::Reference(contents_id)) = page_dict.get(b"Contents") {
                if let Some(Object::Stream(stream)) = doc.objects.get(contents_id) {
                    if let Ok(content) = lopdf::content::Content::decode(&stream.content) {
                        for op in &content.operations {
                            match op.operator.as_str() {
                                "Tj" | "TJ" => {
                                    for param in &op.operands {
                                        if let Object::String(bytes, _) = param {
                                            text.push_str(&String::from_utf8_lossy(bytes));
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
    }

    Ok(text)
}

pub fn search_text_in_doc(doc: &Document, query: &str) -> Result<Vec<serde_json::Value>, String> {
    let page_ids = get_page_ids(doc);
    let mut results = Vec::new();

    for (i, &page_id) in page_ids.iter().enumerate() {
        if let Some(page_obj) = doc.objects.get(&page_id) {
            if let Ok(page_dict) = page_obj.as_dict() {
                if let Ok(Object::Reference(contents_id)) = page_dict.get(b"Contents") {
                    if let Some(Object::Stream(stream)) = doc.objects.get(contents_id) {
                        if let Ok(content) = lopdf::content::Content::decode(&stream.content) {
                            let mut page_text = String::new();
                            for op in &content.operations {
                                match op.operator.as_str() {
                                    "Tj" | "TJ" => {
                                        for param in &op.operands {
                                            if let Object::String(bytes, _) = param {
                                                page_text.push_str(&String::from_utf8_lossy(bytes));
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            if page_text.to_lowercase().contains(&query.to_lowercase()) {
                                results.push(serde_json::json!({
                                    "page": i,
                                    "text": page_text,
                                }));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(results)
}

pub fn search_text(data: &[u8], query: &str) -> Result<Vec<serde_json::Value>, String> {
    let doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    search_text_in_doc(&doc, query)
}

pub fn get_bookmarks_from_doc(doc: &Document) -> Result<Vec<serde_json::Value>, String> {
    let mut bookmarks = Vec::new();

    let root_id = match doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        Ok(id) => id,
        Err(_) => return Ok(bookmarks),
    };

    if let Some(root) = doc.objects.get(&root_id) {
        if let Ok(root_dict) = root.as_dict() {
            if let Ok(Object::Reference(outline_id)) = root_dict.get(b"Outlines") {
                if let Some(Object::Dictionary(outline_dict)) = doc.objects.get(&outline_id) {
                    if let Ok(Object::Array(first_refs)) = outline_dict.get(b"First") {
                        for item_ref in first_refs {
                            if let Object::Reference(item_id) = item_ref {
                                if let Some(Object::Dictionary(item)) = doc.objects.get(&item_id) {
                                    let title = item
                                        .get(b"Title")
                                        .ok()
                                        .and_then(|o| match o {
                                            Object::String(bytes, _) => {
                                                Some(String::from_utf8_lossy(bytes).to_string())
                                            }
                                            _ => None,
                                        })
                                        .unwrap_or_else(|| "Untitled".to_string());
                                    bookmarks.push(serde_json::json!({
                                        "title": title,
                                        "page": 0,
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(bookmarks)
}

pub fn get_bookmarks(data: &[u8]) -> Result<Vec<serde_json::Value>, String> {
    let doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    get_bookmarks_from_doc(&doc)
}

pub fn get_form_fields_from_doc(doc: &Document) -> Result<Vec<serde_json::Value>, String> {
    let mut fields = Vec::new();

    let root_id = match doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        Ok(id) => id,
        Err(_) => return Ok(fields),
    };

    if let Some(root) = doc.objects.get(&root_id) {
        if let Ok(root_dict) = root.as_dict() {
            if let Ok(Object::Reference(acroform_id)) = root_dict.get(b"AcroForm") {
                if let Some(Object::Dictionary(acroform)) = doc.objects.get(&acroform_id) {
                    if let Ok(Object::Array(field_refs)) = acroform.get(b"Fields") {
                        for field_ref in field_refs {
                            if let Object::Reference(field_id) = field_ref {
                                if let Some(Object::Dictionary(field)) = doc.objects.get(&field_id)
                                {
                                    let name = field
                                        .get(b"T")
                                        .ok()
                                        .and_then(|o| match o {
                                            Object::String(bytes, _) => {
                                                Some(String::from_utf8_lossy(bytes).to_string())
                                            }
                                            _ => None,
                                        })
                                        .unwrap_or_default();
                                    let field_type = field
                                        .get(b"FT")
                                        .ok()
                                        .and_then(|o| match o {
                                            Object::Name(bytes) => {
                                                Some(String::from_utf8_lossy(bytes).to_string())
                                            }
                                            _ => None,
                                        })
                                        .unwrap_or_default();
                                    let value = field
                                        .get(b"V")
                                        .ok()
                                        .and_then(|o| match o {
                                            Object::String(bytes, _) => {
                                                Some(String::from_utf8_lossy(bytes).to_string())
                                            }
                                            _ => None,
                                        })
                                        .unwrap_or_default();

                                    fields.push(serde_json::json!({
                                        "name": name,
                                        "type": field_type,
                                        "value": value,
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(fields)
}

pub fn get_form_fields(data: &[u8]) -> Result<Vec<serde_json::Value>, String> {
    let doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    get_form_fields_from_doc(&doc)
}

pub fn set_form_field(data: &[u8], field_name: &str, value: &str) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

    let root_id = match doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        Ok(id) => id,
        Err(_) => return Err("No root".into()),
    };

    // Collect field IDs first to avoid borrow issues
    let mut target_field_id = None;

    if let Some(root) = doc.objects.get(&root_id) {
        if let Ok(root_dict) = root.as_dict() {
            if let Ok(Object::Reference(acroform_id)) = root_dict.get(b"AcroForm") {
                if let Some(Object::Dictionary(acroform)) = doc.objects.get(&acroform_id) {
                    if let Ok(Object::Array(field_refs)) = acroform.get(b"Fields") {
                        for field_ref in field_refs {
                            if let Object::Reference(field_id) = field_ref {
                                if let Some(Object::Dictionary(field)) = doc.objects.get(&field_id)
                                {
                                    if let Ok(Object::String(bytes, _)) = field.get(b"T") {
                                        let name = String::from_utf8_lossy(bytes);
                                        if name == field_name {
                                            target_field_id = Some(*field_id);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Now update the field
    if let Some(field_id) = target_field_id {
        if let Some(Object::Dictionary(ref mut f)) = doc.objects.get_mut(&field_id) {
            f.set(
                "V",
                Object::String(value.as_bytes().to_vec(), lopdf::StringFormat::Literal),
            );
        }
    }

    save_doc(&mut doc)
}

pub fn flatten_form(data: &[u8]) -> Result<Vec<u8>, String> {
    // Simplified: just remove AcroForm
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

    let root_id = match doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        Ok(id) => id,
        Err(_) => return save_doc(&mut doc),
    };

    if let Some(root) = doc.objects.get_mut(&root_id) {
        if let Ok(dict) = root.as_dict_mut() {
            dict.remove(b"AcroForm");
        }
    }

    save_doc(&mut doc)
}

pub fn add_stamp(
    data: &[u8],
    page_index: usize,
    text: &str,
    x: f64,
    y: f64,
    rotation: f32,
    color: &str,
    font_size: f32,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let (r, g, b) = parse_hex_color(color, (1.0, 0.0, 0.0));

    let _rad = rotation * std::f32::consts::PI / 180.0;

    let operations = vec![
        lopdf::content::Operation::new("q", vec![]),
        lopdf::content::Operation::new("BT", vec![]),
        lopdf::content::Operation::new(
            "Tf",
            vec![
                Object::Name("Helvetica-Bold".into()),
                Object::Real(font_size),
            ],
        ),
        lopdf::content::Operation::new(
            "rg",
            vec![Object::Real(r), Object::Real(g), Object::Real(b)],
        ),
        lopdf::content::Operation::new("Td", vec![Object::Real(x as f32), Object::Real(y as f32)]),
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

    let page_id = page_ids[page_index];
    if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
        dict.set("Contents", Object::Reference(content_id));
    }

    save_doc(&mut doc)
}

pub fn print_pdf(data: &[u8]) -> Result<(), String> {
    let temp_dir = std::env::temp_dir();
    let temp_pdf = temp_dir.join("docforge_print.pdf");

    std::fs::write(&temp_pdf, data).map_err(|e| format!("Failed to write temp: {e}"))?;

    let pdf_str = temp_pdf
        .to_str()
        .ok_or_else(|| "Invalid temp path".to_string())?;

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-p", pdf_str])
            .spawn()
            .map_err(|e| format!("Failed to print: {e}"))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", "/p", pdf_str])
            .spawn()
            .map_err(|e| format!("Failed to print: {e}"))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("lp")
            .arg(pdf_str)
            .spawn()
            .map_err(|e| format!("Failed to print: {e}"))?;
    }

    Ok(())
}

pub fn get_pdf_metadata_from_doc(doc: &Document) -> Result<serde_json::Value, String> {
    let page_count = get_page_ids(doc).len();

    let mut title = String::new();
    let mut author = String::new();

    if let Ok(Object::Reference(info_id)) = doc.trailer.get(b"Info") {
        if let Some(Object::Dictionary(info)) = doc.objects.get(&info_id) {
            if let Ok(Object::String(bytes, _)) = info.get(b"Title") {
                title = String::from_utf8_lossy(bytes).to_string();
            }
            if let Ok(Object::String(bytes, _)) = info.get(b"Author") {
                author = String::from_utf8_lossy(bytes).to_string();
            }
        }
    }

    Ok(serde_json::json!({
        "page_count": page_count,
        "title": title,
        "author": author,
        "version": doc.version,
    }))
}

pub fn get_pdf_metadata(data: &[u8]) -> Result<serde_json::Value, String> {
    let doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let mut val = get_pdf_metadata_from_doc(&doc)?;
    if let Some(obj) = val.as_object_mut() {
        obj.insert("size".to_string(), serde_json::json!(data.len()));
    }
    Ok(val)
}
