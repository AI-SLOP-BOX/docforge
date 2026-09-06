use super::common::*;
use super::*;
use lopdf::{Dictionary, Document, Object, Stream};

// ===== BATCH PROCESSING =====

pub fn batch_merge_pdfs(paths: &[String], output_path: &str) -> Result<(), String> {
    let merged = merge_pdfs(paths)?;
    std::fs::write(output_path, merged).map_err(|e| format!("Failed to write output: {e}"))
}

pub fn batch_add_watermark(
    paths: &[String],
    text: &str,
    opacity: f32,
    rotation: f32,
    font_size: f32,
    color: &str,
) -> Result<Vec<Vec<u8>>, String> {
    let mut results = Vec::new();
    for path in paths {
        let data = std::fs::read(path).map_err(|e| format!("Failed to read {}: {e}", path))?;
        let watermarked =
            add_watermark(&data, text, opacity, rotation, font_size, color, true, &[])?;
        results.push(watermarked);
    }
    Ok(results)
}

pub fn batch_protect(paths: &[String], password: &str) -> Result<Vec<Vec<u8>>, String> {
    let mut results = Vec::new();
    for path in paths {
        let data = std::fs::read(path).map_err(|e| format!("Failed to read {}: {e}", path))?;
        let protected = protect_pdf(&data, password)?;
        results.push(protected);
    }
    Ok(results)
}

pub fn batch_optimize(paths: &[String]) -> Result<Vec<Vec<u8>>, String> {
    let mut results = Vec::new();
    for path in paths {
        let data = std::fs::read(path).map_err(|e| format!("Failed to read {}: {e}", path))?;
        let optimized = optimize_pdf(&data)?;
        results.push(optimized);
    }
    Ok(results)
}

// ===== PDF/A COMPLIANCE =====

pub fn convert_to_pdfa(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

    let mut identification = Dictionary::new();
    identification.set("Type", Object::Name("OutputIntents".into()));
    identification.set("S", Object::Name("GTS_PDFA1".into()));
    identification.set(
        "OutputConditionIdentifier",
        Object::String(b"sRGB IEC61966-2.1".to_vec(), lopdf::StringFormat::Literal),
    );
    identification.set(
        "RegistryName",
        Object::String(
            b"http://www.color.org".to_vec(),
            lopdf::StringFormat::Literal,
        ),
    );

    let identification_id = doc.add_object(Object::Dictionary(identification));

    let root_id = doc
        .trailer
        .get(b"Root")
        .and_then(|o| o.as_reference())
        .ok()
        .ok_or("No root")?;

    if let Some(Object::Dictionary(ref mut root)) = doc.objects.get_mut(&root_id) {
        root.set(
            "OutputIntents",
            Object::Array(vec![Object::Reference(identification_id)]),
        );
        root.set("MarkInfo", Object::Dictionary(Dictionary::new()));
        root.set("Metadata", Object::String(b"<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n<rdf:Description rdf:about=\"\" xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\" pdf:Producer=\"DocForge\"/>\n</rdf:RDF>\n</x:xmpmeta>\n<?xpacket end=\"w\"?>".to_vec(), lopdf::StringFormat::Literal));
    }

    doc.version = "1.4".to_string();
    save_doc(&mut doc)
}

// ===== HEADERS & FOOTERS =====

pub fn add_header_footer(
    data: &[u8],
    header_text: &str,
    footer_text: &str,
    font_size: f32,
    margin: f32,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc).clone();

    for (i, &page_id) in page_ids.iter().enumerate() {
        let (_pw, ph) = get_page_dimensions(&doc, page_id);

        let mut operations = vec![
            lopdf::content::Operation::new("q", vec![]),
            lopdf::content::Operation::new("BT", vec![]),
            lopdf::content::Operation::new(
                "Tf",
                vec![Object::Name("Helvetica".into()), Object::Real(font_size)],
            ),
            lopdf::content::Operation::new(
                "rg",
                vec![Object::Real(0.3), Object::Real(0.3), Object::Real(0.3)],
            ),
        ];

        let header = header_text
            .replace("{page}", &(i + 1).to_string())
            .replace("{total}", &page_ids.len().to_string());
        operations.push(lopdf::content::Operation::new(
            "Td",
            vec![Object::Real(margin), Object::Real(ph - margin)],
        ));
        operations.push(lopdf::content::Operation::new(
            "Tj",
            vec![Object::String(
                header.as_bytes().to_vec(),
                lopdf::StringFormat::Literal,
            )],
        ));

        operations.push(lopdf::content::Operation::new("ET", vec![]));
        operations.push(lopdf::content::Operation::new("BT", vec![]));
        operations.push(lopdf::content::Operation::new(
            "Tf",
            vec![Object::Name("Helvetica".into()), Object::Real(font_size)],
        ));

        let footer = footer_text
            .replace("{page}", &(i + 1).to_string())
            .replace("{total}", &page_ids.len().to_string());
        operations.push(lopdf::content::Operation::new(
            "Td",
            vec![Object::Real(margin), Object::Real(margin)],
        ));
        operations.push(lopdf::content::Operation::new(
            "Tj",
            vec![Object::String(
                footer.as_bytes().to_vec(),
                lopdf::StringFormat::Literal,
            )],
        ));

        operations.push(lopdf::content::Operation::new("ET", vec![]));
        operations.push(lopdf::content::Operation::new("Q", vec![]));

        let content = lopdf::content::Content { operations };
        let content_bytes = content.encode().map_err(|e| format!("Encode error: {e}"))?;

        let mut stream = Stream::new(Dictionary::new(), content_bytes);
        stream.dict.set("Type", Object::Name("Content".into()));
        let content_id = doc.add_object(stream);

        append_page_content(&mut doc, page_id, content_id)?;
    }

    save_doc(&mut doc)
}

