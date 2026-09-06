use std::path::Path;
use std::process::Command;

#[derive(serde::Serialize, Clone)]
pub struct OCRSuspect {
    pub text: String,
    pub confidence: f64,
    pub line_num: usize,
    pub word_num: usize,
}

#[derive(serde::Serialize)]
pub struct OCRResult {
    pub text: String,
    pub confidence: f64,
    pub page_count: usize,
    pub suspects: Vec<OCRSuspect>,
}

pub fn ocr_files(paths: &[String], language: &str) -> Result<OCRResult, String> {
    let tess_lang = match language {
        "jpn" => "jpn",
        "eng" => "eng",
        "jpn+eng" => "jpn+eng",
        "chi_sim" => "chi_sim",
        "kor" => "kor",
        _ => "jpn",
    };

    let mut full_text = String::new();
    let mut total_confidence = 0.0;
    let mut page_count = 0;
    let mut all_suspects = Vec::new();

    for path in paths {
        let path = Path::new(path);
        if path.extension().map(|e| e.to_string_lossy().to_lowercase()) == Some("pdf".into()) {
            let (_guard, images) = pdf_to_images(path)?;
            for img_path in &images {
                let (text, conf, suspects) = run_tesseract(img_path, tess_lang)?;
                full_text.push_str(&text);
                total_confidence += conf;
                page_count += 1;
                all_suspects.extend(suspects);
            }
            // _guard drops here or on early error return (`?`), automatically removing the entire temp directory
        } else {
            let (text, conf, suspects) = run_tesseract(path.to_str().unwrap_or(""), tess_lang)?;
            full_text.push_str(&text);
            total_confidence += conf;
            page_count += 1;
            all_suspects.extend(suspects);
        }
    }

    let avg_confidence = if page_count > 0 {
        total_confidence / page_count as f64
    } else {
        0.0
    };

    Ok(OCRResult {
        text: full_text,
        confidence: avg_confidence,
        page_count,
        suspects: all_suspects,
    })
}

fn run_tesseract(
    image_path: &str,
    language: &str,
) -> Result<(String, f64, Vec<OCRSuspect>), String> {
    // Single tesseract invocation in TSV mode to get text, geometry, and confidence in one pass
    let output = Command::new("tesseract")
        .args([
            image_path,
            "stdout",
            "-l",
            language,
            "--psm",
            "3", // Fully automatic page segmentation
            "--oem",
            "3", // Default OCR engine (LSTM + legacy)
            "-c",
            "preserve_interword_spaces=1",
            "tsv",
        ])
        .output()
        .map_err(|e| {
            format!("Failed to run tesseract (install: brew install tesseract tesseract-lang): {e}")
        })?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let tsv_content = String::from_utf8_lossy(&output.stdout);
    let mut total_conf = 0.0;
    let mut count = 0;
    let mut suspects = Vec::new();
    let mut reconstructed_text = String::new();
    let mut current_line_key = (-1, -1); // (par_num, line_num)

    for line in tsv_content.lines().skip(1) {
        // skip header
        let cols: Vec<&str> = line.split('\t').collect();
        // TSV format: level, page_num, block_num, par_num, line_num, word_num, left, top, width, height, conf, text
        if cols.len() >= 12 {
            let par_num = cols[3].parse::<i32>().unwrap_or(0);
            let line_num = cols[4].parse::<i32>().unwrap_or(0);
            let word_text = cols[11].trim();

            if !word_text.is_empty() {
                if current_line_key != (par_num, line_num) {
                    if !reconstructed_text.is_empty() {
                        reconstructed_text.push('\n');
                    }
                    current_line_key = (par_num, line_num);
                } else {
                    reconstructed_text.push(' ');
                }
                reconstructed_text.push_str(word_text);
            }

            if let Ok(conf) = cols[10].parse::<f64>() {
                if conf > 0.0 {
                    total_conf += conf;
                    count += 1;

                    if conf < 75.0 && !word_text.is_empty() {
                        let word_num = cols[5].parse::<usize>().unwrap_or(0);
                        suspects.push(OCRSuspect {
                            text: word_text.to_string(),
                            confidence: conf,
                            line_num: line_num.max(0) as usize,
                            word_num,
                        });
                    }
                }
            }
        }
    }

    if !reconstructed_text.is_empty() {
        reconstructed_text.push('\n');
    }

    let avg_conf = if count > 0 {
        total_conf / count as f64
    } else {
        85.0
    };

    Ok((reconstructed_text, avg_conf, suspects))
}

use std::sync::atomic::{AtomicU64, Ordering};

pub struct AutoCleanupDir(pub std::path::PathBuf);

impl Drop for AutoCleanupDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

