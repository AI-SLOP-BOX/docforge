use super::common::*;
use super::*;
use lopdf::{Dictionary, Document, Object, Stream};

pub fn pdf_to_word(data: &[u8], output_path: &str) -> Result<(), String> {
    let doc = Document::load_mem(data).map_err(|e| e.to_string())?;
    let page_ids = get_page_ids(&doc);

    // Generate clean Word-compatible HTML format with styling and semantic headers
    let mut html = String::from(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>Document Export</title>
<style>
body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; line-height: 1.6; margin: 40px; color: #111; }
h1, h2 { color: #004488; border-bottom: 1px solid #ddd; padding-bottom: 4px; }
.page-break { page-break-after: always; border-top: 1px dashed #ccc; margin: 30px 0; }
p { margin: 8px 0; }
</style>
</head>
<body>
"#,
    );

    for (i, &_page_id) in page_ids.iter().enumerate() {
        html.push_str(&format!("<h2>ページ {}</h2>\n", i + 1));
        if let Ok(page_text) = get_page_text(data, i) {
            for line in page_text.lines() {
                let trimmed = line.trim();
                let escaped = trimmed
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
                    .replace('"', "&quot;");
                html.push_str(&format!("<p>{}</p>\n", escaped));
            }
        }
        if i + 1 < page_ids.len() {
            html.push_str("<div class=\"page-break\"></div>\n");
        }
    }

    html.push_str("</body></html>");
    std::fs::write(output_path, html).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn pdf_to_excel(data: &[u8], output_path: &str) -> Result<(), String> {
    let doc = Document::load_mem(data).map_err(|e| e.to_string())?;
    let page_ids = get_page_ids(&doc);

    // Intelligently parse tabular columns by whitespace / delimiters
    let mut csv = String::from("\u{FEFF}"); // UTF-8 BOM for Excel Japanese compatibility
    csv.push_str("ページ,行番号,列1,列2,列3,列4,列5\n");

    for (i, &_page_id) in page_ids.iter().enumerate() {
        if let Ok(page_text) = get_page_text(data, i) {
            for (line_idx, line) in page_text.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // Split columns by tabs, multiple spaces, or commas
                let cols: Vec<&str> = trimmed
                    .split(|c: char| c == '\t' || c == ',')
                    .flat_map(|part| {
                        if part.contains("  ") {
                            part.split_whitespace().collect::<Vec<&str>>()
                        } else {
                            vec![part.trim()]
                        }
                    })
                    .filter(|s| !s.is_empty())
                    .collect();

                let mut row = format!("{},{}", i + 1, line_idx + 1);
                for c in 0..5 {
                    if let Some(val) = cols.get(c) {
                        let escaped = val.replace('"', "\"\"");
                        row.push_str(&format!(",\"{}\"", escaped));
                    } else {
                        row.push_str(",\"\"");
                    }
                }
                row.push('\n');
                csv.push_str(&row);
            }
        }
    }

    std::fs::write(output_path, csv).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn pdf_to_powerpoint(data: &[u8], output_path: &str) -> Result<(), String> {
    let doc = Document::load_mem(data).map_err(|e| e.to_string())?;
    let page_ids = get_page_ids(&doc);

    let mut html = String::from(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>PDF Slides</title>
<style>
body { margin: 0; padding: 0; }
.slide { width: 100vw; height: 100vh; page-break-after: always; display: flex; align-items: center; justify-content: center; }
.slide img { max-width: 100%; max-height: 100%; }
</style>
</head>
<body>
"#,
    );

    let tmp = std::env::temp_dir().join("docforge_pdf2ppt");
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;

    for i in 0..page_ids.len() {
        let images = pdf_to_images(data, &tmp.to_string_lossy(), "png", 150)?;
        if let Some(img_path) = images.get(i) {
            html.push_str(&format!(
                r#"<div class="slide"><img src="{}"></div>"#,
                img_path
            ));
        }
    }

    html.push_str("</body></html>");

    std::fs::write(output_path, html).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn create_pdf_portfolio(file_paths: &[String], output_path: &str) -> Result<(), String> {
    let mut doc = Document::with_version("1.7");
    let mut files = Vec::new();

    for path in file_paths {
        let path_obj = std::path::Path::new(path);
        let filename = path_obj
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".into());

        let file_data = std::fs::read(path).map_err(|e| format!("Failed to read {path}: {e}"))?;
        let file_size = file_data.len() as i64;

        let mut embed_dict = Dictionary::new();
        embed_dict.set("Type", Object::Name("EmbeddedFile".into()));
        embed_dict.set("Size", Object::Integer(file_size));

        let embed_stream = Stream::new(embed_dict, file_data);
        let embed_id = doc.add_object(Object::Stream(embed_stream));

        let mut fs_dict = Dictionary::new();
        fs_dict.set("Type", Object::Name("Filespec".into()));
        fs_dict.set(
            "F",
            Object::String(filename.clone().into_bytes(), lopdf::StringFormat::Literal),
        );
        fs_dict.set(
            "UF",
            Object::String(filename.into_bytes(), lopdf::StringFormat::Literal),
        );
        fs_dict.set(
            "EF",
            Object::Dictionary({
                let mut ef = Dictionary::new();
                ef.set("F", Object::Reference(embed_id));
                ef
            }),
        );

        let fs_id = doc.add_object(Object::Dictionary(fs_dict));
        files.push(Object::Reference(fs_id));
    }

    let mut collection_dict = Dictionary::new();
    collection_dict.set("Type", Object::Name("Collection".into()));
    collection_dict.set("View", Object::Name("Detail".into()));
    collection_dict.set("Sort", Object::Name("Name".into()));
    collection_dict.set(
        "Title",
        Object::String(
            "PDF Portfolio".as_bytes().to_vec(),
            lopdf::StringFormat::Literal,
        ),
    );

    let collection_id = doc.add_object(Object::Dictionary(collection_dict));

    // EmbeddedFiles Names tree leaf node:
    // /Names [ (filename1) ref1 (filename2) ref2 ... ]
    let mut names_array = Vec::new();
    for (i, path) in file_paths.iter().enumerate() {
        let filename = std::path::Path::new(path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("file_{i}"));
        names_array.push(Object::String(
            filename.into_bytes(),
            lopdf::StringFormat::Literal,
        ));
        if let Some(Object::Reference(ref_id)) = files.get(i) {
            names_array.push(Object::Reference(*ref_id));
        }
    }

    let mut ef_tree_node = Dictionary::new();
    ef_tree_node.set("Names", Object::Array(names_array));
    let ef_tree_id = doc.add_object(Object::Dictionary(ef_tree_node));

    // Catalog Names dictionary:
    // << /EmbeddedFiles ef_tree_id >>
    let mut names_dict = Dictionary::new();
    names_dict.set("EmbeddedFiles", Object::Reference(ef_tree_id));
    let names_id = doc.add_object(Object::Dictionary(names_dict));

    // Standard cover page so the PDF has valid /Pages structure
    let pages_id = doc.new_object_id();

    let mut page_dict = Dictionary::new();
    page_dict.set("Type", Object::Name("Page".into()));
    page_dict.set("Parent", Object::Reference(pages_id));
    page_dict.set(
        "MediaBox",
        Object::Array(vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(595.0),
            Object::Real(842.0),
        ]),
    );

    let cover_text = "q 1 0 0 1 50 750 cm BT /F1 16 Tf (DocForge PDF Portfolio) Tj ET Q";
    let cover_content_id = doc.add_object(Stream::new(
        Dictionary::new(),
        cover_text.as_bytes().to_vec(),
    ));
    page_dict.set("Contents", Object::Reference(cover_content_id));

    let mut font_dict = Dictionary::new();
    font_dict.set("Type", Object::Name("Font".into()));
    font_dict.set("Subtype", Object::Name("Type1".into()));
    font_dict.set("BaseFont", Object::Name("Helvetica".into()));
    let font_id = doc.add_object(Object::Dictionary(font_dict));

    let mut fonts_res = Dictionary::new();
    fonts_res.set("F1", Object::Reference(font_id));
    let mut res_dict = Dictionary::new();
    res_dict.set("Font", Object::Dictionary(fonts_res));
    page_dict.set("Resources", Object::Dictionary(res_dict));

    let page_id = doc.add_object(Object::Dictionary(page_dict));

    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", Object::Name("Pages".into()));
    pages_dict.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    pages_dict.set("Count", Object::Integer(1));
    doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

    // Valid Document Catalog
    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", Object::Name("Catalog".into()));
    catalog_dict.set("Pages", Object::Reference(pages_id));
    catalog_dict.set("Names", Object::Reference(names_id));
    catalog_dict.set("Collection", Object::Reference(collection_id));

    let catalog_id = doc.add_object(Object::Dictionary(catalog_dict));
    doc.trailer.set("Root", Object::Reference(catalog_id));

    let mut buf = Vec::new();
    doc.save_to(&mut buf).map_err(|e| e.to_string())?;
    std::fs::write(output_path, buf).map_err(|e| e.to_string())?;
    Ok(())
}