// ===== BOOKMARKS =====

pub fn add_bookmark(data: &[u8], title: &str, page_index: usize) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let page_id = page_ids[page_index];

    // Ensure Catalog exists
    let root_id = if let Ok(id) = doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        id
    } else {
        let root_dict = Dictionary::new();
        let id = doc.add_object(Object::Dictionary(root_dict));
        doc.trailer.set("Root", Object::Reference(id));
        id
    };

    // Check if Catalog already has Outlines
    let outline_id = if let Some(Object::Dictionary(ref root_dict)) = doc.objects.get(&root_id) {
        root_dict
            .get(b"Outlines")
            .ok()
            .and_then(|o| o.as_reference().ok())
    } else {
        None
    };

    let outline_id = match outline_id {
        Some(oid) => oid,
        None => {
            let mut outlines = Dictionary::new();
            outlines.set("Type", Object::Name("Outlines".into()));
            outlines.set("Count", Object::Integer(0));
            let oid = doc.add_object(Object::Dictionary(outlines));
            if let Some(Object::Dictionary(ref mut root_dict)) = doc.objects.get_mut(&root_id) {
                root_dict.set("Outlines", Object::Reference(oid));
            }
            oid
        }
    };

    // Create bookmark item dictionary
    let mut item_dict = Dictionary::new();
    item_dict.set(
        "Title",
        Object::String(title.as_bytes().to_vec(), lopdf::StringFormat::Literal),
    );
    item_dict.set("Parent", Object::Reference(outline_id));
    item_dict.set(
        "Dest",
        Object::Array(vec![
            Object::Reference(page_id),
            Object::Name("FitH".into()),
            Object::Real(0.0),
        ]),
    );

    // Read existing Outlines dictionary to find First, Last, and Count
    let (first_ref, last_ref, current_count) =
        if let Some(Object::Dictionary(ref out_dict)) = doc.objects.get(&outline_id) {
            let first = out_dict
                .get(b"First")
                .ok()
                .and_then(|o| o.as_reference().ok());
            let last = out_dict
                .get(b"Last")
                .ok()
                .and_then(|o| o.as_reference().ok());
            let count = out_dict
                .get(b"Count")
                .ok()
                .and_then(|o| o.as_i64().ok())
                .unwrap_or(0);
            (first, last, count)
        } else {
            (None, None, 0)
        };

    // If there is an existing last item, link its Next to this new item, and new item's Prev to last
    if let Some(prev_last_id) = last_ref {
        item_dict.set("Prev", Object::Reference(prev_last_id));
    }

    let new_item_id = doc.add_object(Object::Dictionary(item_dict));

    if let Some(prev_last_id) = last_ref {
        if let Some(Object::Dictionary(ref mut prev_dict)) = doc.objects.get_mut(&prev_last_id) {
            prev_dict.set("Next", Object::Reference(new_item_id));
        }
    }

    // Update Outlines dictionary First, Last, and Count
    if let Some(Object::Dictionary(ref mut out_dict)) = doc.objects.get_mut(&outline_id) {
        if first_ref.is_none() {
            out_dict.set("First", Object::Reference(new_item_id));
        }
        out_dict.set("Last", Object::Reference(new_item_id));
        out_dict.set("Count", Object::Integer(current_count + 1));
    }

    save_doc(&mut doc)
}

// ===== BATES NUMBERING =====

pub fn add_bates_number(
    data: &[u8],
    prefix: &str,
    start_number: usize,
    font_size: f32,
    margin: f32,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc).clone();

    for (i, &page_id) in page_ids.iter().enumerate() {
        let (pw, _ph) = get_page_dimensions(&doc, page_id);

        let bates_text = format!("{}{:06}", prefix, start_number + i);

        let operations = vec![
            lopdf::content::Operation::new("q", vec![]),
            lopdf::content::Operation::new("BT", vec![]),
            lopdf::content::Operation::new(
                "Tf",
                vec![Object::Name("Helvetica".into()), Object::Real(font_size)],
            ),
            lopdf::content::Operation::new(
                "rg",
                vec![Object::Real(0.0), Object::Real(0.0), Object::Real(0.0)],
            ),
            lopdf::content::Operation::new(
                "Td",
                vec![Object::Real(pw - margin - 60.0), Object::Real(margin)],
            ),
            lopdf::content::Operation::new(
                "Tj",
                vec![Object::String(
                    bates_text.as_bytes().to_vec(),
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

        append_page_content(&mut doc, page_id, content_id)?;
    }

    save_doc(&mut doc)
}
