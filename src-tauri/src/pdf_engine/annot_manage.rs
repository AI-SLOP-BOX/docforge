use super::common::*;
use lopdf::{Dictionary, Document, Object};

// ===== ANNOTATION MANAGEMENT =====

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Annotation {
    pub id: OID,
    pub annot_type: String,
    pub contents: String,
    pub author: String,
    pub page: usize,
    pub x: f64,
    pub y: f64,
    pub color: String,
    pub status: String,
    pub replies: Vec<AnnotationReply>,
    pub created: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct AnnotationReply {
    pub id: OID,
    pub author: String,
    pub contents: String,
    pub created: String,
}

pub fn get_annotations(data: &[u8]) -> Result<Vec<serde_json::Value>, String> {
    let doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    let mut annotations = Vec::new();

    for (page_idx, &page_id) in page_ids.iter().enumerate() {
        if let Some(Object::Dictionary(ref dict)) = doc.objects.get(&page_id) {
            if let Ok(Object::Array(annots)) = dict.get(b"Annots") {
                for annot_ref in annots {
                    if let Object::Reference(ref_id) = annot_ref {
                        if let Some(Object::Dictionary(annot_dict)) = doc.objects.get(ref_id) {
                            let annot_type = annot_dict
                                .get(b"Subtype")
                                .ok()
                                .and_then(|o| match o {
                                    Object::Name(bytes) => {
                                        Some(String::from_utf8_lossy(bytes).to_string())
                                    }
                                    _ => None,
                                })
                                .unwrap_or_default();

                            let contents = annot_dict
                                .get(b"Contents")
                                .ok()
                                .and_then(|o| match o {
                                    Object::String(bytes, _) => {
                                        Some(String::from_utf8_lossy(bytes).to_string())
                                    }
                                    _ => None,
                                })
                                .unwrap_or_default();

                            let author = annot_dict
                                .get(b"T")
                                .ok()
                                .and_then(|o| match o {
                                    Object::String(bytes, _) => {
                                        Some(String::from_utf8_lossy(bytes).to_string())
                                    }
                                    _ => None,
                                })
                                .unwrap_or_default();

                            let status = annot_dict
                                .get(b"Name")
                                .ok()
                                .and_then(|o| match o {
                                    Object::Name(bytes) => {
                                        Some(String::from_utf8_lossy(bytes).to_string())
                                    }
                                    _ => None,
                                })
                                .unwrap_or_default();

                            let (x, y) = match annot_dict.get(b"Rect") {
                                Ok(Object::Array(arr)) if arr.len() >= 2 => {
                                    let x = match &arr[0] {
                                        Object::Real(v) => *v as f64,
                                        Object::Integer(v) => *v as f64,
                                        _ => 0.0,
                                    };
                                    let y = match &arr[1] {
                                        Object::Real(v) => *v as f64,
                                        Object::Integer(v) => *v as f64,
                                        _ => 0.0,
                                    };
                                    (x, y)
                                }
                                _ => (0.0, 0.0),
                            };

                            let color = match annot_dict.get(b"C") {
                                Ok(Object::Array(arr)) if arr.len() >= 3 => {
                                    let r = match &arr[0] {
                                        Object::Real(v) => (v * 255.0) as u8,
                                        _ => 0,
                                    };
                                    let g = match &arr[1] {
                                        Object::Real(v) => (v * 255.0) as u8,
                                        _ => 0,
                                    };
                                    let b = match &arr[2] {
                                        Object::Real(v) => (v * 255.0) as u8,
                                        _ => 0,
                                    };
                                    format!("#{:02X}{:02X}{:02X}", r, g, b)
                                }
                                _ => "#FF0000".to_string(),
                            };

                            // Check for replies (IRT - In Reply To)
                            let mut replies = Vec::new();
                            for (_, reply_obj) in doc.objects.iter() {
                                if let Object::Dictionary(reply_dict) = reply_obj {
                                    if let Ok(Object::Reference(irt_ref)) = reply_dict.get(b"IRT") {
                                        if irt_ref == ref_id {
                                            let reply_contents = reply_dict
                                                .get(b"Contents")
                                                .ok()
                                                .and_then(|o| match o {
                                                    Object::String(bytes, _) => Some(
                                                        String::from_utf8_lossy(bytes).to_string(),
                                                    ),
                                                    _ => None,
                                                })
                                                .unwrap_or_default();

                                            let reply_author = reply_dict
                                                .get(b"T")
                                                .ok()
                                                .and_then(|o| match o {
                                                    Object::String(bytes, _) => Some(
                                                        String::from_utf8_lossy(bytes).to_string(),
                                                    ),
                                                    _ => None,
                                                })
                                                .unwrap_or_default();

                                            replies.push(serde_json::json!({
                                                "author": reply_author,
                                                "contents": reply_contents,
                                            }));
                                        }
                                    }
                                }
                            }

                            annotations.push(serde_json::json!({
                                "id": format!("{:?}", ref_id),
                                "type": annot_type,
                                "contents": contents,
                                "author": author,
                                "page": page_idx,
                                "x": x,
                                "y": y,
                                "color": color,
                                "status": status,
                                "replies": replies,
                            }));
                        }
                    }
                }
            }
        }
    }

    Ok(annotations)
}

pub fn add_annotation_reply(
    data: &[u8],
    annotation_id: (u32, u16),
    author: &str,
    contents: &str,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

    let mut reply_dict = Dictionary::new();
    reply_dict.set("Type", Object::Name("Annot".into()));
    reply_dict.set("Subtype", Object::Name("Text".into()));
    reply_dict.set(
        "T",
        Object::String(author.as_bytes().to_vec(), lopdf::StringFormat::Literal),
    );
    reply_dict.set(
        "Contents",
        Object::String(contents.as_bytes().to_vec(), lopdf::StringFormat::Literal),
    );
    reply_dict.set("IRT", Object::Reference(annotation_id));
    reply_dict.set(
        "Rect",
        Object::Array(vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(0.0),
        ]),
    );

    let reply_id = doc.add_object(Object::Dictionary(reply_dict));

    // Find the page containing the parent annotation and add reply to its Annots
    for (_, obj) in doc.objects.iter_mut() {
        if let Object::Dictionary(ref mut dict) = obj {
            if let Ok(Object::Array(annots)) = dict.get(b"Annots") {
                for annot_ref in annots {
                    if let Object::Reference(id) = annot_ref {
                        if *id == annotation_id {
                            // Found parent in this page's annots, add reply
                            let mut new_annots = annots.clone();
                            new_annots.push(Object::Reference(reply_id));
                            dict.set("Annots", Object::Array(new_annots));
                            break;
                        }
                    }
                }
            }
        }
    }

    save_doc(&mut doc)
}

