use super::common::*;
use super::*;
use lopdf::{Dictionary, Document, Object, Stream};

// ===== PDF→IMAGE CONVERSION =====

pub fn pdf_to_images(
    data: &[u8],
    output_dir: &str,
    format: &str,
    dpi: u32,
) -> Result<Vec<String>, String> {
    let unique = format!(
        "docforge_pdf2img_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let tmp = std::env::temp_dir().join(unique);
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let input = tmp.join("input.pdf");
    std::fs::write(&input, data).map_err(|e| {
        let _ = std::fs::remove_dir_all(&tmp);
        e.to_string()
    })?;

    let mut cmd = std::process::Command::new("pdftoppm");
    cmd.arg("-r").arg(dpi.to_string());
    if format == "jpg" {
        cmd.arg("-jpeg");
    } else {
        cmd.arg("-png");
    }
    cmd.arg(&input).arg(tmp.join("page"));

    let output = cmd.output().map_err(|e| {
        let _ = std::fs::remove_dir_all(&tmp);
        format!("pdftoppm failed: {e}")
    })?;
    if !output.status.success() {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    // Move files to output_dir
    std::fs::create_dir_all(output_dir).map_err(|e| {
        let _ = std::fs::remove_dir_all(&tmp);
        e.to_string()
    })?;
    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&tmp) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("page") && (name.ends_with(".jpg") || name.ends_with(".png")) {
                let dest = std::path::Path::new(output_dir).join(&name);
                if std::fs::copy(entry.path(), &dest).is_ok() {
                    result.push(dest.to_string_lossy().to_string());
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    result.sort();
    Ok(result)
}

// ===== IMAGE→PDF CONVERSION =====

pub fn images_to_pdf(image_paths: &[String], output_path: &str) -> Result<(), String> {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.add_object(Object::Dictionary(Dictionary::new())); // placeholder

    let mut kids = Vec::new();

    for path in image_paths {
        let img_data = std::fs::read(path).map_err(|e| format!("Failed to read {path}: {e}"))?;
        let img = image::load_from_memory(&img_data).map_err(|e| e.to_string())?;
        let rgb = img.to_rgb8();
        let (width, height) = rgb.dimensions();

        let mut jpeg_buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut jpeg_buf, image::ImageFormat::Jpeg)
            .map_err(|e| format!("Failed to encode image to JPEG: {e}"))?;
        let jpeg_bytes = jpeg_buf.into_inner();

        // Create image XObject
        let mut img_dict = Dictionary::new();
        img_dict.set("Type", Object::Name("XObject".into()));
        img_dict.set("Subtype", Object::Name("Image".into()));
        img_dict.set("Width", Object::Integer(width as i64));
        img_dict.set("Height", Object::Integer(height as i64));
        img_dict.set("ColorSpace", Object::Name("DeviceRGB".into()));
        img_dict.set("BitsPerComponent", Object::Integer(8));
        img_dict.set("Filter", Object::Name("DCTDecode".into()));

        let img_stream = Stream::new(img_dict, jpeg_bytes);
        let img_id = doc.add_object(Object::Stream(img_stream));

        let pt_w = (width as f32 * 72.0 / 96.0).max(1.0);
        let pt_h = (height as f32 * 72.0 / 96.0).max(1.0);

        let mut xobj_dict = Dictionary::new();
        xobj_dict.set("Im1", Object::Reference(img_id));
        let mut res_dict = Dictionary::new();
        res_dict.set("XObject", Object::Dictionary(xobj_dict));

        let content_stream = format!("q {pt_w:.2} 0 0 {pt_h:.2} 0 0 cm /Im1 Do Q");
        let content_id = doc.add_object(Object::Stream(Stream::new(
            Dictionary::new(),
            content_stream.into_bytes(),
        )));

        // Create page
        let mut page_dict = Dictionary::new();
        page_dict.set("Type", Object::Name("Page".into()));
        page_dict.set("Parent", Object::Reference(pages_id));
        page_dict.set(
            "MediaBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(pt_w),
                Object::Real(pt_h),
            ]),
        );
        page_dict.set("Resources", Object::Dictionary(res_dict));
        page_dict.set("Contents", Object::Reference(content_id));

        let page_id = doc.add_object(Object::Dictionary(page_dict));
        kids.push(Object::Reference(page_id));
    }

    // Update Pages dict
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", Object::Name("Pages".into()));
    pages_dict.set("Kids", Object::Array(kids));
    pages_dict.set("Count", Object::Integer(image_paths.len() as i64));
    doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", Object::Name("Catalog".into()));
    catalog_dict.set("Pages", Object::Reference(pages_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog_dict));
    doc.trailer.set("Root", Object::Reference(catalog_id));

    let mut buf = Vec::new();
    doc.save_to(&mut buf).map_err(|e| e.to_string())?;
    std::fs::write(output_path, buf).map_err(|e| e.to_string())?;
    Ok(())
}

