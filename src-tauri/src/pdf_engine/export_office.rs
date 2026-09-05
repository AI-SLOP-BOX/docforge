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

    let mut names_dict = Dictionary::new();
    names_dict.set("EmbeddedFiles", Object::Array(names_array));

    let names_id = doc.add_object(Object::Dictionary(names_dict));

    doc.trailer.set("Root", Object::Reference(names_id));

    let mut root_dict = Dictionary::new();
    root_dict.set("Collection", Object::Reference(collection_id));
    doc.objects.insert(names_id, Object::Dictionary(root_dict));

    let mut buf = Vec::new();
    doc.save_to(&mut buf).map_err(|e| e.to_string())?;
    std::fs::write(output_path, buf).map_err(|e| e.to_string())?;
    Ok(())
}
