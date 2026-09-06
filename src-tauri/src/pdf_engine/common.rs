use lopdf::{Dictionary, Document, Object, Stream};

pub type OID = (u32, u16);

/// Append a new content stream to a page without overwriting existing contents.
/// Handles:
/// - Page with no /Contents (sets as Reference)
/// - Page with single Direct Stream (moves existing to indirect, creates Array [orig, new])
/// - Page with single Indirect Reference (creates Array [orig, new])
/// - Page with existing Array of streams/references (appends new reference preserving order)
pub(crate) fn append_page_content(
    doc: &mut Document,
    page_id: OID,
    new_content_id: OID,
) -> Result<(), String> {
    let page_obj = doc
        .objects
        .get_mut(&page_id)
        .ok_or_else(|| "Page object not found".to_string())?;
    let page_dict = page_obj
        .as_dict_mut()
        .map_err(|_| "Page is not a dictionary".to_string())?;

    let existing_contents = page_dict.get(b"Contents").ok().cloned();

    let updated_contents = match existing_contents {
        None => Object::Reference(new_content_id),
        Some(Object::Reference(orig_id)) => Object::Array(vec![
            Object::Reference(orig_id),
            Object::Reference(new_content_id),
        ]),
        Some(Object::Array(orig_arr)) => {
            let mut arr = orig_arr;
            arr.push(Object::Reference(new_content_id));
            Object::Array(arr)
        }
        Some(Object::Stream(existing_stream)) => {
            // Direct stream inside page dictionary: allocate as new object, then form array
            let orig_id = doc.add_object(Object::Stream(existing_stream));
            Object::Array(vec![
                Object::Reference(orig_id),
                Object::Reference(new_content_id),
            ])
        }
        Some(other) => {
            // Fallback for any other object representation
            Object::Array(vec![other, Object::Reference(new_content_id)])
        }
    };

    if let Some(page_obj) = doc.objects.get_mut(&page_id) {
        if let Ok(dict) = page_obj.as_dict_mut() {
            dict.set("Contents", updated_contents);
        }
    }

    Ok(())
}

/// Recursively resolve and return a copy of the page's resources dictionary,
/// walking up the /Parent tree if necessary to resolve inherited /Resources.
pub(crate) fn resolve_page_resources(doc: &Document, page_id: OID) -> Dictionary {
    let mut current_id = Some(page_id);
    while let Some(cid) = current_id {
        if let Some(Object::Dictionary(dict)) = doc.objects.get(&cid) {
            if let Ok(res_obj) = dict.get(b"Resources") {
                match res_obj {
                    Object::Dictionary(d) => return d.clone(),
                    Object::Reference(r_id) => {
                        if let Some(Object::Dictionary(d)) = doc.objects.get(r_id) {
                            return d.clone();
                        }
                    }
                    _ => {}
                }
            }
            current_id = dict.get(b"Parent").and_then(|p| p.as_reference()).ok();
        } else {
            break;
        }
    }
    Dictionary::new()
}

pub(crate) fn save_doc(doc: &mut Document) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    doc.save_to(&mut buf)
        .map_err(|e| format!("Failed to save: {e}"))?;
    Ok(buf)
}

pub(crate) fn get_page_ids(doc: &Document) -> Vec<OID> {
    super::page_tree::get_logical_page_ids(doc)
}

pub(crate) fn get_page_dimensions(doc: &Document, page_id: OID) -> (f32, f32) {
    if let Some(obj) = doc.objects.get(&page_id) {
        if let Ok(d) = obj.as_dict() {
            if let Ok(Object::Array(arr)) = d.get(b"MediaBox") {
                if arr.len() >= 4 {
                    let w = match &arr[2] {
                        Object::Real(r) => *r as f32,
                        Object::Integer(i) => *i as f32,
                        _ => 595.0,
                    };
                    let h = match &arr[3] {
                        Object::Real(r) => *r as f32,
                        Object::Integer(i) => *i as f32,
                        _ => 842.0,
                    };
                    return (w, h);
                }
            }
        }
    }
    (595.0, 842.0)
}

#[allow(dead_code)]
pub(crate) fn get_kids(doc: &Document) -> Option<Vec<Object>> {
    let root_id = doc.trailer.get(b"Root").ok()?.as_reference().ok()?;
    let root = doc.objects.get(&root_id)?;
    let pages_ref = root
        .as_dict()
        .ok()?
        .get(b"Pages")
        .ok()?
        .as_reference()
        .ok()?;
    let pages = doc.objects.get(&pages_ref)?;
    match pages.as_dict().ok()?.get(b"Kids") {
        Ok(Object::Array(kids)) => Some(kids.clone()),
        _ => None,
    }
}