pub fn set_annotation_status(
    data: &[u8],
    annotation_id: (u32, u16),
    status: &str,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

    if let Some(Object::Dictionary(ref mut annot_dict)) = doc.objects.get_mut(&annotation_id) {
        annot_dict.set("Name", Object::Name(status.as_bytes().to_vec()));
    }

    save_doc(&mut doc)
}

pub fn delete_annotation(data: &[u8], annotation_id: (u32, u16)) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

    // Remove the annotation object
    doc.objects.remove(&annotation_id);

    // Remove reference from page Annots
    for (_, obj) in doc.objects.iter_mut() {
        if let Object::Dictionary(ref mut dict) = obj {
            if let Ok(Object::Array(annots)) = dict.get(b"Annots") {
                let new_annots: Vec<Object> = annots
                    .iter()
                    .filter(|a| {
                        if let Object::Reference(id) = a {
                            *id != annotation_id
                        } else {
                            true
                        }
                    })
                    .cloned()
                    .collect();
                dict.set("Annots", Object::Array(new_annots));
            }
        }
    }

    // Also remove any replies
    let mut replies_to_remove = Vec::new();
    for (id, obj) in doc.objects.iter() {
        if let Object::Dictionary(dict) = obj {
            if let Ok(Object::Reference(irt_ref)) = dict.get(b"IRT") {
                if *irt_ref == annotation_id {
                    replies_to_remove.push(*id);
                }
            }
        }
    }

    for reply_id in replies_to_remove {
        doc.objects.remove(&reply_id);
    }

    save_doc(&mut doc)
}
