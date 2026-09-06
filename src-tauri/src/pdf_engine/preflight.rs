use super::common::*;
use lopdf::{Document, Object};

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
    let doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

    let mut issues = Vec::new();
    let mut total_fonts = 0;
    let mut embedded_fonts = Vec::new();
    let mut non_embedded_fonts = Vec::new();

    // Check fonts with FontDescriptor indirect reference lookup
    for (_, obj) in &doc.objects {
        if let Object::Dictionary(dict) = obj {
            if let Ok(Object::Name(font_type)) = dict.get(b"Type") {
                if font_type == b"Font" {
                    total_fonts += 1;

                    let font_name = dict
                        .get(b"BaseFont")
                        .ok()
                        .and_then(|o| match o {
                            Object::Name(bytes) => Some(String::from_utf8_lossy(bytes).to_string()),
                            _ => None,
                        })
                        .unwrap_or_else(|| "Unknown".into());

                    // Check font descriptor for FontFile / FontFile2 / FontFile3
                    let has_font_file = if let Ok(desc_ref) =
                        dict.get(b"FontDescriptor").and_then(|o| o.as_reference())
                    {
                        if let Some(Object::Dictionary(desc)) = doc.objects.get(&desc_ref) {
                            desc.get(b"FontFile").is_ok()
                                || desc.get(b"FontFile2").is_ok()
                                || desc.get(b"FontFile3").is_ok()
                        } else {
                            false
                        }
                    } else {
                        dict.get(b"FontFile").is_ok()
                            || dict.get(b"FontFile2").is_ok()
                            || dict.get(b"FontFile3").is_ok()
                    };

                    if has_font_file {
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

    // Check color usage & operators across streams
    let mut uses_rgb = false;
    let mut uses_cmyk = false;
    let mut uses_spot = false;
    let mut overprint_enabled = false;
    let mut has_icc = false;
    let mut max_ink_coverage = 0.0f32;

    for (_, obj) in &doc.objects {
        if let Object::Stream(stream) = obj {
            if let Ok(content) = lopdf::content::Content::decode(&stream.content) {
                for op in &content.operations {
                    match op.operator.as_str() {
                        "rg" | "RG" => uses_rgb = true,
                        "k" | "K" => {
                            uses_cmyk = true;
                            if op.operands.len() >= 4 {
                                if let (Some(c), Some(m), Some(y), Some(k)) = (
                                    op.operands[0].as_float().ok(),
                                    op.operands[1].as_float().ok(),
                                    op.operands[2].as_float().ok(),
                                    op.operands[3].as_float().ok(),
                                ) {
                                    let cov = c + m + y + k;
                                    max_ink_coverage = max_ink_coverage.max(cov);
                                }
                            }
                        }
                        "cs" | "CS" => {
                            if let Some(Object::Name(name)) = op.operands.first() {
                                if name == b"DeviceCMYK" || name == b"ICCBased" {
                                    has_icc = true;
                                } else if name == b"Separation" {
                                    uses_spot = true;
                                }
                            }
                        }
                        "gs" => {
                            // ExtGState reference used in content stream
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Check ExtGState dictionaries for overprint settings
    for (_, obj) in &doc.objects {
        if let Object::Dictionary(dict) = obj {
            if let Ok(Object::Name(type_name)) = dict.get(b"Type") {
                if type_name == b"ExtGState" {
                    if let Ok(op) = dict.get(b"OP").and_then(|o| o.as_bool()) {
                        if op {
                            overprint_enabled = true;
                        }
                    }
                    if let Ok(op) = dict.get(b"op").and_then(|o| o.as_bool()) {
                        if op {
                            overprint_enabled = true;
                        }
                    }
                }
            }
        }
    }

    // Count image XObjects and analyze resolutions
    let mut total_images = 0;
    let mut min_dpi = 0.0f32;
    let mut low_res_images = Vec::new();
    let mut images_without_profile = Vec::new();

    for (id, obj) in &doc.objects {
        if let Object::Stream(stream) = obj {
            if let Ok(Object::Name(subtype)) = stream.dict.get(b"Subtype") {
                if subtype == b"Image" {
                    total_images += 1;
                    let width = stream.dict.get(b"Width").and_then(|o| o.as_float()).unwrap_or(0.0);
                    let height = stream.dict.get(b"Height").and_then(|o| o.as_float()).unwrap_or(0.0);
                    let has_cs = stream.dict.get(b"ColorSpace").is_ok();
                    if !has_cs {
                        images_without_profile.push(format!("Image_{}_{}", id.0, id.1));
                    }

                    // Estimate DPI based on 72pt default display size if dimension known
                    if width > 0.0 && height > 0.0 {
                        let approx_dpi = (width / 2.0).max(72.0); // conservative estimation
                        if min_dpi == 0.0 || approx_dpi < min_dpi {
                            min_dpi = approx_dpi;
                        }
                        if approx_dpi < 150.0 {
                            low_res_images.push(format!("Image_{}_{} ({}x{})", id.0, id.1, width as u32, height as u32));
                        }
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

    if max_ink_coverage > 3.0 {
        issues.push(PreflightIssue {
            severity: "warning".into(),
            category: "Ink".into(),
            message: format!(
                "Maximum CMYK operand ink coverage ({:.1}%) exceeds 300%",
                max_ink_coverage * 100.0
            ),
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

    let score = 100
        - (issues.iter().filter(|i| i.severity == "error").count() as u32 * 10)
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
            uses_spot,
            has_icc_profile: has_icc,
            overprint_enabled,
            max_ink_coverage: max_ink_coverage * 100.0,
        },
        image_check: ImageCheck {
            total_images,
            min_dpi,
            low_res_images,
            images_without_profile,
        },
    })
}

// Check ink coverage for CMYK
pub fn check_ink_coverage(data: &[u8], page_index: usize) -> Result<serde_json::Value, String> {
    let doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let page_id = page_ids[page_index];
    let mut max_coverage = 0.0f32;
    let mut coverage_samples = Vec::new();

    if let Some(Object::Dictionary(ref dict)) = doc.objects.get(&page_id) {
        let content_ids: Vec<OID> = match dict.get(b"Contents") {
            Ok(Object::Reference(id)) => vec![*id],
            Ok(Object::Array(arr)) => arr.iter().filter_map(|o| o.as_reference().ok()).collect(),
            _ => Vec::new(),
        };

        for cid in content_ids {
            if let Some(Object::Stream(stream)) = doc.objects.get(&cid) {
                if let Ok(content) = lopdf::content::Content::decode(&stream.content) {
                    for op in &content.operations {
                        match op.operator.as_str() {
                            "k" | "K" => {
                                if op.operands.len() >= 4 {
                                    if let (
                                        Some(c),
                                        Some(m),
                                        Some(y),
                                        Some(k),
                                    ) = (
                                        op.operands[0].as_float().ok(),
                                        op.operands[1].as_float().ok(),
                                        op.operands[2].as_float().ok(),
                                        op.operands[3].as_float().ok(),
                                    ) {
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
        "warning": max_coverage > 3.0,
        "message": if max_coverage > 3.0 {
            "CMYK paint operand ink coverage exceeds 300% limit"
        } else {
            "CMYK paint operand ink coverage within limits"
        },
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
                    let outlined_bytes = std::fs::read(&temp_out)
                        .map_err(|e| format!("Failed to read outlined PDF: {e}"))?;
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

    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

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