// ===== HTML→PDF CONVERSION =====

pub fn html_to_pdf(html_content: &str, output_path: &str) -> Result<(), String> {
    let unique = format!(
        "docforge_html2pdf_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let tmp = std::env::temp_dir().join(unique);
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let html_file = tmp.join("input.html");
    std::fs::write(&html_file, html_content).map_err(|e| {
        let _ = std::fs::remove_dir_all(&tmp);
        e.to_string()
    })?;

    // Try wkhtmltopdf (disable local-file-access by default for untrusted HTML security)
    let result = std::process::Command::new("wkhtmltopdf")
        .arg("--disable-local-file-access")
        .arg(&html_file)
        .arg(output_path)
        .output();

    let _ = std::fs::remove_dir_all(&tmp);

    match result {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(format!(
            "wkhtmltopdf failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )),
        Err(_) => {
            Err("wkhtmltopdf not installed. Install with: brew install wkhtmltopdf".to_string())
        }
    }
}

// ===== PDF REPAIR =====

pub fn repair_pdf(data: &[u8]) -> Result<Vec<u8>, String> {
    // Delegate directly to repair_corrupt_pdf which salvages surviving objects
    // and reconstructs a proper /Type /Catalog and /Type /Pages tree instead of
    // erroneously pointing Root to a Page object.
    super::repair::repair_corrupt_pdf(data)
}

// ===== QUALITY-BASED COMPRESSION =====

pub fn compress_pdf_quality(data: &[u8], quality: u8) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| e.to_string())?;

    // Remove metadata if quality is low
    if quality < 50 {
        doc.trailer.remove(b"Info");
    }

    let comp_level = match quality {
        0..=30 => 9,
        31..=60 => 6,
        61..=85 => 4,
        _ => 1,
    };

    // Rebuild all uncompressed or recompressible streams with flate2 compression
    for (_, obj) in doc.objects.iter_mut() {
        if let Object::Stream(ref mut stream) = obj {
            // Decompress existing Flate content if needed, then recompress with target level
            let raw_data = if let Ok(filter) = stream.dict.get(b"Filter") {
                if let Ok(filter_name) = filter.as_name() {
                    if filter_name == b"FlateDecode" {
                        if stream.decompress().is_err() {
                            // If decompression fails (corrupted or unsupported predictor), do NOT double-compress
                            continue;
                        }
                        stream.content.clone()
                    } else {
                        // Unknown or complex filter (e.g. DCTDecode for JPEG) - keep as is
                        continue;
                    }
                } else {
                    continue;
                }
            } else {
                stream.content.clone()
            };

            let mut encoder = flate2::write::ZlibEncoder::new(
                Vec::new(),
                flate2::Compression::new(comp_level),
            );
            if std::io::Write::write_all(&mut encoder, &raw_data).is_ok() {
                if let Ok(compressed) = encoder.finish() {
                    // Only replace if compression actually saved space or was uncompressed
                    if compressed.len() < stream.content.len() || stream.dict.get(b"Filter").is_err() {
                        stream.set_content(compressed);
                        stream.dict.set("Filter", Object::Name(b"FlateDecode".to_vec()));
                    }
                }
            }
        }
    }

    doc.prune_objects();
    save_doc(&mut doc)
}

// ===== PAGE NUMBERS =====

pub fn add_page_numbers(
    data: &[u8],
    position: &str, // "bottom-center", "top-right", etc.
    font_size: f32,
    start_number: usize,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| e.to_string())?;
    let page_ids = get_page_ids(&doc).clone();

    // Ensure /Helvetica font resource is created and referenced
    let mut font_dict = Dictionary::new();
    font_dict.set("Type", Object::Name(b"Font".to_vec()));
    font_dict.set("Subtype", Object::Name(b"Type1".to_vec()));
    font_dict.set("BaseFont", Object::Name(b"Helvetica".to_vec()));
    font_dict.set("Encoding", Object::Name(b"WinAnsiEncoding".to_vec()));
    let font_id = doc.add_object(Object::Dictionary(font_dict));

    for (i, &page_id) in page_ids.iter().enumerate() {
        let page_num = start_number + i;
        let (pw, ph) = get_page_dimensions(&doc, page_id);

        // Calculate position
        let (x, y) = match position {
            "top-left" => (50.0, ph - 30.0),
            "top-center" => (pw / 2.0, ph - 30.0),
            "top-right" => (pw - 50.0, ph - 30.0),
            "bottom-left" => (50.0, 30.0),
            "bottom-center" => (pw / 2.0, 30.0),
            "bottom-right" => (pw - 50.0, 30.0),
            _ => (pw / 2.0, 30.0),
        };

        let text = format!("{page_num}");

        // Format content stream safely wrapping graphics state
        let new_content = format!(
            " q BT /DocForgeHelv {} Tf {} {} Td ({}) Tj ET Q ",
            font_size, x, y, text
        );

        // Create independent new content stream object
        let mut new_stream = Stream::new(Dictionary::new(), new_content.into_bytes());
        new_stream.dict.set("Type", Object::Name("Content".into()));
        let new_cid = doc.add_object(new_stream);

        // Register /DocForgeHelv font into page's Resources and attach content stream
        // Safely extract or inherit existing Resources (direct or indirect) without blowing them away
        let mut resources_dict = if let Some(Object::Dictionary(page_dict)) = doc.objects.get(&page_id) {
            match page_dict.get(b"Resources") {
                Ok(Object::Dictionary(d)) => d.clone(),
                Ok(Object::Reference(res_ref)) => {
                    doc.objects.get(res_ref).and_then(|o| o.as_dict().ok()).cloned().unwrap_or_default()
                }
                _ => {
                    // Check parent for inherited Resources
                    let mut parent_id = page_dict.get(b"Parent").and_then(|p| p.as_reference()).ok();
                    let mut inherited_res = Dictionary::new();
                    while let Some(pid) = parent_id {
                        if let Some(Object::Dictionary(parent_dict)) = doc.objects.get(&pid) {
                            if let Ok(p_res) = parent_dict.get(b"Resources") {
                                if let Ok(d) = p_res.as_dict() {
                                    inherited_res = d.clone();
                                    break;
                                } else if let Ok(r_id) = p_res.as_reference() {
                                    if let Some(Object::Dictionary(d)) = doc.objects.get(&r_id) {
                                        inherited_res = d.clone();
                                        break;
                                    }
                                }
                            }
                            parent_id = parent_dict.get(b"Parent").and_then(|p| p.as_reference()).ok();
                        } else {
                            break;
                        }
                    }
                    inherited_res
                }
            }
        } else {
            Dictionary::new()
        };

        let mut fonts_dict = match resources_dict.get(b"Font") {
            Ok(Object::Dictionary(fd)) => fd.clone(),
            Ok(Object::Reference(f_ref)) => {
                doc.objects.get(f_ref).and_then(|o| o.as_dict().ok()).cloned().unwrap_or_default()
            }
            _ => Dictionary::new(),
        };
        fonts_dict.set("DocForgeHelv", Object::Reference(font_id));
        resources_dict.set("Font", Object::Dictionary(fonts_dict));

        if let Some(Object::Dictionary(ref mut page_dict)) = doc.objects.get_mut(&page_id) {
            page_dict.set("Resources", Object::Dictionary(resources_dict));

            // Combine Contents safely as an Array [existing..., new_cid]
            let updated_contents = match page_dict.get(b"Contents") {
                Ok(Object::Reference(orig_id)) => {
                    Object::Array(vec![Object::Reference(*orig_id), Object::Reference(new_cid)])
                }
                Ok(Object::Array(orig_arr)) => {
                    let mut arr = orig_arr.clone();
                    arr.push(Object::Reference(new_cid));
                    Object::Array(arr)
                }
                _ => Object::Reference(new_cid),
            };
            page_dict.set("Contents", updated_contents);
        }
    }

    save_doc(&mut doc)
}

