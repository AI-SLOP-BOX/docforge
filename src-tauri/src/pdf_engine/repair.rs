use super::common::*;
use lopdf::{Dictionary, Document, Object};

/// Repair corrupted, truncated, or broken-XRef PDF documents.
/// Uses a fallback heuristic salvage approach:
/// Scans the raw binary byte stream for `N M obj ... endobj` patterns,
/// extracts surviving objects, reconstructs a healthy Catalog/Pages tree,
/// and writes a pristine cross-reference table and trailer.
pub fn repair_corrupt_pdf(data: &[u8]) -> Result<Vec<u8>, String> {
    // 1. Try standard load first. If successful and valid, re-save with clean cross-references.
    if let Ok(mut doc) = Document::load_mem(data) {
        if !doc.get_pages().is_empty() {
            doc.prune_objects();
            return save_doc(&mut doc);
        }
    }

    // 2. Salvage parse: Heuristically scan for objects in raw byte buffer
    let mut salvaged_doc = Document::with_version("1.7");
    let mut found_page_ids: Vec<OID> = Vec::new();
    let mut found_catalog_pages_ref: Option<OID> = None;

    let len = data.len();
    let mut cursor = 0;

    while cursor < len {
        if let Some(pos) = find_subsequence(&data[cursor..], b"obj") {
            let obj_keyword_pos = cursor + pos;
            let prefix = &data[..obj_keyword_pos];

            if let Some((obj_num, gen_num)) = parse_obj_header(prefix) {
                let content_start = obj_keyword_pos + 3;
                if let Some(end_pos) = find_subsequence(&data[content_start..], b"endobj") {
                    let obj_body = &data[content_start..content_start + end_pos];
                    if let Ok(parsed_obj) = parse_salvaged_object(obj_body) {
                        if let Object::Dictionary(ref dict) = parsed_obj {
                            if let Ok(Object::Name(ref type_name)) = dict.get(b"Type") {
                                if type_name == b"Page" {
                                    found_page_ids.push((obj_num, gen_num));
                                } else if type_name == b"Catalog" {
                                    if let Ok(Object::Reference(pages_ref)) = dict.get(b"Pages") {
                                        found_catalog_pages_ref = Some(*pages_ref);
                                    }
                                }
                            }
                        }
                        salvaged_doc.objects.insert((obj_num, gen_num), parsed_obj);
                    }
                    cursor = content_start + end_pos + 6;
                    continue;
                }
            }
            cursor = obj_keyword_pos + 3;
        } else {
            break;
        }
    }

    if salvaged_doc.objects.is_empty() {
        return Err("No recoverable PDF objects could be salvaged from the file".to_string());
    }

    // 3. Reconstruct Pages tree if broken or missing
    let pages_id = if let Some(existing_pages) = found_catalog_pages_ref {
        existing_pages
    } else {
        let mut pages_dict = Dictionary::new();
        pages_dict.set("Type", Object::Name("Pages".into()));
        pages_dict.set("Count", Object::Integer(found_page_ids.len() as i64));
        let mut kids = Vec::new();
        for &pid in &found_page_ids {
            kids.push(Object::Reference(pid));
        }
        pages_dict.set("Kids", Object::Array(kids));
        salvaged_doc.add_object(Object::Dictionary(pages_dict))
    };

    // Update parent reference for salvaged pages
    for &pid in &found_page_ids {
        if let Some(Object::Dictionary(ref mut pdict)) = salvaged_doc.objects.get_mut(&pid) {
            pdict.set("Parent", Object::Reference(pages_id));
        }
    }

    // 4. Reconstruct Catalog (Root)
    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", Object::Name("Catalog".into()));
    catalog_dict.set("Pages", Object::Reference(pages_id));
    let catalog_id = salvaged_doc.add_object(Object::Dictionary(catalog_dict));
    salvaged_doc
        .trailer
        .set("Root", Object::Reference(catalog_id));

    // 5. Clean, prune and produce valid PDF byte stream
    salvaged_doc.prune_objects();
    save_doc(&mut salvaged_doc)
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn parse_obj_header(slice: &[u8]) -> Option<(u32, u16)> {
    // Only inspect the tail (last ~32 bytes) of the prefix before " obj"
    let tail_len = slice.len().min(32);
    let tail = &slice[slice.len() - tail_len..];
    let s = String::from_utf8_lossy(tail);
    let words: Vec<&str> = s.split_whitespace().collect();
    if words.len() >= 2 {
        let obj_num = words[words.len() - 2].parse::<u32>().ok()?;
        let gen_num = words[words.len() - 1].parse::<u16>().ok()?;
        return Some((obj_num, gen_num));
    }
    None
}

fn parse_salvaged_object(body: &[u8]) -> Result<Object, ()> {
    let trimmed = body.trim_ascii();
    let mut skeleton = Vec::new();
    skeleton.extend_from_slice(b"%PDF-1.4\n1 0 obj\n");
    let obj_offset = 9; // offset of "1 0 obj\n"
    skeleton.extend_from_slice(trimmed);
    skeleton.extend_from_slice(b"\nendobj\n");
    let xref_pos = skeleton.len();
    skeleton.extend_from_slice(
        format!(
            "xref\n0 2\n0000000000 65535 f \n{:010} 00000 n \ntrailer\n<< /Size 2 /Root 1 0 R >>\nstartxref\n{}\n%%EOF",
            obj_offset, xref_pos
        ).as_bytes()
    );

    if let Ok(temp_doc) = Document::load_mem(&skeleton) {
        if let Some((_, obj)) = temp_doc.objects.into_iter().next() {
            return Ok(obj);
        }
    }
    Err(())
}
