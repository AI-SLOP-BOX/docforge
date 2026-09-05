use lopdf::{Document, Object};
use super::common::*;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct DiffItem {
    pub page: usize,
    pub kind: String, // "added" | "deleted" | "modified"
    pub original_text: String,
    pub revised_text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct CompareReport {
    pub total_pages_original: usize,
    pub total_pages_revised: usize,
    pub total_changes: usize,
    pub changes_added: usize,
    pub changes_deleted: usize,
    pub changes_modified: usize,
    pub diffs: Vec<DiffItem>,
}

/// Professional semantic & graphical document comparison.
/// Compares text blocks across pages and detects changes, additions, and deletions.
pub fn compare_pdf_documents(original: &[u8], revised: &[u8]) -> Result<CompareReport, String> {
    let doc_orig = Document::load_mem(original)
        .map_err(|e| format!("Failed to parse original PDF: {e}"))?;
    let doc_rev = Document::load_mem(revised)
        .map_err(|e| format!("Failed to parse revised PDF: {e}"))?;

    let orig_pages = get_page_ids(&doc_orig);
    let rev_pages = get_page_ids(&doc_rev);

    let max_pages = orig_pages.len().max(rev_pages.len());
    let mut diffs = Vec::new();
    let mut added_count = 0;
    let mut deleted_count = 0;
    let mut modified_count = 0;

    for p in 0..max_pages {
        let text_orig = if p < orig_pages.len() {
            extract_page_text(&doc_orig, orig_pages[p])
        } else {
            Vec::new()
        };

        let text_rev = if p < rev_pages.len() {
            extract_page_text(&doc_rev, rev_pages[p])
        } else {
            Vec::new()
        };

        // Compute block-level differences
        let mut rev_matched = vec![false; text_rev.len()];

        for orig_item in &text_orig {
            let mut found = false;
            for (rj, rev_item) in text_rev.iter().enumerate() {
                if !rev_matched[rj] && orig_item.text.trim() == rev_item.text.trim() {
                    rev_matched[rj] = true;
                    found = true;
                    break;
                }
            }

            if !found {
                // Check if it was modified (close position)
                let mut is_mod = false;
                for (rj, rev_item) in text_rev.iter().enumerate() {
                    if !rev_matched[rj] && (orig_item.y - rev_item.y).abs() < 15.0 {
                        diffs.push(DiffItem {
                            page: p,
                            kind: "modified".into(),
                            original_text: orig_item.text.clone(),
                            revised_text: rev_item.text.clone(),
                            x: rev_item.x,
                            y: rev_item.y,
                            width: rev_item.width.max(orig_item.width),
                            height: rev_item.height.max(orig_item.height),
                        });
                        rev_matched[rj] = true;
                        modified_count += 1;
                        is_mod = true;
                        break;
                    }
                }

                if !is_mod {
                    diffs.push(DiffItem {
                        page: p,
                        kind: "deleted".into(),
                        original_text: orig_item.text.clone(),
                        revised_text: String::new(),
                        x: orig_item.x,
                        y: orig_item.y,
                        width: orig_item.width,
                        height: orig_item.height,
                    });
                    deleted_count += 1;
                }
            }
        }

        // Remaining unmatched in rev are added
        for (rj, rev_item) in text_rev.iter().enumerate() {
            if !rev_matched[rj] {
                diffs.push(DiffItem {
                    page: p,
                    kind: "added".into(),
                    original_text: String::new(),
                    revised_text: rev_item.text.clone(),
                    x: rev_item.x,
                    y: rev_item.y,
                    width: rev_item.width,
                    height: rev_item.height,
                });
                added_count += 1;
            }
        }
    }

    let total = added_count + deleted_count + modified_count;

    Ok(CompareReport {
        total_pages_original: orig_pages.len(),
        total_pages_revised: rev_pages.len(),
        total_changes: total,
        changes_added: added_count,
        changes_deleted: deleted_count,
        changes_modified: modified_count,
        diffs,
    })
}

struct SimpleTextBlock {
    text: String,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

fn extract_page_text(doc: &Document, page_id: OID) -> Vec<SimpleTextBlock> {
    let mut blocks = Vec::new();

    let contents_id = if let Some(obj) = doc.objects.get(&page_id) {
        if let Ok(d) = obj.as_dict() {
            d.get(b"Contents").ok().and_then(|c| c.as_reference().ok())
        } else {
            None
        }
    } else {
        None
    };

    if let Some(cid) = contents_id {
        if let Some(obj) = doc.objects.get(&cid) {
            if let Ok(stream) = obj.as_stream() {
                if let Ok(content) = lopdf::content::Content::decode(&stream.content) {
                    let mut current_x = 0.0f32;
                    let mut current_y = 0.0f32;

                    for op in content.operations {
                        match op.operator.as_str() {
                            "Td" | "TD" => {
                                if op.operands.len() >= 2 {
                                    if let (Ok(dx), Ok(dy)) = (op.operands[0].as_float(), op.operands[1].as_float()) {
                                        current_x += dx;
                                        current_y += dy;
                                    }
                                }
                            }
                            "Tm" => {
                                if op.operands.len() >= 6 {
                                    if let (Ok(tx), Ok(ty)) = (op.operands[4].as_float(), op.operands[5].as_float()) {
                                        current_x = tx;
                                        current_y = ty;
                                    }
                                }
                            }
                            "Tj" => {
                                if let Some(s) = op.operands.first().and_then(|o| match o {
                                    Object::String(bytes, _) => Some(String::from_utf8_lossy(bytes).to_string()),
                                    _ => None,
                                }) {
                                    let str_len = s.chars().count() as f32;
                                    blocks.push(SimpleTextBlock {
                                        text: s,
                                        x: current_x,
                                        y: current_y,
                                        width: (str_len * 7.5).max(20.0),
                                        height: 14.0,
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    blocks
}
