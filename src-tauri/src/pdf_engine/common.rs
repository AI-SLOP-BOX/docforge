use lopdf::{Document, Object, Stream, Dictionary};

pub type OID = (u32, u16);

pub(crate) fn save_doc(doc: &mut Document) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    doc.save_to(&mut buf).map_err(|e| format!("Failed to save: {e}"))?;
    Ok(buf)
}

pub(crate) fn get_page_ids(doc: &Document) -> Vec<OID> {
    let mut pages: Vec<(u32, OID)> = doc.get_pages().into_iter().collect();
    pages.sort_by_key(|&(page_num, _)| page_num);
    pages.into_iter().map(|(_, oid)| oid).collect()
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

pub(crate) fn get_kids(doc: &Document) -> Option<Vec<Object>> {
    let root_id = doc.trailer.get(b"Root").ok()?.as_reference().ok()?;
    let root = doc.objects.get(&root_id)?;
    let pages_ref = root.as_dict().ok()?.get(b"Pages").ok()?.as_reference().ok()?;
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
        let r = u8::from_str_radix(&s[0..2], 16).unwrap_or((default.0 * 255.0) as u8) as f32 / 255.0;
        let g = u8::from_str_radix(&s[2..4], 16).unwrap_or((default.1 * 255.0) as u8) as f32 / 255.0;
        let b = u8::from_str_radix(&s[4..6], 16).unwrap_or((default.2 * 255.0) as u8) as f32 / 255.0;
        (r, g, b)
    } else {
        default
    }
}

pub(crate) fn ensure_page_root(doc: &mut Document) -> OID {
    let root_id = match doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        Ok(id) => id,
        Err(_) => {
            let pages_id = doc.add_object(Dictionary::new());
            doc.trailer.set("Root", Object::Reference(pages_id));
            return pages_id;
        }
    };
    let existing = match doc.objects.get(&root_id) {
        Some(root) => match root.as_dict() {
            Ok(d) => d.get(b"Pages").ok().and_then(|p| p.as_reference().ok()),
            Err(_) => None,
        },
        None => None,
    };
    if let Some(pages_id) = existing {
        return pages_id;
    }
    let pages_id = doc.add_object(Dictionary::new());
    if let Some(root) = doc.objects.get_mut(&root_id) {
        if let Ok(d) = root.as_dict_mut() {
            d.set("Pages", Object::Reference(pages_id));
        }
    }
    pages_id
}

pub fn merge_pdfs(paths: &[String]) -> Result<Vec<u8>, String> {
    if paths.is_empty() {
        return Err("No files to merge".into());
    }

    let mut base = Document::load(&paths[0])
        .map_err(|e| format!("Failed to load {}: {e}", paths[0]))?;

    let mut kids = get_kids(&mut base).unwrap_or_default();

    for path in &paths[1..] {
        let other = Document::load(path)
            .map_err(|e| format!("Failed to load {path}: {e}"))?;

        let other_page_ids = get_page_ids(&other);

        // Copy all objects from other doc, remapping IDs
        let mut id_map: std::collections::HashMap<OID, OID> = std::collections::HashMap::new();

        for (&old_id, obj) in &other.objects {
            let new_id = base.add_object(obj.clone());
            id_map.insert(old_id, new_id);
        }

        // Re-map all references
        for (_old_id, new_id) in &id_map {
            if let Some(obj) = base.objects.get_mut(new_id) {
                remap_references(obj, &id_map);
            }
        }

        // Add page references to kids
        for page_oid in &other_page_ids {
            if let Some(&new_id) = id_map.get(page_oid) {
                kids.push(Object::Reference(new_id));
            }
        }
    }

    // Update the page root
    let pages_id = ensure_page_root(&mut base);
    if let Some(Object::Dictionary(ref mut pages_dict)) = base.objects.get_mut(&pages_id) {
        pages_dict.set("Kids", Object::Array(kids.clone()));
        pages_dict.set("Count", Object::Integer(kids.len() as i64));
    }

    save_doc(&mut base)
}