// ===== EXPORT TO OFFICE & PORTFOLIO (Separated to export_office.rs) =====
pub use super::export_office::*;

// ===== ACTION WIZARD (Record & Replay) =====

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ActionStep {
    pub action_type: String,
    pub params: serde_json::Value,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ActionWizard {
    pub name: String,
    pub steps: Vec<ActionStep>,
}

pub fn create_action_wizard(name: &str, steps: &[ActionStep]) -> Result<String, String> {
    let wizard = ActionWizard {
        name: name.to_string(),
        steps: steps.to_vec(),
    };
    serde_json::to_string_pretty(&wizard).map_err(|e| e.to_string())
}

pub fn execute_action_wizard(data: &[u8], wizard_json: &str) -> Result<Vec<u8>, String> {
    let wizard: ActionWizard =
        serde_json::from_str(wizard_json).map_err(|e| format!("Invalid wizard JSON: {e}"))?;

    let mut current_data = data.to_vec();

    for step in &wizard.steps {
        match step.action_type.as_str() {
            "add_watermark" => {
                let text = step
                    .params
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let font_size = step
                    .params
                    .get("font_size")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(48.0) as f32;
                let color = step
                    .params
                    .get("color")
                    .and_then(|v| v.as_str())
                    .unwrap_or("#FF0000");
                let opacity = step
                    .params
                    .get("opacity")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.3) as f32;
                let rotation = step
                    .params
                    .get("rotation")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(45.0) as f32;
                current_data = add_watermark(
                    &current_data,
                    text,
                    opacity,
                    rotation,
                    font_size,
                    color,
                    true,
                    &[],
                )?;
            }
            "add_page_numbers" => {
                let position = step
                    .params
                    .get("position")
                    .and_then(|v| v.as_str())
                    .unwrap_or("bottom-center");
                let font_size = step
                    .params
                    .get("font_size")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(12.0) as f32;
                let start = step
                    .params
                    .get("start_number")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1) as usize;
                current_data = add_page_numbers(&current_data, position, font_size, start)?;
            }
            "optimize" => {
                current_data = optimize_pdf(&current_data)?;
            }
            "flatten_form" => {
                current_data = flatten_form(&current_data)?;
            }
            "remove_metadata" => {
                current_data = remove_metadata(&current_data)?;
            }
            "convert_to_pdfa" => {
                current_data = convert_to_pdfa(&current_data)?;
            }
            _ => {
                return Err(format!("Unknown action: {}", step.action_type));
            }
        }
    }

    Ok(current_data)
}

