use lopdf::{Document, Object, Dictionary};
use super::*;
use super::common::*;

// ===== ADVANCED FORM =====

pub fn add_form_field(
    data: &[u8],
    page_index: usize,
    field_name: &str,
    field_type: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    default_value: &str,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    // Create field dictionary
    let mut field_dict = Dictionary::new();
    field_dict.set("Type", Object::Name("Annot".into()));
    field_dict.set("Subtype", Object::Name("Widget".into()));
    field_dict.set("FT", Object::Name(field_type.as_bytes().to_vec()));
    field_dict.set("T", Object::String(field_name.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    field_dict.set("V", Object::String(default_value.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    field_dict.set("Rect", Object::Array(vec![
        Object::Real(x as f32), Object::Real(y as f32),
        Object::Real((x + width) as f32), Object::Real((y + height) as f32),
    ]));
    field_dict.set("F", Object::Integer(4));
    field_dict.set("DA", Object::String(b"/Helv 12 Tf 0 0 0 rg".to_vec(), lopdf::StringFormat::Literal));

    let field_id = doc.add_object(Object::Dictionary(field_dict));

    // Add to page annotations
    let page_id = page_ids[page_index];
    if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
        let mut annots = match dict.get(b"Annots") {
            Ok(Object::Array(a)) => a.clone(),
            _ => Vec::new(),
        };
        annots.push(Object::Reference(field_id));
        dict.set("Annots", Object::Array(annots));
    }

    // Update AcroForm
    let root_id = doc.trailer.get(b"Root")
        .and_then(|o| o.as_reference())
        .ok()
        .ok_or("No root")?;

    // Get acroform_id first
    let acroform_id = if let Some(root) = doc.objects.get(&root_id) {
        if let Ok(root_dict) = root.as_dict() {
            if let Ok(Object::Reference(id)) = root_dict.get(b"AcroForm") {
                Some(*id)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    if let Some(acroform_id) = acroform_id {
        if let Some(Object::Dictionary(ref mut acroform)) = doc.objects.get_mut(&acroform_id) {
            let mut fields = match acroform.get(b"Fields") {
                Ok(Object::Array(f)) => f.clone(),
                _ => Vec::new(),
            };
            fields.push(Object::Reference(field_id));
            acroform.set("Fields", Object::Array(fields));
        }
    }

    save_doc(&mut doc)
}

pub fn add_calculated_field(
    data: &[u8],
    page_index: usize,
    field_name: &str,
    formula: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    // Create field with JavaScript calculation
    let mut field_dict = Dictionary::new();
    field_dict.set("Type", Object::Name("Annot".into()));
    field_dict.set("Subtype", Object::Name("Widget".into()));
    field_dict.set("FT", Object::Name(b"Tx".to_vec()));
    field_dict.set("T", Object::String(field_name.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    field_dict.set("V", Object::String(b"0".to_vec(), lopdf::StringFormat::Literal));
    field_dict.set("Rect", Object::Array(vec![
        Object::Real(x as f32), Object::Real(y as f32),
        Object::Real((x + width) as f32), Object::Real((y + height) as f32),
    ]));

    // Add JavaScript action
    let js_code = format!("this.getField(\"{}\").value = {};", field_name, formula);
    let mut action_dict = Dictionary::new();
    action_dict.set("S", Object::Name("JavaScript".into()));
    action_dict.set("JS", Object::String(js_code.as_bytes().to_vec(), lopdf::StringFormat::Literal));

    let action_id = doc.add_object(Object::Dictionary(action_dict));
    field_dict.set("AA", Object::Reference(action_id));

    let field_id = doc.add_object(Object::Dictionary(field_dict));

    // Add to page
    let page_id = page_ids[page_index];
    if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
        let mut annots = match dict.get(b"Annots") {
            Ok(Object::Array(a)) => a.clone(),
            _ => Vec::new(),
        };
        annots.push(Object::Reference(field_id));
        dict.set("Annots", Object::Array(annots));
    }

    save_doc(&mut doc)
}

// ===== XFDF/FDF IMPORT/EXPORT =====

pub fn export_xfdf(data: &[u8]) -> Result<String, String> {
    let doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);

    let mut xfdf = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xfdf.push_str("<xfdf xmlns=\"http://ns.adobe.com/xfdf/\" xml:space=\"preserve\">\n");
    xfdf.push_str("  <annotations>\n");

    for (page_idx, &page_id) in page_ids.iter().enumerate() {
        if let Some(Object::Dictionary(ref dict)) = doc.objects.get(&page_id) {
            if let Ok(Object::Array(annots)) = dict.get(b"Annots") {
                for annot_ref in annots {
                    if let Object::Reference(ref_id) = annot_ref {
                        if let Some(Object::Dictionary(annot_dict)) = doc.objects.get(ref_id) {
                            let annot_type = annot_dict.get(b"Subtype")
                                .ok()
                                .and_then(|o| match o {
                                    Object::Name(bytes) => Some(String::from_utf8_lossy(bytes).to_string()),
                                    _ => None,
                                })
                                .unwrap_or_default();

                            let contents = annot_dict.get(b"Contents")
                                .ok()
                                .and_then(|o| match o {
                                    Object::String(bytes, _) => Some(String::from_utf8_lossy(bytes).to_string()),
                                    _ => None,
                                })
                                .unwrap_or_default();

                            let author = annot_dict.get(b"T")
                                .ok()
                                .and_then(|o| match o {
                                    Object::String(bytes, _) => Some(String::from_utf8_lossy(bytes).to_string()),
                                    _ => None,
                                })
                                .unwrap_or_default();

                            let (x, y, w, h) = match annot_dict.get(b"Rect") {
                                Ok(Object::Array(arr)) if arr.len() >= 4 => {
                                    let x = match &arr[0] { Object::Real(v) => *v, Object::Integer(v) => *v as f32, _ => 0.0 };
                                    let y = match &arr[1] { Object::Real(v) => *v, Object::Integer(v) => *v as f32, _ => 0.0 };
                                    let w = match &arr[2] { Object::Real(v) => *v, Object::Integer(v) => *v as f32, _ => 0.0 };
                                    let h = match &arr[3] { Object::Real(v) => *v, Object::Integer(v) => *v as f32, _ => 0.0 };
                                    (x, y, w - x, h - y)
                                }
                                _ => (0.0, 0.0, 0.0, 0.0),
                            };

                            let color = match annot_dict.get(b"C") {
                                Ok(Object::Array(arr)) if arr.len() >= 3 => {
                                    let r = match &arr[0] { Object::Real(v) => (v * 255.0) as u8, _ => 0 };
                                    let g = match &arr[1] { Object::Real(v) => (v * 255.0) as u8, _ => 0 };
                                    let b = match &arr[2] { Object::Real(v) => (v * 255.0) as u8, _ => 0 };
                                    format!("{},{},{}", r, g, b)
                                }
                                _ => "255,0,0".to_string(),
                            };

                            xfdf.push_str(&format!(
                                "    <{} page=\"{}\" name=\"{}\" title=\"{}\" color=\"{}\" left=\"{}\" top=\"{}\" width=\"{}\" height=\"{}\">\n",
                                annot_type, page_idx + 1, format!("annot_{}", ref_id.0), author, color, x, y, w, h
                            ));
                            if !contents.is_empty() {
                                xfdf.push_str(&format!("      <contents>{}</contents>\n", contents));
                            }
                            xfdf.push_str(&format!("    </{}>\n", annot_type));
                        }
                    }
                }
            }
        }
    }

    xfdf.push_str("  </annotations>\n");
    xfdf.push_str("</xfdf>\n");

    Ok(xfdf)
}

pub fn import_xfdf(data: &[u8], xfdf_content: &str) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);

    // Simple XFDF parser (for basic annotations)
    let lines: Vec<&str> = xfdf_content.lines().collect();
    let mut current_type = String::new();
    let mut current_page = 0;
    let mut current_contents = String::new();
    let mut current_author = String::new();
    let current_rect = (0.0f32, 0.0f32, 100.0f32, 100.0f32);

    for line in &lines {
        let trimmed = line.trim();

        if trimmed.starts_with("<highlight") || trimmed.starts_with("<Text") ||
           trimmed.starts_with("<Underline") || trimmed.starts_with("<StrikeOut") {
            // Parse annotation start
            if let Some(page_start) = trimmed.find("page=\"") {
                let page_str = &trimmed[page_start + 6..];
                if let Some(page_end) = page_str.find('\"') {
                    current_page = page_str[..page_end].parse().unwrap_or(1) - 1;
                }
            }
            if trimmed.starts_with("<highlight") { current_type = "Highlight".to_string(); }
            else if trimmed.starts_with("<Text") { current_type = "Text".to_string(); }
            else if trimmed.starts_with("<Underline") { current_type = "Underline".to_string(); }

            if let Some(title_start) = trimmed.find("title=\"") {
                let title_str = &trimmed[title_start + 7..];
                if let Some(title_end) = title_str.find('\"') {
                    current_author = title_str[..title_end].to_string();
                }
            }
        }

        if trimmed.starts_with("<contents>") && trimmed.ends_with("</contents>") {
            current_contents = trimmed[10..trimmed.len() - 11].to_string();
        }

        if trimmed.starts_with(&format!("<{}>", current_type)) || trimmed.starts_with(&format!("<{} ", current_type)) {
            if !current_type.is_empty() && current_page < page_ids.len() {
                // Create annotation
                let mut annot_dict = Dictionary::new();
                annot_dict.set("Type", Object::Name("Annot".into()));
                annot_dict.set("Subtype", Object::Name(current_type.as_bytes().to_vec()));
                annot_dict.set("Rect", Object::Array(vec![
                    Object::Real(current_rect.0), Object::Real(current_rect.1),
                    Object::Real(current_rect.2), Object::Real(current_rect.3),
                ]));
                if !current_contents.is_empty() {
                    annot_dict.set("Contents", Object::String(current_contents.as_bytes().to_vec(), lopdf::StringFormat::Literal));
                }
                if !current_author.is_empty() {
                    annot_dict.set("T", Object::String(current_author.as_bytes().to_vec(), lopdf::StringFormat::Literal));
                }

                let annot_id = doc.add_object(Object::Dictionary(annot_dict));

                let page_id = page_ids[current_page];
                if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
                    let mut annots = match dict.get(b"Annots") {
                        Ok(Object::Array(a)) => a.clone(),
                        _ => Vec::new(),
                    };
                    annots.push(Object::Reference(annot_id));
                    dict.set("Annots", Object::Array(annots));
                }
            }
            current_type.clear();
            current_contents.clear();
            current_author.clear();
        }
    }

    save_doc(&mut doc)
}


// ===== FORM DATA AGGREGATION =====

pub fn aggregate_form_data(pdf_paths: &[String]) -> Result<serde_json::Value, String> {
    let mut all_data = Vec::new();

    for path in pdf_paths {
        let data = std::fs::read(path).map_err(|e| format!("Failed to read {path}: {e}"))?;
        let fields = get_form_fields(&data)?;
        
        let mut file_data = serde_json::Map::new();
        file_data.insert("file".into(), serde_json::Value::String(path.clone()));
        file_data.insert("fields".into(), serde_json::Value::Array(fields));
        
        all_data.push(serde_json::Value::Object(file_data));
    }

    // Create summary
    let mut summary = serde_json::Map::new();
    summary.insert("total_files".into(), serde_json::Value::Number(all_data.len().into()));
    summary.insert("files".into(), serde_json::Value::Array(all_data));

    Ok(serde_json::Value::Object(summary))
}


// ===== INTERACTIVE FORM CREATION =====

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FormFieldConfig {
    pub field_type: String,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub value: Option<String>,
    pub options: Option<Vec<String>>,
    pub required: bool,
    pub read_only: bool,
    pub max_length: Option<u32>,
}

// Create interactive form field
pub fn create_form_field(
    data: &[u8],
    page_index: usize,
    config: &FormFieldConfig,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let page_id = page_ids[page_index];

    // Create field dictionary
    let mut field_dict = Dictionary::new();
    field_dict.set("Type", Object::Name("Annot".into()));
    field_dict.set("Subtype", Object::Name("Widget".into()));
    field_dict.set("FT", Object::Name(config.field_type.as_bytes().to_vec()));
    field_dict.set("T", Object::String(config.name.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    field_dict.set("Rect", Object::Array(vec![
        Object::Real(config.x),
        Object::Real(config.y),
        Object::Real(config.x + config.width),
        Object::Real(config.y + config.height),
    ]));

    if let Some(ref value) = config.value {
        field_dict.set("V", Object::String(value.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    }

    if config.required {
        field_dict.set("Ff", Object::Integer(1)); // Required
    }

    if config.read_only {
        field_dict.set("Ff", Object::Integer(1 << 1)); // Read only
    }

    if let Some(max_len) = config.max_length {
        field_dict.set("MaxLen", Object::Integer(max_len as i64));
    }

    // Add options for choice fields
    if let Some(ref options) = config.options {
        let opt_array: Vec<Object> = options.iter()
            .map(|o| Object::String(o.as_bytes().to_vec(), lopdf::StringFormat::Literal))
            .collect();
        field_dict.set("Opt", Object::Array(opt_array));
    }

    let field_id = doc.add_object(Object::Dictionary(field_dict));

    // Add field to page annotations
    if let Some(Object::Dictionary(ref mut page_dict)) = doc.objects.get_mut(&page_id) {
        let mut annots = match page_dict.get(b"Annots") {
            Ok(Object::Array(arr)) => arr.clone(),
            _ => Vec::new(),
        };
        annots.push(Object::Reference(field_id));
        page_dict.set("Annots", Object::Array(annots));
    }

    // Add to AcroForm
    let root_id = doc.trailer.get(b"Root")
        .and_then(|o| o.as_reference())
        .ok()
        .ok_or("No root")?;

    if let Some(Object::Dictionary(ref mut root_dict)) = doc.objects.get_mut(&root_id) {
        let mut fields = match root_dict.get(b"AcroForm") {
            Ok(Object::Dictionary(form_dict)) => {
                match form_dict.get(b"Fields") {
                    Ok(Object::Array(arr)) => arr.clone(),
                    _ => Vec::new(),
                }
            }
            _ => Vec::new(),
        };
        fields.push(Object::Reference(field_id));

        let mut acro_form = Dictionary::new();
        acro_form.set("Fields", Object::Array(fields));
        root_dict.set("AcroForm", Object::Dictionary(acro_form));
    }

    save_doc(&mut doc)
}

// Create checkbox field
pub fn create_checkbox(
    data: &[u8],
    page_index: usize,
    name: &str,
    x: f32,
    y: f32,
    checked: bool,
) -> Result<Vec<u8>, String> {
    let config = FormFieldConfig {
        field_type: "Btn".into(),
        name: name.to_string(),
        x,
        y,
        width: 20.0,
        height: 20.0,
        value: Some(if checked { "Yes".into() } else { "Off".into() }),
        options: None,
        required: false,
        read_only: false,
        max_length: None,
    };
    create_form_field(data, page_index, &config)
}

// Create radio button group
pub fn create_radio_button(
    data: &[u8],
    page_index: usize,
    group_name: &str,
    options: &[String],
    x: f32,
    y: f32,
) -> Result<Vec<u8>, String> {
    let mut current_data = data.to_vec();
    let mut current_y = y;

    for (i, _option) in options.iter().enumerate() {
        let field_name = format!("{}_{}", group_name, i);
        let config = FormFieldConfig {
            field_type: "Btn".into(),
            name: field_name,
            x,
            y: current_y,
            width: 20.0,
            height: 20.0,
            value: Some(if i == 0 { "Yes".into() } else { "Off".into() }),
            options: Some(vec!["Yes".into(), "Off".into()]),
            required: false,
            read_only: false,
            max_length: None,
        };
        current_data = create_form_field(&current_data, page_index, &config)?;
        current_y -= 25.0;
    }

    Ok(current_data)
}

// Create text input field
pub fn create_text_field(
    data: &[u8],
    page_index: usize,
    name: &str,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    max_length: Option<u32>,
) -> Result<Vec<u8>, String> {
    let config = FormFieldConfig {
        field_type: "Tx".into(),
        name: name.to_string(),
        x,
        y,
        width,
        height,
        value: None,
        options: None,
        required: false,
        read_only: false,
        max_length,
    };
    create_form_field(data, page_index, &config)
}

// Create signature field
pub fn create_signature_field(
    data: &[u8],
    page_index: usize,
    name: &str,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> Result<Vec<u8>, String> {
    let config = FormFieldConfig {
        field_type: "Sig".into(),
        name: name.to_string(),
        x,
        y,
        width,
        height,
        value: None,
        options: None,
        required: true,
        read_only: false,
        max_length: None,
    };
    create_form_field(data, page_index, &config)
}

// Create dropdown/combo box
pub fn create_dropdown(
    data: &[u8],
    page_index: usize,
    name: &str,
    options: &[String],
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> Result<Vec<u8>, String> {
    let config = FormFieldConfig {
        field_type: "Ch".into(),
        name: name.to_string(),
        x,
        y,
        width,
        height,
        value: options.first().cloned(),
        options: Some(options.to_vec()),
        required: false,
        read_only: false,
        max_length: None,
    };
    create_form_field(data, page_index, &config)
}