fn remap_references(obj: &mut Object, id_map: &std::collections::HashMap<OID, OID>) {
    match obj {
        Object::Reference(ref mut r) => {
            if let Some(&new_id) = id_map.get(r) {
                *r = new_id;
            }
        }
        Object::Array(arr) => {
            for item in arr.iter_mut() {
                remap_references(item, id_map);
            }
        }
        Object::Dictionary(dict) => {
            for (_, v) in dict.iter_mut() {
                remap_references(v, id_map);
            }
        }
        Object::Stream(stream) => {
            for (_, v) in stream.dict.iter_mut() {
                remap_references(v, id_map);
            }
        }
        _ => {}
    }
}

pub fn delete_page(data: &[u8], page_index: usize) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }
    let page_id = page_ids[page_index];
    let mut kids = get_kids(&mut doc).unwrap_or_default();
    if page_index < kids.len() {
        kids.remove(page_index);
    }
    doc.objects.remove(&page_id);
    let pages_id = ensure_page_root(&mut doc);
    if let Some(Object::Dictionary(ref mut pages_dict)) = doc.objects.get_mut(&pages_id) {
        pages_dict.set("Kids", Object::Array(kids.clone()));
        pages_dict.set("Count", Object::Integer(kids.len() as i64));
    }
    save_doc(&mut doc)
}

pub fn rotate_page(data: &[u8], page_index: usize, degrees: i32) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }
    let page_id = page_ids[page_index];
    if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
        let current = match dict.get(b"Rotate") {
            Ok(Object::Integer(r)) => *r,
            _ => 0,
        };
        dict.set("Rotate", Object::Integer(current + degrees as i64));
    }
    save_doc(&mut doc)
}

pub fn reorder_pages(data: &[u8], from_index: usize, to_index: usize) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;
    let mut kids = get_kids(&mut doc).unwrap_or_default();
    if from_index >= kids.len() || to_index >= kids.len() {
        return Err("Page index out of range".into());
    }
    let item = kids.remove(from_index);
    kids.insert(to_index, item);
    let pages_id = ensure_page_root(&mut doc);
    if let Some(Object::Dictionary(ref mut pages_dict)) = doc.objects.get_mut(&pages_id) {
        pages_dict.set("Kids", Object::Array(kids.clone()));
        pages_dict.set("Count", Object::Integer(kids.len() as i64));
    }
    save_doc(&mut doc)
}

pub fn extract_pages(data: &[u8], indices: &[usize]) -> Result<Vec<u8>, String> {
    let doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    let mut new_doc = Document::with_version("1.7");
    let mut new_kids = Vec::new();

    for &idx in indices {
        if idx < page_ids.len() {
            let page_id = page_ids[idx];
            if let Some(obj) = doc.objects.get(&page_id) {
                let new_page_id = new_doc.add_object(obj.clone());
                new_kids.push(Object::Reference(new_page_id));
            }
        }
    }

    let pages_id = new_doc.add_object(Dictionary::new());
    if let Some(Object::Dictionary(ref mut pages_dict)) = new_doc.objects.get_mut(&pages_id) {
        pages_dict.set("Type", Object::Name("Pages".into()));
        pages_dict.set("Kids", Object::Array(new_kids.clone()));
        pages_dict.set("Count", Object::Integer(new_kids.len() as i64));
    }

    new_doc.trailer.set("Root", Object::Reference(pages_id));
    save_doc(&mut new_doc)
}