static OCR_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn pdf_to_images(pdf_path: &Path) -> Result<(AutoCleanupDir, Vec<String>), String> {
    let count = OCR_TEMP_COUNTER.fetch_add(1, Ordering::SeqCst);
    let unique_name = format!(
        "docforge_ocr_{}_{}_{}",
        std::process::id(),
        count,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let dir = std::env::temp_dir().join(unique_name);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create temp dir: {e}"))?;
    let cleanup_guard = AutoCleanupDir(dir.clone());

    let prefix = dir.join("page").to_string_lossy().to_string();

    let cmd = Command::new("pdftoppm")
        .args([
            "-png",
            "-r",
            "300",
            pdf_path.to_str().unwrap_or(""),
            &prefix,
        ])
        .output()
        .map_err(|e| {
            format!("Failed to run pdftoppm (install: brew install poppler): {e}")
        })?;

    if !cmd.status.success() {
        return Err(String::from_utf8_lossy(&cmd.stderr).to_string());
    }

    let mut images = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("page") && name.ends_with(".png") {
                images.push(entry.path().to_string_lossy().to_string());
            }
        }
    }
    images.sort();
    Ok((cleanup_guard, images))
}

pub fn create_epub(text: &str, output_path: &str, title: &str) -> Result<(), String> {
    use epub_builder::EpubBuilder;
    use epub_builder::ZipLibrary;
    use std::fs::File;

    let mut file = File::create(output_path).map_err(|e| format!("Failed to create file: {e}"))?;

    let zip = ZipLibrary::new().map_err(|e| format!("Failed to create zip library: {e}"))?;
    let mut builder =
        EpubBuilder::new(zip).map_err(|e| format!("Failed to create EPUB builder: {e}"))?;

    builder
        .metadata("title", title)
        .map_err(|e| format!("Failed to set metadata: {e}"))?;

    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    let mut chapter_num = 0;

    for para_group in paragraphs {
        let trimmed = para_group.trim();
        if trimmed.is_empty() {
            continue;
        }
        chapter_num += 1;

        let body_html: String = trimmed
            .lines()
            .map(|line| format!("<p>{}</p>", html_escape(line)))
            .collect::<Vec<_>>()
            .join("\n");

        let html_content = format!(
            "<html><head><title>Chapter {chapter_num}</title></head>\
             <body><h1>Chapter {chapter_num}</h1>{body_html}</body></html>"
        );

        let content = epub_builder::EpubContent::new(
            format!("chapter_{chapter_num}.html"),
            html_content.as_bytes(),
        )
        .title(format!("Chapter {chapter_num}"));

        builder
            .add_content(content)
            .map_err(|e| format!("Failed to add chapter: {e}"))?;
    }

    builder
        .generate(&mut file)
        .map_err(|e| format!("Failed to generate EPUB: {e}"))?;

    Ok(())
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn create_searchable_pdf(
    original_paths: &[String],
    ocr_text: &str,
    output_path: &str,
) -> Result<(), String> {
    use lopdf::content::{Content, Operation};
    use lopdf::{Dictionary, Document, Object, Stream};

    let mut doc = Document::with_version("1.7");
    let pages_id = doc.add_object(Object::Dictionary(Dictionary::new()));

    // Shared standard font resource for OCR text layer
    let mut font_dict = Dictionary::new();
    font_dict.set("Type", Object::Name("Font".into()));
    font_dict.set("Subtype", Object::Name("Type1".into()));
    font_dict.set("BaseFont", Object::Name("Helvetica".into()));
    let font_id = doc.add_object(Object::Dictionary(font_dict));

    let mut page_refs = Vec::new();
    let lines: Vec<&str> = ocr_text.lines().collect();

    if !original_paths.is_empty() {
        let lines_per_page = (lines.len() / original_paths.len()).max(1);

        for (page_idx, path) in original_paths.iter().enumerate() {
            let img = image::open(path).map_err(|e| format!("Failed to open image {path}: {e}"))?;
            let rgb = img.to_rgb8();
            let (width, height) = rgb.dimensions();
            let pt_w = (width as f32 * 72.0 / 300.0).max(1.0);
            let pt_h = (height as f32 * 72.0 / 300.0).max(1.0);

            let mut jpeg_buf = std::io::Cursor::new(Vec::new());
            img.write_to(&mut jpeg_buf, image::ImageFormat::Jpeg)
                .map_err(|e| format!("Failed to encode image to JPEG: {e}"))?;
            let jpeg_bytes = jpeg_buf.into_inner();

            let mut img_dict = Dictionary::new();
            img_dict.set("Type", Object::Name("XObject".into()));
            img_dict.set("Subtype", Object::Name("Image".into()));
            img_dict.set("Width", Object::Integer(width as i64));
            img_dict.set("Height", Object::Integer(height as i64));
            img_dict.set("ColorSpace", Object::Name("DeviceRGB".into()));
            img_dict.set("BitsPerComponent", Object::Integer(8));
            img_dict.set("Filter", Object::Name("DCTDecode".into()));

            let img_stream = Stream::new(img_dict, jpeg_bytes);
            let img_id = doc.add_object(Object::Stream(img_stream));

            let mut operations = Vec::new();
            operations.push(Operation::new("q", vec![]));
            operations.push(Operation::new(
                "cm",
                vec![
                    Object::Real(pt_w),
                    Object::Real(0.0),
                    Object::Real(0.0),
                    Object::Real(pt_h),
                    Object::Real(0.0),
                    Object::Real(0.0),
                ],
            ));
            operations.push(Operation::new("Do", vec![Object::Name("Im1".into())]));
            operations.push(Operation::new("Q", vec![]));

            // Overlay invisible selectable text (rendering mode 3 Tr)
            operations.push(Operation::new("BT", vec![]));
            operations.push(Operation::new(
                "Tf",
                vec![Object::Name("F1".into()), Object::Real(10.0)],
            ));
            operations.push(Operation::new("Tr", vec![Object::Integer(3)]));

            let start_line = page_idx * lines_per_page;
            let end_line = if page_idx == original_paths.len() - 1 {
                lines.len()
            } else {
                (start_line + lines_per_page).min(lines.len())
            };

            let mut y = pt_h - 20.0;
            for line_idx in start_line..end_line {
                if y < 20.0 {
                    break;
                }
                operations.push(Operation::new(
                    "Td",
                    vec![Object::Real(20.0), Object::Real(y)],
                ));
                operations.push(Operation::new(
                    "Tj",
                    vec![Object::String(
                        lines[line_idx].as_bytes().to_vec(),
                        lopdf::StringFormat::Literal,
                    )],
                ));
                y -= 12.0;
            }
            operations.push(Operation::new("ET", vec![]));

            let content = Content { operations };
            let content_bytes = content.encode().map_err(|e| e.to_string())?;
            let content_id = doc.add_object(Object::Stream(Stream::new(
                Dictionary::new(),
                content_bytes,
            )));

            let mut xobject_dict = Dictionary::new();
            xobject_dict.set("Im1", Object::Reference(img_id));

            let mut font_res = Dictionary::new();
            font_res.set("F1", Object::Reference(font_id));

            let mut resources = Dictionary::new();
            resources.set("XObject", Object::Dictionary(xobject_dict));
            resources.set("Font", Object::Dictionary(font_res));

            let mut page_dict = Dictionary::new();
            page_dict.set("Type", Object::Name("Page".into()));
            page_dict.set("Parent", Object::Reference(pages_id));
            page_dict.set(
                "MediaBox",
                Object::Array(vec![
                    Object::Real(0.0),
                    Object::Real(0.0),
                    Object::Real(pt_w),
                    Object::Real(pt_h),
                ]),
            );
            page_dict.set("Resources", Object::Dictionary(resources));
            page_dict.set("Contents", Object::Reference(content_id));

            let page_id = doc.add_object(Object::Dictionary(page_dict));
            page_refs.push(Object::Reference(page_id));
        }
    } else {
        // Fallback when no original image paths provided
        let page_width = 595.0f32;
        let page_height = 842.0f32;
        let margin = 50.0f32;
        let lines_per_page = 50;

        let chunks: Vec<&[&str]> = if lines.is_empty() {
            vec![&[]]
        } else {
            lines.chunks(lines_per_page).collect()
        };

        for chunk in chunks {
            let mut operations = vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec![Object::Name("F1".into()), Object::Real(11.0)]),
            ];

            let mut y = page_height - margin;
            for line in chunk {
                y -= 14.0;
                if y < margin {
                    break;
                }
                operations.push(Operation::new(
                    "Td",
                    vec![Object::Real(margin), Object::Real(y)],
                ));
                operations.push(Operation::new(
                    "Tj",
                    vec![Object::String(
                        line.as_bytes().to_vec(),
                        lopdf::StringFormat::Literal,
                    )],
                ));
            }
            operations.push(Operation::new("ET", vec![]));

            let content = Content { operations };
            let content_bytes = content.encode().map_err(|e| e.to_string())?;
            let content_id = doc.add_object(Object::Stream(Stream::new(
                Dictionary::new(),
                content_bytes,
            )));

            let mut font_res = Dictionary::new();
            font_res.set("F1", Object::Reference(font_id));
            let mut resources = Dictionary::new();
            resources.set("Font", Object::Dictionary(font_res));

            let mut page_dict = Dictionary::new();
            page_dict.set("Type", Object::Name("Page".into()));
            page_dict.set("Parent", Object::Reference(pages_id));
            page_dict.set(
                "MediaBox",
                Object::Array(vec![
                    Object::Real(0.0),
                    Object::Real(0.0),
                    Object::Real(page_width),
                    Object::Real(page_height),
                ]),
            );
            page_dict.set("Resources", Object::Dictionary(resources));
            page_dict.set("Contents", Object::Reference(content_id));

            let page_id = doc.add_object(Object::Dictionary(page_dict));
            page_refs.push(Object::Reference(page_id));
        }
    }

    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", Object::Name("Pages".into()));
    pages_dict.set("Count", Object::Integer(page_refs.len() as i64));
    pages_dict.set("Kids", Object::Array(page_refs));
    doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", Object::Name("Catalog".into()));
    catalog_dict.set("Pages", Object::Reference(pages_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog_dict));
    doc.trailer.set("Root", Object::Reference(catalog_id));

    let mut buf = Vec::new();
    doc.save_to(&mut buf).map_err(|e| e.to_string())?;
    std::fs::write(output_path, &buf).map_err(|e| format!("Failed to write file: {e}"))?;

    Ok(())
}
