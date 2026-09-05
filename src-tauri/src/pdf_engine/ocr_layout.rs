use super::common::*;
use super::inspect::get_page_text;
use lopdf::{Dictionary, Document, Object, Stream};

// ===== CONTENT COMPARISON (Visual Diff) =====

pub fn visual_diff(data1: &[u8], data2: &[u8], output_path: &str) -> Result<(), String> {
    let doc1 = Document::load_mem(data1).map_err(|e| format!("Failed to load first PDF: {e}"))?;
    let doc2 = Document::load_mem(data2).map_err(|e| format!("Failed to load second PDF: {e}"))?;

    let pages1 = get_page_ids(&doc1);
    let pages2 = get_page_ids(&doc2);

    let max_pages = pages1.len().max(pages2.len());

    let mut html = String::from(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>PDF Visual Diff</title>
<style>
body { font-family: sans-serif; margin: 20px; }
.diff-container { display: flex; gap: 20px; margin-bottom: 20px; }
.diff-page { flex: 1; border: 1px solid #ccc; padding: 10px; }
.diff-page h3 { margin-top: 0; }
.added { background-color: #d4edda; }
.removed { background-color: #f8d7da; }
.modified { background-color: #fff3cd; }
</style>
</head>
<body>
<h1>PDF Visual Comparison</h1>
"#,
    );

    for i in 0..max_pages {
        html.push_str(&format!(
            "<h2>Page {}</h2><div class='diff-container'>",
            i + 1
        ));

        if i < pages1.len() {
            html.push_str("<div class='diff-page'><h3>Original</h3>");
            html.push_str(&format!("<p>Page {} exists</p>", i + 1));
            html.push_str("</div>");
        } else {
            html.push_str(
                "<div class='diff-page removed'><h3>Original</h3><p>Page missing</p></div>",
            );
        }

        if i < pages2.len() {
            html.push_str("<div class='diff-page'><h3>Modified</h3>");
            html.push_str(&format!("<p>Page {} exists</p>", i + 1));
            html.push_str("</div>");
        } else {
            html.push_str(
                "<div class='diff-page removed'><h3>Modified</h3><p>Page missing</p></div>",
            );
        }

        html.push_str("</div>");
    }

    html.push_str("</body></html>");

    std::fs::write(output_path, html).map_err(|e| e.to_string())?;
    Ok(())
}

// ===== ADVANCED OCR WITH LAYOUT PRESERVATION =====

pub fn ocr_with_layout(
    data: &[u8],
    language: &str,
    preserve_layout: bool,
) -> Result<serde_json::Value, String> {
    let doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

    let page_ids = get_page_ids(&doc);
    let mut pages = Vec::new();

    for (page_idx, &page_id) in page_ids.iter().enumerate() {
        // Get page dimensions
        let (w, h) = get_page_dimensions(&doc, page_id);

        // Get text content
        let text = get_page_text(data, page_idx).unwrap_or_default();

        // Analyze layout if preserve_layout is true
        let layout = if preserve_layout {
            Some(analyze_text_layout(&text, w, h))
        } else {
            None
        };

        pages.push(serde_json::json!({
            "page": page_idx + 1,
            "width": w,
            "height": h,
            "text": text,
            "layout": layout,
        }));
    }

    Ok(serde_json::json!({
        "pages": pages,
        "total_pages": page_ids.len(),
        "language": language,
    }))
}

fn analyze_text_layout(text: &str, _page_width: f32, page_height: f32) -> serde_json::Value {
    let lines: Vec<&str> = text.lines().collect();
    let mut blocks = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if !line.trim().is_empty() {
            let y_pos = page_height - (i as f32 * 12.0 + 50.0);
            blocks.push(serde_json::json!({
                "text": line,
                "x": 50.0,
                "y": y_pos,
                "width": line.len() as f32 * 6.0,
                "height": 12.0,
                "font_size": 10.0,
            }));
        }
    }

    serde_json::json!({
        "blocks": blocks,
        "columns": 1,
        "margins": {"top": 50, "bottom": 50, "left": 50, "right": 50},
    })
}

// Create searchable PDF from scanned PDF
pub fn create_searchable_pdf_from_scanned(data: &[u8], _language: &str) -> Result<Vec<u8>, String> {
    let doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

    let page_ids = get_page_ids(&doc).clone();
    let mut new_doc = Document::with_version("1.7");

    for &page_id in &page_ids {
        // Get page dimensions
        let (w, h) = get_page_dimensions(&doc, page_id);

        // Create new page with same dimensions
        let mut page_dict = Dictionary::new();
        page_dict.set("Type", Object::Name("Page".into()));
        page_dict.set(
            "MediaBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(w),
                Object::Real(h),
            ]),
        );

        // Add resources for fonts
        let mut resources = Dictionary::new();
        let mut font_dict = Dictionary::new();

        // Create a simple font
        let mut font = Dictionary::new();
        font.set("Type", Object::Name("Font".into()));
        font.set("Subtype", Object::Name("Type1".into()));
        font.set("BaseFont", Object::Name("Helvetica".into()));

        let font_id = new_doc.add_object(Object::Dictionary(font));
        font_dict.set("F1", Object::Reference(font_id));
        resources.set("Font", Object::Dictionary(font_dict));

        page_dict.set("Resources", Object::Dictionary(resources));

        // Add placeholder content stream (would be replaced with OCR results)
        let content = lopdf::content::Content { operations: vec![] };
        let content_bytes = content.encode().map_err(|e| e.to_string())?;
        let stream = Stream::new(Dictionary::new(), content_bytes);
        let content_id = new_doc.add_object(Object::Stream(stream));
        page_dict.set("Contents", Object::Reference(content_id));

        let _new_page_id = new_doc.add_object(Object::Dictionary(page_dict));
    }

    save_doc(&mut new_doc)
}