pub fn duplicate_page(data: &[u8], page_index: usize) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }
    let page_id = page_ids[page_index];
    let mut kids = get_kids(&mut doc).unwrap_or_default();

    // Clone the page object
    if let Some(obj) = doc.objects.get(&page_id).cloned() {
        let new_page_id = doc.add_object(obj);
        // Insert the duplicate right after the original
        let insert_at = std::cmp::min(page_index + 1, kids.len());
        kids.insert(insert_at, Object::Reference(new_page_id));
    }

    let pages_id = ensure_page_root(&mut doc);
    if let Some(Object::Dictionary(ref mut pages_dict)) = doc.objects.get_mut(&pages_id) {
        pages_dict.set("Kids", Object::Array(kids.clone()));
        pages_dict.set("Count", Object::Integer(kids.len() as i64));
    }

    save_doc(&mut doc)
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
    let mut doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let (r, g, b) = parse_hex_color(color, (0.0, 0.0, 0.0));

    let operations = vec![
        lopdf::content::Operation::new("BT", vec![]),
        lopdf::content::Operation::new("Tf", vec![Object::Name("Helvetica".into()), Object::Real(size as f32)]),
        lopdf::content::Operation::new("rg", vec![Object::Real(r), Object::Real(g), Object::Real(b)]),
        lopdf::content::Operation::new("Td", vec![Object::Real(x as f32), Object::Real(y as f32)]),
        lopdf::content::Operation::new("Tj", vec![Object::String(text.as_bytes().to_vec(), lopdf::StringFormat::Literal)]),
        lopdf::content::Operation::new("ET", vec![]),
    ];

    let content = lopdf::content::Content { operations };
    let content_bytes = content.encode()
        .map_err(|e| format!("Failed to encode content: {e}"))?;

    let page_id = page_ids[page_index];
    let mut stream = Stream::new(Dictionary::new(), content_bytes);
    stream.dict.set("Type", Object::Name("Content".into()));
    let content_id = doc.add_object(stream);

    if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
        dict.set("Contents", Object::Reference(content_id));
    }

    save_doc(&mut doc)
}

pub fn protect_pdf(data: &[u8], password: &str) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;

    let mut crypt_dict = Dictionary::new();
    crypt_dict.set("Filter", Object::Name("Standard".into()));
    crypt_dict.set("V", Object::Integer(2));
    crypt_dict.set("R", Object::Integer(3));
    crypt_dict.set("Length", Object::Integer(128));
    crypt_dict.set("O", Object::String(password.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    crypt_dict.set("U", Object::String(password.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    crypt_dict.set("P", Object::Integer(-4));

    let crypt_id = doc.add_object(Object::Dictionary(crypt_dict));
    doc.trailer.set("Encrypt", Object::Reference(crypt_id));

    save_doc(&mut doc)
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
        page_dict.set("MediaBox", Object::Array(vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(width as f32),
            Object::Real(height as f32),
        ]));
        let page_id = doc.add_object(Object::Dictionary(page_dict));
        kids.push(Object::Reference(page_id));
    }

    if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&pages_id) {
        dict.set("Kids", Object::Array(kids));
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
    let mut doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let img = image::load_from_memory(image_data)
        .map_err(|e| format!("Failed to decode image: {e}"))?;
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
    let operations = vec![
        lopdf::content::Operation::new("q", vec![]),
        lopdf::content::Operation::new("cm", vec![
            Object::Real(width as f32),
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(height as f32),
            Object::Real(x as f32),
            Object::Real(y as f32),
        ]),
        lopdf::content::Operation::new("Do", vec![Object::Name("Img".into())]),
        lopdf::content::Operation::new("Q", vec![]),
    ];

    let content = lopdf::content::Content { operations };
    let content_bytes = content.encode().map_err(|e| format!("Failed to encode: {e}"))?;

    let mut content_stream = Stream::new(Dictionary::new(), content_bytes);
    content_stream.dict.set("Type", Object::Name("Content".into()));
    let content_id = doc.add_object(content_stream);

    if let Some(Object::Dictionary(ref mut page_dict)) = doc.objects.get_mut(&page_id) {
        page_dict.set("Contents", Object::Reference(content_id));

        let mut resources = match page_dict.get(b"Resources") {
            Ok(Object::Dictionary(r)) => r.clone(),
            _ => Dictionary::new(),
        };
        let mut xobjects = match resources.get(b"XObject") {
            Ok(Object::Dictionary(x)) => x.clone(),
            _ => Dictionary::new(),
        };
        xobjects.set("Img", Object::Reference(img_id));
        resources.set("XObject", Object::Dictionary(xobjects));
        page_dict.set("Resources", Object::Dictionary(resources));
    }

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
    let mut doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }
    let page_id = page_ids[page_index];
    if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
        dict.set("MediaBox", Object::Array(vec![
            Object::Real(x as f32),
            Object::Real(y as f32),
            Object::Real((x + width) as f32),
            Object::Real((y + height) as f32),
        ]));
    }
    save_doc(&mut doc)
}

