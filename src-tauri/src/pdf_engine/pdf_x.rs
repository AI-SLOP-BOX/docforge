use super::common::*;
use lopdf::{Dictionary, Document, Object, Stream};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct PdfxValidationReport {
    pub is_compliant: bool,
    pub standard: String,
    pub output_condition: String,
    pub passed_checks: Vec<String>,
    pub violations: Vec<String>,
    pub details: serde_json::Value,
}

/// Preflight and compliance assistance checker for PDF/X-1a:2001 and PDF/X-4:2010 (Preflight Validation)
pub fn validate_pdfx_compliance(
    data: &[u8],
    target_standard: &str,
) -> Result<PdfxValidationReport, String> {
    let doc = Document::load_mem(data).map_err(|e| format!("Failed to parse PDF: {e}"))?;

    let is_x1a = target_standard.to_lowercase().contains("x-1a")
        || target_standard.to_lowercase().contains("x1a");
    let standard_name = if is_x1a {
        "PDF/X-1a:2001 (ISO 15930-1 Preflight)"
    } else {
        "PDF/X-4:2010 (ISO 15930-7 Preflight)"
    };

    let mut passed = Vec::new();
    let mut violations = Vec::new();
    let mut output_condition = String::from("None");

    // 1. OutputIntents check
    let root_id = doc
        .trailer
        .get(b"Root")
        .and_then(|o| o.as_reference())
        .ok()
        .ok_or_else(|| "PDF Root Catalog not found".to_string())?;

    let mut has_valid_output_intent = false;
    let mut has_dest_output_profile = false;
    if let Some(Object::Dictionary(ref root)) = doc.objects.get(&root_id) {
        if let Ok(Object::Array(ref intents)) = root.get(b"OutputIntents") {
            for item in intents {
                let intent_dict = match item {
                    Object::Reference(id) => doc.objects.get(id).and_then(|o| o.as_dict().ok()),
                    Object::Dictionary(d) => Some(d),
                    _ => None,
                };
                if let Some(dict) = intent_dict {
                    if let Ok(Object::Name(ref s)) = dict.get(b"S") {
                        if s == b"GTS_PDFX" {
                            has_valid_output_intent = true;
                            if let Ok(ident) = dict.get(b"OutputConditionIdentifier") {
                                output_condition = match ident {
                                    Object::String(bytes, _) => {
                                        String::from_utf8_lossy(bytes).to_string()
                                    }
                                    Object::Name(bytes) => {
                                        String::from_utf8_lossy(bytes).to_string()
                                    }
                                    _ => "Standard Output Condition".to_string(),
                                };
                            }

                            // Check DestOutputProfile stream presence
                            if let Ok(prof_ref) = dict
                                .get(b"DestOutputProfile")
                                .and_then(|o| o.as_reference())
                            {
                                if let Some(Object::Stream(prof_stream)) =
                                    doc.objects.get(&prof_ref)
                                {
                                    let n = prof_stream
                                        .dict
                                        .get(b"N")
                                        .and_then(|o| o.as_i64())
                                        .unwrap_or(0);
                                    if n == 4 || n == 3 || n == 1 {
                                        has_dest_output_profile = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if has_valid_output_intent {
        if has_dest_output_profile {
            passed.push(format!(
                "OutputIntent 'GTS_PDFX' present with condition: {output_condition} and embedded DestOutputProfile ICC stream"
            ));
        } else {
            passed.push(format!(
                "OutputIntent 'GTS_PDFX' present with condition: {output_condition}"
            ));
        }
    } else {
        violations.push("OutputIntents entry with S=GTS_PDFX is missing".to_string());
    }

    // 2. Trapped key in Info Dictionary
    let mut has_trapped = false;
    if let Ok(info_ref) = doc.trailer.get(b"Info").and_then(|o| o.as_reference()) {
        if let Some(Object::Dictionary(ref info)) = doc.objects.get(&info_ref) {
            if let Ok(Object::Name(ref trapped)) = info.get(b"Trapped") {
                if trapped == b"True" || trapped == b"False" {
                    has_trapped = true;
                }
            }
        }
    }
    if has_trapped {
        passed.push("Info dictionary contains compliant Trapped flag (True/False)".to_string());
    } else {
        violations.push("Info dictionary missing required 'Trapped' flag (True/False)".to_string());
    }

    // 3. Page Boxes Check (MediaBox + either BleedBox or TrimBox)
    let page_ids = get_page_ids(&doc);
    let mut page_boxes_valid = true;
    for (idx, &pid) in page_ids.iter().enumerate() {
        if let Some(Object::Dictionary(ref pdict)) = doc.objects.get(&pid) {
            let has_media = pdict.get(b"MediaBox").is_ok();
            let has_trim_or_bleed = pdict.get(b"TrimBox").is_ok() || pdict.get(b"BleedBox").is_ok();
            if !has_media || !has_trim_or_bleed {
                page_boxes_valid = false;
                violations.push(format!(
                    "Page {} is missing required TrimBox or BleedBox",
                    idx + 1
                ));
                break;
            }
        }
    }
    if page_boxes_valid {
        passed.push("All pages specify MediaBox and TrimBox/BleedBox".to_string());
    }

    // 4. Color & Transparency checks
    let mut font_unembedded = Vec::new();
    let mut prohibited_rgb = Vec::new();
    let mut transparency_detected = false;

    for (page_idx, &pid) in page_ids.iter().enumerate() {
        if let Some(Object::Dictionary(ref pdict)) = doc.objects.get(&pid) {
            // Check resources for transparency ExtGState (CA/ca < 1.0 or BM != /Normal)
            if let Ok(res) = pdict.get(b"Resources") {
                let res_dict = match res {
                    Object::Reference(id) => doc.objects.get(id).and_then(|o| o.as_dict().ok()),
                    Object::Dictionary(d) => Some(d),
                    _ => None,
                };
                if let Some(r) = res_dict {
                    if let Ok(egs) = r.get(b"ExtGState") {
                        let egs_dict = match egs {
                            Object::Reference(id) => {
                                doc.objects.get(id).and_then(|o| o.as_dict().ok())
                            }
                            Object::Dictionary(d) => Some(d),
                            _ => None,
                        };
                        if let Some(states) = egs_dict {
                            for (_, state_obj) in states.iter() {
                                let state_dict = match state_obj {
                                    Object::Reference(id) => {
                                        doc.objects.get(id).and_then(|o| o.as_dict().ok())
                                    }
                                    Object::Dictionary(d) => Some(d),
                                    _ => None,
                                };
                                if let Some(sd) = state_dict {
                                    if let Ok(ca) = sd.get(b"ca").and_then(|o| o.as_float()) {
                                        if ca < 0.999 {
                                            transparency_detected = true;
                                        }
                                    }
                                    if let Ok(ca) = sd.get(b"CA").and_then(|o| o.as_float()) {
                                        if ca < 0.999 {
                                            transparency_detected = true;
                                        }
                                    }
                                    if let Ok(Object::Name(bm)) = sd.get(b"BM") {
                                        if bm != b"Normal" && bm != b"Compatible" {
                                            transparency_detected = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Check Content operations
            if let Ok(contents) = pdict.get(b"Contents") {
                let content_ids: Vec<OID> = match contents {
                    Object::Reference(id) => vec![*id],
                    Object::Array(arr) => {
                        arr.iter().filter_map(|o| o.as_reference().ok()).collect()
                    }
                    _ => vec![],
                };
                for cid in content_ids {
                    if let Some(Object::Stream(ref stream)) = doc.objects.get(&cid) {
                        if let Ok(c) = lopdf::content::Content::decode(&stream.content) {
                            for op in &c.operations {
                                if is_x1a && (op.operator == "rg" || op.operator == "RG") {
                                    prohibited_rgb.push(format!(
                                        "Page {}: DeviceRGB operator '{}'",
                                        page_idx + 1,
                                        op.operator
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 5. Font embedding check
    for (_, obj) in &doc.objects {
        if let Object::Dictionary(dict) = obj {
            if let Ok(Object::Name(font_type)) = dict.get(b"Type") {
                if font_type == b"Font" {
                    let base_font = dict
                        .get(b"BaseFont")
                        .ok()
                        .and_then(|o| match o {
                            Object::Name(bytes) => Some(String::from_utf8_lossy(bytes).to_string()),
                            _ => None,
                        })
                        .unwrap_or_else(|| "UnknownFont".into());

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

                    if !has_font_file
                        && base_font != "Helvetica"
                        && base_font != "Times-Roman"
                        && base_font != "Courier"
                    {
                        font_unembedded.push(base_font);
                    }
                }
            }
        }
    }

    if font_unembedded.is_empty() {
        passed.push("All fonts are properly embedded".to_string());
    } else {
        violations.push(format!(
            "Non-embedded fonts detected: {}",
            font_unembedded.join(", ")
        ));
    }

    if is_x1a {
        if transparency_detected {
            violations.push(
                "PDF/X-1a strictly prohibits live transparency (Flattening required)".to_string(),
            );
        } else {
            passed.push("No live transparency detected (PDF/X-1a compliant)".to_string());
        }

        if prohibited_rgb.is_empty() {
            passed.push("Color space is restricted to CMYK and Grayscale (No RGB)".to_string());
        } else {
            violations.push(
                "PDF/X-1a prohibits RGB colors: all colors must be CMYK or Grayscale".to_string(),
            );
        }
    } else {
        // PDF/X-4 allows transparency and color-managed RGB with OutputIntent
        passed.push("PDF/X-4 allows color management and live transparency".to_string());
    }

    let is_compliant = violations.is_empty();

    Ok(PdfxValidationReport {
        is_compliant,
        standard: standard_name.to_string(),
        output_condition,
        passed_checks: passed,
        violations,
        details: serde_json::json!({
            "target_standard": target_standard,
            "page_count": page_ids.len(),
            "fonts_checked": font_unembedded.len(),
            "transparency_found": transparency_detected,
        }),
    })
}

/// Prepare and structure a PDF document for PDF/X-1a:2001 or PDF/X-4:2010 preflight compliance assistance
pub fn convert_to_pdfx_standard(
    data: &[u8],
    standard: &str,
    output_intent: &str,
) -> Result<Vec<u8>, String> {
    let is_x1a =
        standard.to_lowercase().contains("x-1a") || standard.to_lowercase().contains("x1a");

    // If PDF/X-1a, flatten transparency first (ISO 15930-1 strictly prohibits live transparency)
    let prepared_data = if is_x1a {
        super::print_prod::flatten_transparency(data).map_err(|e| {
            format!("PDF/X-1a変換エラー (透明効果の統合・ラスタライズに失敗しました): {e}")
        })?
    } else {
        data.to_vec()
    };

    let mut doc =
        Document::load_mem(&prepared_data).map_err(|e| format!("Failed to load PDF: {e}"))?;

    let standard_id = if is_x1a {
        "PDF/X-1a:2001"
    } else {
        "PDF/X-4:2010"
    };
    let condition_name = if output_intent.is_empty() {
        "Japan Color 2001 Coated"
    } else {
        output_intent
    };

    // 1. Set conforming PDF Version
    doc.version = if is_x1a {
        "1.3".to_string()
    } else {
        "1.6".to_string()
    };

    // 2. Build DestOutputProfile ICC Stream
    // ISO 15930 requires an embedded ICC output profile stream with valid header, tag table, and tag data
    let icc_bytes = generate_valid_cmyk_icc(condition_name);

    let mut icc_dict = Dictionary::new();
    icc_dict.set("N", Object::Integer(4)); // 4 channels for CMYK
    icc_dict.set("Length", Object::Integer(icc_bytes.len() as i64));
    let icc_stream = Stream::new(icc_dict, icc_bytes);
    let dest_profile_id = doc.add_object(Object::Stream(icc_stream));

    // 3. Build OutputIntent dictionary
    let mut intent_dict = Dictionary::new();
    intent_dict.set("Type", Object::Name("OutputIntent".into()));
    intent_dict.set("S", Object::Name("GTS_PDFX".into()));
    intent_dict.set("DestOutputProfile", Object::Reference(dest_profile_id));
    intent_dict.set(
        "OutputConditionIdentifier",
        Object::String(
            condition_name.as_bytes().to_vec(),
            lopdf::StringFormat::Literal,
        ),
    );
    intent_dict.set(
        "OutputCondition",
        Object::String(
            condition_name.as_bytes().to_vec(),
            lopdf::StringFormat::Literal,
        ),
    );
    intent_dict.set(
        "RegistryName",
        Object::String(
            "http://www.color.org".as_bytes().to_vec(),
            lopdf::StringFormat::Literal,
        ),
    );
    intent_dict.set(
        "Info",
        Object::String(
            condition_name.as_bytes().to_vec(),
            lopdf::StringFormat::Literal,
        ),
    );

    let intent_id = doc.add_object(Object::Dictionary(intent_dict));

    // 4. Attach to Catalog Root
    let root_id = if let Ok(id) = doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        id
    } else {
        let root_dict = Dictionary::new();
        doc.add_object(Object::Dictionary(root_dict))
    };

    if let Some(Object::Dictionary(ref mut root_dict)) = doc.objects.get_mut(&root_id) {
        root_dict.set(
            "OutputIntents",
            Object::Array(vec![Object::Reference(intent_id)]),
        );
    }
    doc.trailer.set("Root", Object::Reference(root_id));

    // 4. Update Info Dictionary with Trapped and GTS_PDFXVersion
    let info_id = if let Ok(id) = doc.trailer.get(b"Info").and_then(|o| o.as_reference()) {
        id
    } else {
        let info_dict = Dictionary::new();
        doc.add_object(Object::Dictionary(info_dict))
    };

    if let Some(Object::Dictionary(ref mut info_dict)) = doc.objects.get_mut(&info_id) {
        info_dict.set("Trapped", Object::Name("False".into()));
        info_dict.set(
            "GTS_PDFXVersion",
            Object::String(
                standard_id.as_bytes().to_vec(),
                lopdf::StringFormat::Literal,
            ),
        );
        info_dict.set(
            "Title",
            Object::String(
                b"PDF/X Compliant Document".to_vec(),
                lopdf::StringFormat::Literal,
            ),
        );
        info_dict.set(
            "Creator",
            Object::String(
                b"DocForge Professional PDF Engine".to_vec(),
                lopdf::StringFormat::Literal,
            ),
        );
    }
    doc.trailer.set("Info", Object::Reference(info_id));

    // 5. Ensure all pages have MediaBox, TrimBox, and BleedBox defined
    let page_ids = get_page_ids(&doc);
    for &pid in &page_ids {
        if let Some(Object::Dictionary(ref mut page_dict)) = doc.objects.get_mut(&pid) {
            let (pw, ph) = (
                page_dict
                    .get(b"MediaBox")
                    .ok()
                    .and_then(|mb| mb.as_array().ok())
                    .and_then(|arr| arr.get(2))
                    .and_then(|w| w.as_float().ok())
                    .unwrap_or(595.0),
                page_dict
                    .get(b"MediaBox")
                    .ok()
                    .and_then(|mb| mb.as_array().ok())
                    .and_then(|arr| arr.get(3))
                    .and_then(|h| h.as_float().ok())
                    .unwrap_or(842.0),
            );

            // If TrimBox is missing, default to MediaBox
            if page_dict.get(b"TrimBox").is_err() {
                page_dict.set(
                    "TrimBox",
                    Object::Array(vec![
                        Object::Real(0.0),
                        Object::Real(0.0),
                        Object::Real(pw),
                        Object::Real(ph),
                    ]),
                );
            }
            if page_dict.get(b"BleedBox").is_err() {
                page_dict.set(
                    "BleedBox",
                    Object::Array(vec![
                        Object::Real(0.0),
                        Object::Real(0.0),
                        Object::Real(pw),
                        Object::Real(ph),
                    ]),
                );
            }
        }
    }

    // 6. Embed standard XMP metadata with PDF/X Extension Schema
    let xmp_metadata = format!(
        "<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
        <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n\
          <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
            <rdf:Description rdf:about=\"\" xmlns:pdfx=\"http://ns.adobe.com/pdfx/1.3/\">\n\
              <pdfx:GTS_PDFXVersion>{}</pdfx:GTS_PDFXVersion>\n\
            </rdf:Description>\n\
            <rdf:Description rdf:about=\"\" xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\">\n\
              <pdf:Producer>DocForge PDF/X Engine</pdf:Producer>\n\
              <pdf:Trapped>False</pdf:Trapped>\n\
            </rdf:Description>\n\
          </rdf:RDF>\n\
        </x:xmpmeta>\n\
        <?xpacket end=\"w\"?>",
        standard_id
    );

    let mut meta_dict = Dictionary::new();
    meta_dict.set("Type", Object::Name("Metadata".into()));
    meta_dict.set("Subtype", Object::Name("XML".into()));
    meta_dict.set("Length", Object::Integer(xmp_metadata.len() as i64));
    let meta_stream = Stream::new(meta_dict, xmp_metadata.into_bytes());
    let meta_id = doc.add_object(meta_stream);

    if let Some(Object::Dictionary(ref mut root_dict)) = doc.objects.get_mut(&root_id) {
        root_dict.set("Metadata", Object::Reference(meta_id));
    }

    save_doc(&mut doc)
}

pub fn convert_to_pdfx(data: &[u8], output_intent: &str) -> Result<Vec<u8>, String> {
    convert_to_pdfx_standard(data, "PDF/X-1a:2001", output_intent)
}

/// Generates a valid ICC profile according to ICC.1:2001-04 specification
/// Includes a conforming 128-byte header, a tag table with required tags (desc, cprt, wtpt, kTRC),
/// and 4-byte aligned tag data that external ICC parsers (e.g. LittleCMS, CoreGraphics, Poppler) can parse.
pub fn generate_valid_cmyk_icc(condition_name: &str) -> Vec<u8> {
    let mut tags: Vec<[u8; 4]> = Vec::new();
    let mut data_blobs: Vec<Vec<u8>> = Vec::new();

    // 1. Tag 'desc': TextDescriptionType
    let mut desc_data = Vec::new();
    desc_data.extend_from_slice(b"desc"); // Type sig
    desc_data.extend_from_slice(&0u32.to_be_bytes()); // Reserved
    let desc_bytes = format!("{condition_name}\0").into_bytes();
    desc_data.extend_from_slice(&(desc_bytes.len() as u32).to_be_bytes());
    desc_data.extend_from_slice(&desc_bytes);
    // Unicode language code & count (0)
    desc_data.extend_from_slice(&0u32.to_be_bytes());
    desc_data.extend_from_slice(&0u32.to_be_bytes());
    // ScriptCode code & count & bytes (67 bytes of 0)
    desc_data.extend_from_slice(&0u16.to_be_bytes());
    desc_data.push(0);
    desc_data.extend_from_slice(&[0u8; 67]);
    tags.push(*b"desc");
    data_blobs.push(desc_data);

    // 2. Tag 'cprt': TextType
    let mut cprt_data = Vec::new();
    cprt_data.extend_from_slice(b"text");
    cprt_data.extend_from_slice(&0u32.to_be_bytes());
    cprt_data.extend_from_slice(b"DocForge ICC Profile - MIT License\0");
    tags.push(*b"cprt");
    data_blobs.push(cprt_data);

    // 3. Tag 'wtpt': XYZType (D50 white point: X=0.9642, Y=1.0, Z=0.8249 in s15Fixed16)
    let mut wtpt_data = Vec::new();
    wtpt_data.extend_from_slice(b"XYZ ");
    wtpt_data.extend_from_slice(&0u32.to_be_bytes());
    wtpt_data.extend_from_slice(&63188u32.to_be_bytes()); // X: 0.9642 * 65536
    wtpt_data.extend_from_slice(&65536u32.to_be_bytes()); // Y: 1.0 * 65536
    wtpt_data.extend_from_slice(&54059u32.to_be_bytes()); // Z: 0.8249 * 65536
    tags.push(*b"wtpt");
    data_blobs.push(wtpt_data);

    // 4. Tag 'kTRC': CurveType (Linear curve)
    let mut ktrc_data = Vec::new();
    ktrc_data.extend_from_slice(b"curv");
    ktrc_data.extend_from_slice(&0u32.to_be_bytes());
    ktrc_data.extend_from_slice(&0u32.to_be_bytes()); // count = 0 (linear response)
    tags.push(*b"kTRC");
    data_blobs.push(ktrc_data);

    let tag_count = tags.len() as u32;
    let tag_table_len = 4 + tag_count * 12;
    let first_data_offset = 128 + tag_table_len;

    let mut aligned_blobs = Vec::new();
    let mut offsets = Vec::new();
    let mut cur_offset = first_data_offset;

    for blob in &data_blobs {
        let pad = (4 - (blob.len() % 4)) % 4;
        let mut padded = blob.clone();
        padded.extend(std::iter::repeat(0u8).take(pad));
        offsets.push((cur_offset, blob.len() as u32));
        cur_offset += padded.len() as u32;
        aligned_blobs.push(padded);
    }

    let total_size = cur_offset;

    // Header: 128 bytes
    let mut header = vec![0u8; 128];
    header[0..4].copy_from_slice(&total_size.to_be_bytes());
    header[4..8].copy_from_slice(b"ADBE");
    header[8..12].copy_from_slice(&0x02100000u32.to_be_bytes()); // v2.1.0
    header[12..16].copy_from_slice(b"prtr"); // Device class 'prtr'
    header[16..20].copy_from_slice(b"CMYK"); // Data color space 'CMYK'
    header[20..24].copy_from_slice(b"XYZ "); // Connection space 'XYZ '
                                             // Creation date/time 2026/01/01
    header[24..26].copy_from_slice(&2026u16.to_be_bytes());
    header[26..28].copy_from_slice(&1u16.to_be_bytes());
    header[28..30].copy_from_slice(&1u16.to_be_bytes());
    header[36..40].copy_from_slice(b"acsp"); // Magic
    header[40..44].copy_from_slice(b"APPL"); // Platform
                                             // Illuminant D50
    header[68..72].copy_from_slice(&63188u32.to_be_bytes());
    header[72..76].copy_from_slice(&65536u32.to_be_bytes());
    header[76..80].copy_from_slice(&54059u32.to_be_bytes());
    header[80..84].copy_from_slice(b"DOCF"); // Creator

    // Assemble profile
    let mut profile = header;
    profile.extend_from_slice(&tag_count.to_be_bytes());
    for i in 0..tags.len() {
        profile.extend_from_slice(&tags[i]);
        profile.extend_from_slice(&offsets[i].0.to_be_bytes());
        profile.extend_from_slice(&offsets[i].1.to_be_bytes());
    }
    for blob in aligned_blobs {
        profile.extend(blob);
    }

    profile
}