// ===== ACCESSIBILITY (Separated to accessibility.rs) =====
pub use super::accessibility::*;

// ===== JAVASCRIPT EMBEDDING =====

pub fn embed_javascript(data: &[u8], script: &str) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

    // Create JavaScript action
    let mut js_dict = Dictionary::new();
    js_dict.set("S", Object::Name("JavaScript".into()));
    js_dict.set(
        "JS",
        Object::String(script.as_bytes().to_vec(), lopdf::StringFormat::Literal),
    );

    let _js_id = doc.add_object(Object::Dictionary(js_dict));

    // Create OpenAction to run on document open
    let mut open_action = Dictionary::new();
    open_action.set("S", Object::Name("JavaScript".into()));
    open_action.set(
        "JS",
        Object::String(script.as_bytes().to_vec(), lopdf::StringFormat::Literal),
    );

    let open_action_id = doc.add_object(Object::Dictionary(open_action));

    // Add to root
    let root_id = if let Ok(id) = doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        id
    } else {
        let root_dict = Dictionary::new();
        doc.add_object(Object::Dictionary(root_dict))
    };

    if let Some(Object::Dictionary(ref mut root_dict)) = doc.objects.get_mut(&root_id) {
        root_dict.set("OpenAction", Object::Reference(open_action_id));
    }

    doc.trailer.set("Root", Object::Reference(root_id));

    save_doc(&mut doc)
}

