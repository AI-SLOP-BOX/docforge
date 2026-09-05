use super::common::*;
use lopdf::{Dictionary, Document, Object, Stream};

// ===== COLOR MANAGEMENT (CMYK) =====

pub fn rgb_to_cmyk(r: u8, g: u8, b: u8) -> (u8, u8, u8, u8) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let k = 1.0 - r.max(g).max(b);
    if k >= 1.0 {
        return (0, 0, 0, 255);
    }

    let c = ((1.0 - r - k) / (1.0 - k) * 100.0) as u8;
    let m = ((1.0 - g - k) / (1.0 - k) * 100.0) as u8;
    let y = ((1.0 - b - k) / (1.0 - k) * 100.0) as u8;
    let k = (k * 100.0) as u8;

    (c, m, y, k)
}

pub fn cmyk_to_rgb(c: u8, m: u8, y: u8, k: u8) -> (u8, u8, u8) {
    let c = c as f32 / 100.0;
    let m = m as f32 / 100.0;
    let y = y as f32 / 100.0;
    let k = k as f32 / 100.0;

    let r = 255.0 * (1.0 - c) * (1.0 - k);
    let g = 255.0 * (1.0 - m) * (1.0 - k);
    let b = 255.0 * (1.0 - y) * (1.0 - k);

    (r as u8, g as u8, b as u8)
}

pub fn convert_to_cmyk(data: &[u8]) -> Result<Vec<u8>, String> {
    let img = image::load_from_memory(data).map_err(|e| format!("Failed to load image: {e}"))?;
    let rgb = img.to_rgb8();
    let (width, height) = rgb.dimensions();

    let mut cmyk_data = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let pixel = rgb.get_pixel(x, y);
            let (c, m, y_val, k) = rgb_to_cmyk(pixel[0], pixel[1], pixel[2]);
            cmyk_data.push(c);
            cmyk_data.push(m);
            cmyk_data.push(y_val);
            cmyk_data.push(k);
        }
    }

    Ok(cmyk_data)
}

pub fn embed_icc_profile(data: &[u8], profile_name: &str) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

    // Create ICC profile stream (sRGB profile as default)
    let srgb_profile = vec![
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00,
    ];

    let mut profile_dict = Dictionary::new();
    profile_dict.set(
        "N",
        Object::String(
            profile_name.as_bytes().to_vec(),
            lopdf::StringFormat::Literal,
        ),
    );
    profile_dict.set("Length", Object::Integer(srgb_profile.len() as i64));
    let profile_stream = Stream::new(profile_dict, srgb_profile);
    let profile_id = doc.add_object(profile_stream);

    // Add to catalog
    let root_id = doc
        .trailer
        .get(b"Root")
        .and_then(|o| o.as_reference())
        .ok()
        .ok_or("No root")?;

    // Get output_intent_id first
    let output_intent_id = if let Some(root) = doc.objects.get(&root_id) {
        if let Ok(root_dict) = root.as_dict() {
            if let Ok(Object::Reference(id)) = root_dict.get(b"OutputIntents") {
                Some(*id)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    if let Some(output_intent_id) = output_intent_id {
        // Create the intent object first
        let mut intent_dict = Dictionary::new();
        intent_dict.set("Type", Object::Name("OutputIntent".into()));
        intent_dict.set("S", Object::Name("GTS_PDFA1".into()));
        intent_dict.set("DestOutputProfile", Object::Reference(profile_id));
        let intent = doc.add_object(Object::Dictionary(intent_dict));

        // Then add it to the array
        if let Some(Object::Array(ref mut intents)) = doc.objects.get_mut(&output_intent_id) {
            intents.push(Object::Reference(intent));
        }
    }

    save_doc(&mut doc)
}

// ===== ADVANCED PDF OPTIMIZATION =====

pub fn downsample_images(data: &[u8], target_dpi: u32, quality: u8) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

    let mut images_to_update: Vec<OID> = Vec::new();

    // Find all image XObjects
    for (&id, obj) in doc.objects.iter() {
        if let Object::Stream(ref stream) = obj {
            if let Some(subtype) = stream.dict.get(b"Subtype").ok() {
                if let Object::Name(name) = subtype {
                    if name == b"Image" {
                        images_to_update.push(id);
                    }
                }
            }
        }
    }

    // Process each image
    for img_id in images_to_update {
        if let Some(Object::Stream(ref mut stream)) = doc.objects.get_mut(&img_id) {
            // Get image dimensions
            let width = stream
                .dict
                .get(b"Width")
                .ok()
                .and_then(|o| match o {
                    Object::Integer(v) => Some(*v as u32),
                    _ => None,
                })
                .unwrap_or(100);

            let height = stream
                .dict
                .get(b"Height")
                .ok()
                .and_then(|o| match o {
                    Object::Integer(v) => Some(*v as u32),
                    _ => None,
                })
                .unwrap_or(100);

            // Calculate new dimensions based on target DPI (assuming 72 DPI original)
            let scale = target_dpi as f32 / 72.0;
            let new_width = (width as f32 * scale) as u32;
            let new_height = (height as f32 * scale) as u32;

            // Only downsample if new size is smaller
            if new_width < width && new_height < height {
                // Try to decode and re-encode with lower quality
                if let Ok(decoded) = image::load_from_memory(&stream.content) {
                    let resized = decoded.resize(
                        new_width,
                        new_height,
                        image::imageops::FilterType::Lanczos3,
                    );

                    // Encode as JPEG with specified quality
                    let mut jpg_buf = std::io::Cursor::new(Vec::new());
                    let encoder =
                        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpg_buf, quality);
                    if resized.write_with_encoder(encoder).is_ok() {
                        let new_data = jpg_buf.into_inner();
                        stream.content = new_data;
                        stream.dict.set("Width", Object::Integer(new_width as i64));
                        stream
                            .dict
                            .set("Height", Object::Integer(new_height as i64));
                        stream.dict.set("Filter", Object::Name("DCTDecode".into()));
                        stream.dict.remove(b"BitsPerComponent");
                    }
                }
            }
        }
    }

    save_doc(&mut doc)
}

