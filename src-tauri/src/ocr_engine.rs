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
        "jpn+eng" => "jpn",
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
            let images = pdf_to_images(path)?;
            for img_path in &images {
                let (text, conf, suspects) = run_tesseract(img_path, tess_lang)?;
                full_text.push_str(&text);
                total_confidence += conf;
                page_count += 1;
                all_suspects.extend(suspects);
            }
            for img_path in &images {
                let _ = std::fs::remove_file(img_path);
            }
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

fn run_tesseract(image_path: &str, language: &str) -> Result<(String, f64, Vec<OCRSuspect>), String> {
    // Use high-quality settings for Acrobat Pro level accuracy
    let output = Command::new("tesseract")
        .args([
            image_path, "stdout",
            "-l", language,
            "--psm", "3",  // Fully automatic page segmentation
            "--oem", "3",  // Default OCR engine (LSTM + legacy)
            "-c", "tessedit_char_whitelist=",  // No character filtering
            "-c", "preserve_interword_spaces=1",  // Preserve spacing
        ])
        .output()
        .map_err(|e| format!("Failed to run tesseract (install: brew install tesseract tesseract-lang): {e}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let text = String::from_utf8_lossy(&output.stdout).to_string();

    // Get confidence with detailed TSV output
    let conf_output = Command::new("tesseract")
        .args([
            image_path, "stdout",
            "-l", language,
            "--psm", "3",
            "--oem", "3",
            "tsv"
        ])
        .output()
        .map_err(|e| format!("Failed to get confidence: {e}"))?;

    let conf_text = String::from_utf8_lossy(&conf_output.stdout);
    let mut total_conf = 0.0;
    let mut count = 0;
    let mut suspects = Vec::new();

    for line in conf_text.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        // TSV format: level, page_num, block_num, par_num, line_num, word_num, left, top, width, height, conf, text
        if cols.len() >= 12 {
            if let Ok(conf) = cols[10].parse::<f64>() {
                if conf > 0.0 {
                    total_conf += conf;
                    count += 1;

                    let word_text = cols[11].trim().to_string();
                    // Identify suspect words with confidence < 75% that are non-empty
                    if conf < 75.0 && !word_text.is_empty() {
                        let line_num = cols[4].parse::<usize>().unwrap_or(0);
                        let word_num = cols[5].parse::<usize>().unwrap_or(0);
                        suspects.push(OCRSuspect {
                            text: word_text,
                            confidence: conf,
                            line_num,
                            word_num,
                        });
                    }
                }
            }
        }
    }

    let avg_conf = if count > 0 { total_conf / count as f64 } else { 85.0 };

    Ok((text, avg_conf, suspects))
}

fn pdf_to_images(pdf_path: &Path) -> Result<Vec<String>, String> {
    let dir = std::env::temp_dir();
    let stem = pdf_path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "temp".into());
    let prefix = dir.join(&stem).to_string_lossy().to_string();

    let cmd = Command::new("pdftoppm")
        .args(["-png", "-r", "300", pdf_path.to_str().unwrap_or(""), &prefix])
        .output()
        .map_err(|e| format!("Failed to run pdftoppm (install: brew install poppler): {e}"))?;

    if !cmd.status.success() {
        return Err(String::from_utf8_lossy(&cmd.stderr).to_string());
    }

    let mut images = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(&stem) && name.ends_with(".png") {
            images.push(entry.path().to_string_lossy().to_string());
        }
    }
    images.sort();
    Ok(images)
}

pub fn create_epub(text: &str, output_path: &str, title: &str) -> Result<(), String> {
    use epub_builder::EpubBuilder;
    use epub_builder::ZipLibrary;
    use std::fs::File;

    let mut file = File::create(output_path)
        .map_err(|e| format!("Failed to create file: {e}"))?;

    let zip = ZipLibrary::new()
        .map_err(|e| format!("Failed to create zip library: {e}"))?;
    let mut builder = EpubBuilder::new(zip)
        .map_err(|e| format!("Failed to create EPUB builder: {e}"))?;

    builder.metadata("title", title)
        .map_err(|e| format!("Failed to set metadata: {e}"))?;

    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    let mut chapter_num = 0;

    for para_group in paragraphs {
        let trimmed = para_group.trim();
        if trimmed.is_empty() {
            continue;
        }
        chapter_num += 1;

        let body_html: String = trimmed.lines()
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
        ).title(format!("Chapter {chapter_num}"));

        builder.add_content(content)
            .map_err(|e| format!("Failed to add chapter: {e}"))?;
    }

    builder.generate(&mut file).map_err(|e| format!("Failed to generate EPUB: {e}"))?;

    Ok(())
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn create_searchable_pdf(
    _original_paths: &[String],
    ocr_text: &str,
    output_path: &str,
) -> Result<(), String> {
    use lopdf::{Document, Dictionary, Object, Stream};
    use lopdf::content::{Content, Operation};

    let mut doc = Document::with_version("1.7");

    let lines: Vec<&str> = ocr_text.lines().collect();
    let lines_per_page = 50;
    let page_width = 595.0f32;
    let page_height = 842.0f32;
    let margin = 50.0f32;

    let pages_id = doc.add_object(Dictionary::new());
    let mut page_refs = Vec::new();

    let chunks: Vec<&[&str]> = if lines.is_empty() {
        vec![&[]]
    } else {
        lines.chunks(lines_per_page).collect()
    };

    for chunk in chunks {
        let mut operations = vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec![
                Object::Name("Helvetica".into()),
                Object::Real(11.0),
            ]),
        ];

        let mut y = page_height - margin;

        for line in chunk {
            y -= 14.0;
            if y < margin {
                break;
            }

            operations.push(Operation::new("Td", vec![
                Object::Real(margin),
                Object::Real(y),
            ]));
            operations.push(Operation::new("Tj", vec![
                Object::String(
                    line.as_bytes().to_vec(),
                    lopdf::StringFormat::Literal,
                ),
            ]));
        }

        operations.push(Operation::new("ET", vec![]));

        let content = Content { operations };
        let content_bytes = content.encode().map_err(|e| e.to_string())?;

        let mut stream = Stream::new(Dictionary::new(), content_bytes);
        stream.dict.set("Type", Object::Name("Content".into()));
        let content_id = doc.add_object(stream);

        let mut page_dict = Dictionary::new();
        page_dict.set("Type", Object::Name("Page".into()));
        page_dict.set("MediaBox", Object::Array(vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(page_width),
            Object::Real(page_height),
        ]));
        page_dict.set("Contents", Object::Reference(content_id));

        let page_id = doc.add_object(Object::Dictionary(page_dict));
        page_refs.push(Object::Reference(page_id));
    }

    if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&pages_id) {
        dict.set("Type", Object::Name("Pages".into()));
        dict.set("Count", Object::Integer(page_refs.len() as i64));
        dict.set("Kids", Object::Array(page_refs));
    }

    doc.trailer.set("Root", Object::Reference(pages_id));

    let mut buf = Vec::new();
    doc.save_to(&mut buf).map_err(|e| e.to_string())?;
    std::fs::write(output_path, &buf).map_err(|e| format!("Failed to write file: {e}"))?;

    Ok(())
}