#[allow(dead_code)]
pub(crate) fn set_page_info(doc: &mut Document, kids: Vec<Object>) {
    let root_id = match doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        Ok(id) => id,
        Err(_) => return,
    };
    let pages_id = match doc.objects.get(&root_id) {
        Some(root) => match root.as_dict() {
            Ok(d) => match d.get(b"Pages") {
                Ok(p) => match p.as_reference() {
                    Ok(id) => id,
                    Err(_) => return,
                },
                Err(_) => return,
            },
            Err(_) => return,
        },
        None => return,
    };
    if let Some(Object::Dictionary(ref mut pages_dict)) = doc.objects.get_mut(&pages_id) {
        pages_dict.set("Kids", Object::Array(kids.clone()));
        pages_dict.set("Count", Object::Integer(kids.len() as i64));
    }
}

#[allow(dead_code)]
pub(crate) fn get_page_count(doc: &Document) -> usize {
    let root_id = match doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        Ok(id) => id,
        Err(_) => return 0,
    };
    let pages_id = match doc.objects.get(&root_id) {
        Some(root) => match root.as_dict() {
            Ok(d) => match d.get(b"Pages") {
                Ok(p) => match p.as_reference() {
                    Ok(id) => id,
                    Err(_) => return 0,
                },
                Err(_) => return 0,
            },
            Err(_) => return 0,
        },
        None => return 0,
    };
    match doc.objects.get(&pages_id) {
        Some(pages) => match pages.as_dict() {
            Ok(d) => match d.get(b"Count") {
                Ok(Object::Integer(n)) => *n as usize,
                _ => 0,
            },
            Err(_) => 0,
        },
        None => 0,
    }
}

pub(crate) fn parse_hex_color(color: &str, default: (f32, f32, f32)) -> (f32, f32, f32) {
    let s = color.trim().trim_start_matches('#');
    if s.len() >= 6 {
        let r =
            u8::from_str_radix(&s[0..2], 16).unwrap_or((default.0 * 255.0) as u8) as f32 / 255.0;
        let g =
            u8::from_str_radix(&s[2..4], 16).unwrap_or((default.1 * 255.0) as u8) as f32 / 255.0;
        let b =
            u8::from_str_radix(&s[4..6], 16).unwrap_or((default.2 * 255.0) as u8) as f32 / 255.0;
        (r, g, b)
    } else {
        default
    }
}

#[allow(dead_code)]
pub(crate) fn ensure_page_root(doc: &mut Document) -> OID {
    let (_catalog_id, pages_id) = super::page_tree::ensure_catalog_and_pages_root(doc);
    pages_id
}

pub fn merge_pdfs(paths: &[String]) -> Result<Vec<u8>, String> {
    super::page_tree::merge_pdfs_robust(paths)
}

pub fn delete_page_in_doc(doc: &mut Document, page_index: usize) -> Result<(), String> {
    let mut page_ids = super::page_tree::get_logical_page_ids(doc);
    if page_index >= page_ids.len() {
        return Err(format!(
            "Page index {page_index} out of range (total pages: {})",
            page_ids.len()
        ));
    }
    if page_ids.len() <= 1 {
        return Err("Cannot delete the only remaining page in the document".to_string());
    }
    let removed_pid = page_ids.remove(page_index);
    doc.objects.remove(&removed_pid);
    super::page_tree::rebuild_flat_page_tree(doc, &page_ids)?;
    doc.prune_objects();
    Ok(())
}

pub fn delete_page(data: &[u8], page_index: usize) -> Result<Vec<u8>, String> {
    super::page_tree::delete_page_robust(data, page_index)
}

pub fn rotate_page_in_doc(
    doc: &mut Document,
    page_index: usize,
    degrees: i32,
) -> Result<(), String> {
    let page_ids = super::page_tree::get_logical_page_ids(doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }
    let page_id = page_ids[page_index];
    if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
        let current = match dict.get(b"Rotate") {
            Ok(Object::Integer(r)) => *r,
            _ => 0,
        };
        let new_rot = (current + degrees as i64).rem_euclid(360);
        dict.set("Rotate", Object::Integer(new_rot));
    }
    Ok(())
}

pub fn rotate_page(data: &[u8], page_index: usize, degrees: i32) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    rotate_page_in_doc(&mut doc, page_index, degrees)?;
    save_doc(&mut doc)
}