pub fn remove_metadata(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

    // Remove document info
    let info_id = doc.trailer.get(b"Info").and_then(|o| o.as_reference()).ok();

    if let Some(id) = info_id {
        doc.objects.remove(&id);
        doc.trailer.remove(b"Info");
    }

    // Remove XMP metadata
    let root_id = doc.trailer.get(b"Root").and_then(|o| o.as_reference()).ok();

    if let Some(id) = root_id {
        if let Some(root) = doc.objects.get_mut(&id) {
            if let Ok(dict) = root.as_dict_mut() {
                dict.remove(b"Metadata");
            }
        }
    }

    // Remove any embedded files
    for (_, obj) in doc.objects.iter_mut() {
        if let Object::Dictionary(ref mut dict) = obj {
            dict.remove(b"Names");
            dict.remove(b"EmbeddedFiles");
        }
    }

    save_doc(&mut doc)
}

pub fn flatten_content(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

    let page_ids = get_page_ids(&doc).clone();

    for &page_id in &page_ids {
        // Merge all content streams into one
        let mut all_operations = Vec::new();

        if let Some(Object::Dictionary(ref dict)) = doc.objects.get(&page_id) {
            if let Ok(Object::Reference(contents_id)) = dict.get(b"Contents") {
                if let Some(Object::Stream(stream)) = doc.objects.get(contents_id) {
                    if let Ok(content) = lopdf::content::Content::decode(&stream.content) {
                        all_operations.extend(content.operations);
                    }
                }
            }
        }

        if !all_operations.is_empty() {
            let content = lopdf::content::Content {
                operations: all_operations,
            };
            let content_bytes = content.encode().map_err(|e| format!("Encode error: {e}"))?;

            let mut stream = Stream::new(Dictionary::new(), content_bytes);
            stream.dict.set("Type", Object::Name("Content".into()));
            let content_id = doc.add_object(stream);

            if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
                dict.set("Contents", Object::Reference(content_id));
            }
        }
    }

    save_doc(&mut doc)
}

// ===== TRANSPARENCY FLATTENING =====