// ===== BOOKMARK TREE =====

pub fn add_bookmark_tree(data: &[u8], bookmarks: &[serde_json::Value]) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

    let page_ids = get_page_ids(&doc);

    // Create outline dictionary
    let mut outline_items = Vec::new();

    for bm in bookmarks {
        let title = bm
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled");
        let page_idx = bm.get("page").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        if page_idx < page_ids.len() {
            let page_id = page_ids[page_idx];

            let mut item_dict = Dictionary::new();
            item_dict.set(
                "Title",
                Object::String(title.as_bytes().to_vec(), lopdf::StringFormat::Literal),
            );
            item_dict.set(
                "Dest",
                Object::Array(vec![
                    Object::Reference(page_id),
                    Object::Name("FitH".into()),
                    Object::Real(0.0),
                ]),
            );

            let item_id = doc.add_object(Object::Dictionary(item_dict));
            outline_items.push(Object::Reference(item_id));
        }
    }

    // Create outline
    let mut outline_dict = Dictionary::new();
    outline_dict.set("Type", Object::Name("Outlines".into()));
    outline_dict.set("Count", Object::Integer(outline_items.len() as i64));

    if let Some(first) = outline_items.first() {
        outline_dict.set("First", first.clone());
    }
    if let Some(last) = outline_items.last() {
        outline_dict.set("Last", last.clone());
    }

    let outline_id = doc.add_object(Object::Dictionary(outline_dict));

    // Link items
    for i in 0..outline_items.len() {
        if let Object::Reference(item_id) = outline_items[i] {
            if let Some(Object::Dictionary(ref mut item_dict)) = doc.objects.get_mut(&item_id) {
                if i > 0 {
                    if let Object::Reference(prev_id) = outline_items[i - 1] {
                        item_dict.set("Prev", Object::Reference(prev_id));
                    }
                }
                if i < outline_items.len() - 1 {
                    if let Object::Reference(next_id) = outline_items[i + 1] {
                        item_dict.set("Next", Object::Reference(next_id));
                    }
                }
                item_dict.set("Parent", Object::Reference(outline_id));
            }
        }
    }

    // Add to root
    let root_id = if let Ok(id) = doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        id
    } else {
        let root_dict = Dictionary::new();
        doc.add_object(Object::Dictionary(root_dict))
    };

    if let Some(Object::Dictionary(ref mut root_dict)) = doc.objects.get_mut(&root_id) {
        root_dict.set("Outlines", Object::Reference(outline_id));
    }

    doc.trailer.set("Root", Object::Reference(root_id));

    save_doc(&mut doc)
}

// ===== CONTENT COMPARISON & OCR (Separated to ocr_layout.rs) =====
pub use super::ocr_layout::*;
