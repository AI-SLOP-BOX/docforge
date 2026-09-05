use lopdf::{Document, Object, Dictionary};
use super::common::*;

// ===== ACCESSIBILITY CHECK =====

pub fn check_accessibility(data: &[u8]) -> Result<serde_json::Value, String> {
    let doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;

    let page_ids = get_page_ids(&doc);
    let mut issues = Vec::new();

    // Check for tagged PDF
    let has_tags = if let Ok(root_id) = doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        if let Some(Object::Dictionary(ref root_dict)) = doc.objects.get(&root_id) {
            root_dict.get(b"MarkInfo").is_ok()
        } else {
            false
        }
    } else {
        false
    };

    if !has_tags {
        issues.push(serde_json::json!({
            "severity": "error",
            "message": "PDF is not tagged (required for accessibility)"
        }));
    }

    // Check for document title
    let has_title = if let Ok(info_id) = doc.trailer.get(b"Info").and_then(|o| o.as_reference()) {
        if let Some(Object::Dictionary(ref info_dict)) = doc.objects.get(&info_id) {
            info_dict.get(b"Title").is_ok()
        } else {
            false
        }
    } else {
        false
    };

    if !has_title {
        issues.push(serde_json::json!({
            "severity": "warning",
            "message": "Document title is not set"
        }));
    }

    // Check for language
    let has_lang = if let Ok(root_id) = doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        if let Some(Object::Dictionary(ref root_dict)) = doc.objects.get(&root_id) {
            root_dict.get(b"Lang").is_ok()
        } else {
            false
        }
    } else {
        false
    };

    if !has_lang {
        issues.push(serde_json::json!({
            "severity": "warning",
            "message": "Document language is not set"
        }));
    }

    // Check each page for images without alt text
    for &page_id in &page_ids {
        if let Some(Object::Dictionary(ref dict)) = doc.objects.get(&page_id) {
            if let Ok(Object::Array(annots)) = dict.get(b"Annots") {
                for annot_ref in annots {
                    if let Object::Reference(ref_id) = annot_ref {
                        if let Some(Object::Dictionary(annot_dict)) = doc.objects.get(ref_id) {
                            if let Ok(Object::Name(subtype)) = annot_dict.get(b"Subtype") {
                                if subtype == b"Widget" {
                                    // Form field - check for tooltip
                                    if annot_dict.get(b"TU").is_err() {
                                        issues.push(serde_json::json!({
                                            "severity": "warning",
                                            "message": "Form field missing tooltip (TU entry)"
                                        }));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let score = if issues.is_empty() { 100 } else { 
        100 - (issues.iter().filter(|i| i["severity"] == "error").count() * 20) 
              - (issues.iter().filter(|i| i["severity"] == "warning").count() * 10)
    };

    Ok(serde_json::json!({
        "score": score.max(0),
        "issues": issues,
        "page_count": page_ids.len(),
        "has_tags": has_tags,
        "has_title": has_title,
        "has_language": has_lang,
    }))
}

/// Automatically repair accessibility issues (inject MarkInfo/Marked, default Lang, Title, and field Tooltips)
pub fn fix_accessibility_issues(
    data: &[u8],
    default_title: &str,
    default_lang: &str,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;

    let root_id = match doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        Ok(id) => id,
        Err(_) => return Err("No Root catalog found in PDF".into()),
    };

    let title_str = if default_title.trim().is_empty() { "Accessible Document" } else { default_title };
    let lang_str = if default_lang.trim().is_empty() { "ja-JP" } else { default_lang };

    // 1. Mark Root with MarkInfo /Marked true and Lang
    if let Some(Object::Dictionary(ref mut root_dict)) = doc.objects.get_mut(&root_id) {
        let mut mark_info = Dictionary::new();
        mark_info.set("Marked", Object::Boolean(true));
        root_dict.set("MarkInfo", Object::Dictionary(mark_info));
        root_dict.set("Lang", Object::String(lang_str.as_bytes().to_vec(), lopdf::StringFormat::Literal));

        // Set ViewerPreferences to display DocTitle
        let mut vp = Dictionary::new();
        vp.set("DisplayDocTitle", Object::Boolean(true));
        root_dict.set("ViewerPreferences", Object::Dictionary(vp));
    }

    // 2. Set Info Title
    let info_id = if let Ok(id) = doc.trailer.get(b"Info").and_then(|o| o.as_reference()) {
        id
    } else {
        let info_dict = Dictionary::new();
        doc.add_object(Object::Dictionary(info_dict))
    };

    if let Some(Object::Dictionary(ref mut info_dict)) = doc.objects.get_mut(&info_id) {
        if info_dict.get(b"Title").is_err() {
            info_dict.set("Title", Object::String(title_str.as_bytes().to_vec(), lopdf::StringFormat::Literal));
        }
    }
    doc.trailer.set("Info", Object::Reference(info_id));

    // 3. Add Tooltips (TU) to Widget form fields if missing
    let page_ids = get_page_ids(&doc);
    for &page_id in &page_ids {
        let annot_refs: Vec<OID> = if let Some(Object::Dictionary(ref pdict)) = doc.objects.get(&page_id) {
            pdict.get(b"Annots")
                .ok()
                .and_then(|a| a.as_array().ok())
                .map(|arr| arr.iter().filter_map(|o| o.as_reference().ok()).collect())
                .unwrap_or_default()
        } else {
            vec![]
        };

        for aref in annot_refs {
            if let Some(Object::Dictionary(ref mut adict)) = doc.objects.get_mut(&aref) {
                if let Ok(Object::Name(sub)) = adict.get(b"Subtype") {
                    if sub == b"Widget" && adict.get(b"TU").is_err() {
                        let name_desc = adict.get(b"T")
                            .ok()
                            .and_then(|o| match o {
                                Object::String(b, _) => Some(String::from_utf8_lossy(b).to_string()),
                                _ => None,
                            })
                            .unwrap_or_else(|| "Form Input Field".to_string());
                        adict.set("TU", Object::String(name_desc.as_bytes().to_vec(), lopdf::StringFormat::Literal));
                    }
                }
            }
        }
    }

    save_doc(&mut doc)
}