pub fn reorder_pages(data: &[u8], from_index: usize, to_index: usize) -> Result<Vec<u8>, String> {
    super::page_tree::reorder_pages_robust(data, from_index, to_index)
}

pub fn extract_pages(data: &[u8], indices: &[usize]) -> Result<Vec<u8>, String> {
    super::page_tree::extract_pages_robust(data, indices)
}

pub fn duplicate_page(data: &[u8], page_index: usize) -> Result<Vec<u8>, String> {
    super::page_tree::duplicate_page_robust(data, page_index)
}

pub fn add_text(
    data: &[u8],
    page_index: usize,
    text: &str,
    x: f64,
    y: f64,
    size: f64,
    color: &str,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let (r, g, b) = parse_hex_color(color, (0.0, 0.0, 0.0));
    let page_id = page_ids[page_index];

    let is_ascii = text.is_ascii();
    let (font_res_name, font_id, tj_string_obj) = if is_ascii {
        // Standard Type1 Helvetica for ASCII
        let mut font_dict = Dictionary::new();
        font_dict.set("Type", Object::Name("Font".into()));
        font_dict.set("Subtype", Object::Name("Type1".into()));
        font_dict.set("BaseFont", Object::Name("Helvetica".into()));
        let fid = doc.add_object(Object::Dictionary(font_dict));
        (
            format!("DocForgeTextHelv_{}_{}", fid.0, fid.1),
            fid,
            Object::String(text.as_bytes().to_vec(), lopdf::StringFormat::Literal),
        )
    } else {
        // True embedded Type0/CIDFontType2 with real TTF cmap, dynamic widths, and ToUnicode
        let (fid, encoded_cids) =
            super::font_unicode::embed_and_encode_unicode_text(&mut doc, text)?;
        (
            format!("DocForgeUniFont_{}_{}", fid.0, fid.1),
            fid,
            Object::String(encoded_cids, lopdf::StringFormat::Hexadecimal),
        )
    };

    let mut resources_dict = resolve_page_resources(&doc, page_id);
    let mut fonts_dict = match resources_dict.get(b"Font") {
        Ok(Object::Dictionary(fd)) => fd.clone(),
        Ok(Object::Reference(f_ref)) => doc
            .objects
            .get(f_ref)
            .and_then(|o| o.as_dict().ok())
            .cloned()
            .unwrap_or_default(),
        _ => Dictionary::new(),
    };
    fonts_dict.set(font_res_name.clone(), Object::Reference(font_id));
    resources_dict.set("Font", Object::Dictionary(fonts_dict));
    if let Some(Object::Dictionary(ref mut page_dict)) = doc.objects.get_mut(&page_id) {
        page_dict.set("Resources", Object::Dictionary(resources_dict));
    }

    // Wrap operations with q / Q to protect graphics state
    let operations = vec![
        lopdf::content::Operation::new("q", vec![]),
        lopdf::content::Operation::new("BT", vec![]),
        lopdf::content::Operation::new(
            "Tf",
            vec![
                Object::Name(font_res_name.into()),
                Object::Real(size as f32),
            ],
        ),
        lopdf::content::Operation::new(
            "rg",
            vec![Object::Real(r), Object::Real(g), Object::Real(b)],
        ),
        lopdf::content::Operation::new("Td", vec![Object::Real(x as f32), Object::Real(y as f32)]),
        lopdf::content::Operation::new("Tj", vec![tj_string_obj]),
        lopdf::content::Operation::new("ET", vec![]),
        lopdf::content::Operation::new("Q", vec![]),
    ];

    let content = lopdf::content::Content { operations };
    let content_bytes = content
        .encode()
        .map_err(|e| format!("Failed to encode content: {e}"))?;

    let mut stream = Stream::new(Dictionary::new(), content_bytes);
    stream.dict.set("Type", Object::Name("Content".into()));
    let content_id = doc.add_object(stream);

    append_page_content(&mut doc, page_id, content_id)?;

    save_doc(&mut doc)
}

pub fn protect_pdf(_data: &[u8], _password: &str) -> Result<Vec<u8>, String> {
    // Honest: Refuse to generate corrupted pseudo-encrypted PDF.
    // Full Standard Security Handler with AES-128/256 and key derivation schedule
    // is required to safely encrypt streams and strings without corrupting the document.
    Err("PDF暗号化（AES-128/256 Standard Security Handler）によるストリーム暗号化は現在実装準備中です。破損した暗号化PDFの出力を防止するため処理を安全に中断しました。".into())
}

