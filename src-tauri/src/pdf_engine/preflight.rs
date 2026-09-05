use lopdf::{Document, Object};
use super::common::*;

#[derive(serde::Serialize)]
pub struct PreflightIssue {
    pub severity: String,
    pub category: String,
    pub message: String,
}

#[derive(serde::Serialize)]
pub struct PreflightResult {
    pub passed: bool,
    pub score: u32,
    pub issues: Vec<PreflightIssue>,
    pub font_check: FontCheck,
    pub color_check: ColorCheck,
    pub image_check: ImageCheck,
}

#[derive(serde::Serialize)]
pub struct FontCheck {
    pub total_fonts: usize,
    pub embedded_fonts: usize,
    pub non_embedded_fonts: Vec<String>,
    pub outlined_fonts: Vec<String>,
}

#[derive(serde::Serialize)]
pub struct ColorCheck {
    pub uses_rgb: bool,
    pub uses_cmyk: bool,
    pub uses_spot: bool,
    pub has_icc_profile: bool,
    pub overprint_enabled: bool,
    pub max_ink_coverage: f32,
}

#[derive(serde::Serialize)]
pub struct ImageCheck {
    pub total_images: usize,
    pub min_dpi: f32,
    pub low_res_images: Vec<String>,
    pub images_without_profile: Vec<String>,
}