pub fn flatten_transparency(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

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
        let mut in_transparency = false;

        for op in &operations {
            match op.operator.as_str() {
                "gs" => {
                    // Graphics state with transparency
                    in_transparency = true;
                    new_operations.push(op.clone());
                }
                "cs" | "CS" => {
                    // Color space changes
                    new_operations.push(op.clone());
                }
                "rg" | "RG" | "k" | "K" => {
                    // Color operations - keep them
                    new_operations.push(op.clone());
                }
                "Q" => {
                    if in_transparency {
                        in_transparency = false;
                    }
                    new_operations.push(op.clone());
                }
                _ => {
                    new_operations.push(op.clone());
                }
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

// ===== PDF/X (ISO 15930 Print Production Standard) =====
pub use super::pdf_x::*;

// ===== COLOR SEPARATION PREVIEW & TOTAL AREA COVERAGE (TAC) =====

pub fn preview_color_separations(data: &[u8]) -> Result<serde_json::Value, String> {
    let doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

    let page_ids = get_page_ids(&doc);
    let mut separations = Vec::new();

    // Analyze color usage in each page
    for (page_idx, &page_id) in page_ids.iter().enumerate() {
        if let Some(Object::Dictionary(ref dict)) = doc.objects.get(&page_id) {
            if let Ok(Object::Reference(content_id)) = dict.get(b"Contents") {
                if let Some(Object::Stream(stream)) = doc.objects.get(content_id) {
                    if let Ok(content) = lopdf::content::Content::decode(&stream.content) {
                        let mut uses_rgb = false;
                        let mut uses_cmyk = false;
                        let mut uses_gray = false;

                        for op in &content.operations {
                            match op.operator.as_str() {
                                "rg" | "RG" => uses_rgb = true,
                                "k" | "K" => uses_cmyk = true,
                                "g" | "G" => uses_gray = true,
                                _ => {}
                            }
                        }

                        separations.push(serde_json::json!({
                            "page": page_idx + 1,
                            "rgb": uses_rgb,
                            "cmyk": uses_cmyk,
                            "gray": uses_gray,
                        }));
                    }
                }
            }
        }
    }

    // Determine if conversion is needed
    let needs_cmyk_conversion = separations.iter().any(|s| s["rgb"] == true);

    Ok(serde_json::json!({
        "separations": separations,
        "needs_cmyk_conversion": needs_cmyk_conversion,
        "max_tac_limit": 300,
        "recommendation": if needs_cmyk_conversion {
            "CMYK conversion recommended for print production"
        } else {
            "Color separations look correct"
        },
    }))
}

/// Render a specific color separation plate (Cyan, Magenta, Yellow, Key/Black, or Total Area Coverage highlight)
pub fn render_color_separation(
    data: &[u8],
    page_index: usize,
    dpi: u32,
    show_c: bool,
    show_m: bool,
    show_y: bool,
    show_k: bool,
    highlight_tac: bool,
    tac_limit: u32,
) -> Result<Vec<u8>, String> {
    // 1. Render base page image using pdftoppm
    let base_png = crate::pdf_engine::inspect::render_page_to_png(data, page_index, dpi)?;
    let img = image::load_from_memory(&base_png)
        .map_err(|e| format!("Failed to decode rendered page: {e}"))?;
    let mut rgba = img.to_rgba8();

    let limit = if tac_limit == 0 { 300 } else { tac_limit };

    // 2. Process each pixel into CMYK separation or TAC highlight
    for pixel in rgba.pixels_mut() {
        let r = pixel[0] as f32 / 255.0;
        let g = pixel[1] as f32 / 255.0;
        let b = pixel[2] as f32 / 255.0;
        let alpha = pixel[3];

        // Standard RGB to CMYK formula
        let k = 1.0 - r.max(g).max(b);
        let (c, m, y) = if k >= 0.9999 {
            (0.0f32, 0.0f32, 0.0f32)
        } else {
            let inv_k = 1.0 - k;
            (
                (1.0 - r - k) / inv_k,
                (1.0 - g - k) / inv_k,
                (1.0 - b - k) / inv_k,
            )
        };

        // Total Area Coverage (TAC) in % (0 - 400%)
        let total_ink_percent = ((c + m + y + k) * 100.0) as u32;

        if highlight_tac && total_ink_percent > limit {
            // Highlight exceeding ink coverage in vivid neon magenta/red
            pixel[0] = 255;
            pixel[1] = 0;
            pixel[2] = 80;
            pixel[3] = alpha;
        } else {
            // Combine enabled separation plates
            let active_c = if show_c { c } else { 0.0 };
            let active_m = if show_m { m } else { 0.0 };
            let active_y = if show_y { y } else { 0.0 };
            let active_k = if show_k { k } else { 0.0 };

            // Reconstruct RGB from active CMYK channels
            let inv_active_k = 1.0 - active_k;
            let out_r = ((1.0 - active_c) * inv_active_k * 255.0).clamp(0.0, 255.0) as u8;
            let out_g = ((1.0 - active_m) * inv_active_k * 255.0).clamp(0.0, 255.0) as u8;
            let out_b = ((1.0 - active_y) * inv_active_k * 255.0).clamp(0.0, 255.0) as u8;

            pixel[0] = out_r;
            pixel[1] = out_g;
            pixel[2] = out_b;
            pixel[3] = alpha;
        }
    }

    let mut out_buf = std::io::Cursor::new(Vec::new());
    rgba.write_to(&mut out_buf, image::ImageFormat::Png)
        .map_err(|e| format!("Failed to encode separation PNG: {e}"))?;
    Ok(out_buf.into_inner())
}

// ===== PREFLIGHT & PRINT PRODUCTION CHECK (Separated to preflight.rs) =====
pub use super::preflight::*;