pub fn create_blank_pdf(width: f64, height: f64, page_count: usize) -> Result<Vec<u8>, String> {
    let mut doc = Document::with_version("1.7");

    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", Object::Name("Pages".into()));
    pages_dict.set("Count", Object::Integer(page_count as i64));
    let pages_id = doc.add_object(Object::Dictionary(pages_dict));

    let mut kids = Vec::new();
    for _ in 0..page_count {
        let mut page_dict = Dictionary::new();
        page_dict.set("Type", Object::Name("Page".into()));
        page_dict.set("Parent", Object::Reference(pages_id));
        page_dict.set(
            "MediaBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(width as f32),
                Object::Real(height as f32),
            ]),
        );
        let page_id = doc.add_object(Object::Dictionary(page_dict));
        kids.push(Object::Reference(page_id));
    }

    if let Some(Object::Dictionary(ref mut pages)) = doc.objects.get_mut(&pages_id) {
        pages.set("Kids", Object::Array(kids));
    }

    let mut catalog = Dictionary::new();
    catalog.set("Type", Object::Name("Catalog".into()));
    catalog.set("Pages", Object::Reference(pages_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog));

    doc.trailer.set("Root", Object::Reference(catalog_id));

    save_doc(&mut doc)
}

pub fn add_image_to_page(
    data: &[u8],
    page_index: usize,
    image_data: &[u8],
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let img =
        image::load_from_memory(image_data).map_err(|e| format!("Failed to decode image: {e}"))?;
    let rgb = img.to_rgb8();
    let img_width = rgb.width();
    let img_height = rgb.height();

    let mut img_dict = Dictionary::new();
    img_dict.set("Type", Object::Name("XObject".into()));
    img_dict.set("Subtype", Object::Name("Image".into()));
    img_dict.set("Width", Object::Integer(img_width as i64));
    img_dict.set("Height", Object::Integer(img_height as i64));
    img_dict.set("ColorSpace", Object::Name("DeviceRGB".into()));
    img_dict.set("BitsPerComponent", Object::Integer(8));
    img_dict.set("Filter", Object::Name("DCTDecode".into()));

    let mut jpg_buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut jpg_buf, image::ImageFormat::Jpeg)
        .map_err(|e| format!("Failed to encode JPEG: {e}"))?;
    let jpg_bytes = jpg_buf.into_inner();

    let stream = Stream::new(img_dict, jpg_bytes);
    let img_id = doc.add_object(stream);

    let page_id = page_ids[page_index];

    // Unique XObject resource name based on img_id
    let img_res_name = format!("DocForgeImg_{}_{}", img_id.0, img_id.1);

    // Update page resources safely
    let mut resources = resolve_page_resources(&doc, page_id);
    let mut xobjects = match resources.get(b"XObject") {
        Ok(Object::Dictionary(x)) => x.clone(),
        Ok(Object::Reference(x_ref)) => doc
            .objects
            .get(x_ref)
            .and_then(|o| o.as_dict().ok())
            .cloned()
            .unwrap_or_default(),
        _ => Dictionary::new(),
    };
    xobjects.set(img_res_name.clone(), Object::Reference(img_id));
    resources.set("XObject", Object::Dictionary(xobjects));

    if let Some(Object::Dictionary(ref mut page_dict)) = doc.objects.get_mut(&page_id) {
        page_dict.set("Resources", Object::Dictionary(resources));
    }

    let operations = vec![
        lopdf::content::Operation::new("q", vec![]),
        lopdf::content::Operation::new(
            "cm",
            vec![
                Object::Real(width as f32),
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(height as f32),
                Object::Real(x as f32),
                Object::Real(y as f32),
            ],
        ),
        lopdf::content::Operation::new("Do", vec![Object::Name(img_res_name.into())]),
        lopdf::content::Operation::new("Q", vec![]),
    ];

    let content = lopdf::content::Content { operations };
    let content_bytes = content
        .encode()
        .map_err(|e| format!("Failed to encode: {e}"))?;

    let mut content_stream = Stream::new(Dictionary::new(), content_bytes);
    content_stream
        .dict
        .set("Type", Object::Name("Content".into()));
    let content_id = doc.add_object(content_stream);

    append_page_content(&mut doc, page_id, content_id)?;

    save_doc(&mut doc)
}

pub fn crop_page(
    data: &[u8],
    page_index: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }
    let page_id = page_ids[page_index];
    if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
        dict.set(
            "MediaBox",
            Object::Array(vec![
                Object::Real(x as f32),
                Object::Real(y as f32),
                Object::Real((x + width) as f32),
                Object::Real((y + height) as f32),
            ]),
        );
    }
    save_doc(&mut doc)
}