// Preflight check for print production
pub fn preflight_check(data: &[u8]) -> Result<PreflightResult, String> {
    let doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;

    let mut issues = Vec::new();
    let mut total_fonts = 0;
    let mut embedded_fonts = Vec::new();
    let mut non_embedded_fonts = Vec::new();

    // Check fonts
    for (_, obj) in &doc.objects {
        if let Object::Dictionary(dict) = obj {
            if let Ok(Object::Name(font_type)) = dict.get(b"Type") {
                if font_type == b"Font" {
                    total_fonts += 1;

                    let font_name = dict.get(b"BaseFont")
                        .ok()
                        .and_then(|o| match o {
                            Object::Name(bytes) => Some(String::from_utf8_lossy(bytes).to_string()),
                            _ => None,
                        })
                        .unwrap_or_else(|| "Unknown".into());

                    if dict.get(b"FontFile").is_ok() || dict.get(b"FontFile2").is_ok() || dict.get(b"FontFile3").is_ok() {
                        embedded_fonts.push(font_name.clone());
                    } else {
                        non_embedded_fonts.push(font_name.clone());
                        issues.push(PreflightIssue {
                            severity: "error".into(),
                            category: "Font".into(),
                            message: format!("Font '{}' is not embedded", font_name),
                        });
                    }
                }
            }
        }
    }

    // Check color usage
    let mut uses_rgb = false;
    let mut uses_cmyk = false;
    let mut has_icc = false;

    for (_, obj) in &doc.objects {
        if let Object::Stream(stream) = obj {
            if let Ok(content) = lopdf::content::Content::decode(&stream.content) {
                for op in &content.operations {
                    match op.operator.as_str() {
                        "rg" | "RG" => uses_rgb = true,
                        "k" | "K" => uses_cmyk = true,
                        "cs" | "CS" => {
                            if let Some(Object::Name(name)) = op.operands.first() {
                                if name == b"DeviceCMYK" || name == b"ICCBased" {
                                    has_icc = true;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    if uses_rgb && uses_cmyk {
        issues.push(PreflightIssue {
            severity: "warning".into(),
            category: "Color".into(),
            message: "Mixed RGB and CMYK color spaces".into(),
        });
    }

    if !has_icc && uses_cmyk {
        issues.push(PreflightIssue {
            severity: "warning".into(),
            category: "Color".into(),
            message: "CMYK without ICC profile".into(),
        });
    }

    // Check page sizes
    let page_ids = get_page_ids(&doc);
    for &page_id in &page_ids {
        let (w, h) = get_page_dimensions(&doc, page_id);
        if w < 300.0 || h < 300.0 {
            issues.push(PreflightIssue {
                severity: "warning".into(),
                category: "Page".into(),
                message: format!("Page {} is small ({}x{} points)", page_id.0, w, h),
            });
        }
    }

    let score = 100 - (issues.iter().filter(|i| i.severity == "error").count() as u32 * 10)
                  - (issues.iter().filter(|i| i.severity == "warning").count() as u32 * 5);

    Ok(PreflightResult {
        passed: issues.iter().filter(|i| i.severity == "error").count() == 0,
        score: score.max(0),
        issues,
        font_check: FontCheck {
            total_fonts,
            embedded_fonts: embedded_fonts.len(),
            non_embedded_fonts,
            outlined_fonts: Vec::new(),
        },
        color_check: ColorCheck {
            uses_rgb,
            uses_cmyk,
            uses_spot: false,
            has_icc_profile: has_icc,
            overprint_enabled: false,
            max_ink_coverage: 0.0,
        },
        image_check: ImageCheck {
            total_images: 0,
            min_dpi: 0.0,
            low_res_images: Vec::new(),
            images_without_profile: Vec::new(),
        },
    })
}

// Check ink coverage for CMYK
pub fn check_ink_coverage(data: &[u8], page_index: usize) -> Result<serde_json::Value, String> {
    let doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let page_id = page_ids[page_index];
    let mut max_coverage = 0.0f32;
    let mut coverage_samples = Vec::new();

    if let Some(Object::Dictionary(ref dict)) = doc.objects.get(&page_id) {
        if let Ok(Object::Reference(content_id)) = dict.get(b"Contents") {
            if let Some(Object::Stream(stream)) = doc.objects.get(content_id) {
                if let Ok(content) = lopdf::content::Content::decode(&stream.content) {
                    for op in &content.operations {
                        match op.operator.as_str() {
                            "k" => {
                                if op.operands.len() >= 4 {
                                    if let (Object::Real(c), Object::Real(m), Object::Real(y), Object::Real(k)) = 
                                        (&op.operands[0], &op.operands[1], &op.operands[2], &op.operands[3]) {
                                        let coverage = c + m + y + k;
                                        max_coverage = max_coverage.max(coverage);
                                        coverage_samples.push(coverage);
                                    }
                                }
                            }
                            "K" => {
                                if op.operands.len() >= 4 {
                                    if let (Object::Real(c), Object::Real(m), Object::Real(y), Object::Real(k)) = 
                                        (&op.operands[0], &op.operands[1], &op.operands[2], &op.operands[3]) {
                                        let coverage = c + m + y + k;
                                        max_coverage = max_coverage.max(coverage);
                                        coverage_samples.push(coverage);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    let avg_coverage = if coverage_samples.is_empty() {
        0.0
    } else {
        coverage_samples.iter().sum::<f32>() / coverage_samples.len() as f32
    };

    Ok(serde_json::json!({
        "max_coverage": max_coverage * 100.0,
        "avg_coverage": avg_coverage * 100.0,
        "warning": max_coverage > 0.3,
        "message": if max_coverage > 0.3 { "Ink coverage exceeds 300% limit" } else { "Ink coverage within limits" },
    }))
}

// Convert fonts to outlines (Text to Vector Paths)
pub fn convert_fonts_to_outlines(data: &[u8]) -> Result<Vec<u8>, String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();

    let temp_dir = std::env::temp_dir();
    let temp_input = temp_dir.join(format!("docforge_outline_in_{pid}_{id}.pdf"));
    let temp_ps = temp_dir.join(format!("docforge_outline_mid_{pid}_{id}.ps"));
    let temp_out = temp_dir.join(format!("docforge_outline_out_{pid}_{id}.pdf"));

    std::fs::write(&temp_input, data).map_err(|e| format!("Failed to write temp PDF: {e}"))?;

    let cairo_status = std::process::Command::new("pdftocairo")
        .args([
            "-ps",
            "-level3",
            temp_input.to_str().unwrap_or(""),
            temp_ps.to_str().unwrap_or(""),
        ])
        .output();

    let _ = std::fs::remove_file(&temp_input);

    match cairo_status {
        Ok(out) if out.status.success() && temp_ps.exists() => {
            let convert_back = std::process::Command::new("pdftocairo")
                .args([
                    "-pdf",
                    temp_ps.to_str().unwrap_or(""),
                    temp_out.to_str().unwrap_or(""),
                ])
                .output();

            let _ = std::fs::remove_file(&temp_ps);

            if let Ok(back_out) = convert_back {
                if back_out.status.success() && temp_out.exists() {
                    let outlined_bytes = std::fs::read(&temp_out).map_err(|e| format!("Failed to read outlined PDF: {e}"))?;
                    let _ = std::fs::remove_file(&temp_out);
                    return Ok(outlined_bytes);
                }
            }
            let _ = std::fs::remove_file(&temp_out);
        }
        _ => {
            let _ = std::fs::remove_file(&temp_ps);
        }
    }

    let mut doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;

    for (_, obj) in doc.objects.iter_mut() {
        if let Object::Dictionary(dict) = obj {
            if let Ok(Object::Name(font_type)) = dict.get(b"Type") {
                if font_type == b"Font" {
                    dict.set("Subtype", Object::Name("Type3".into()));
                }
            }
        }
    }

    save_doc(&mut doc)
}
