use super::common::*;
use lopdf::{Dictionary, Document, Object};

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
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
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
    field_dict.set(
        "T",
        Object::String(
            config.name.as_bytes().to_vec(),
            lopdf::StringFormat::Literal,
        ),
    );
    field_dict.set(
        "Rect",
        Object::Array(vec![
            Object::Real(config.x),
            Object::Real(config.y),
            Object::Real(config.x + config.width),
            Object::Real(config.y + config.height),
        ]),
    );

    if let Some(ref value) = config.value {
        field_dict.set(
            "V",
            Object::String(value.as_bytes().to_vec(), lopdf::StringFormat::Literal),
        );
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
        let opt_array: Vec<Object> = options
            .iter()
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
    let root_id = doc
        .trailer
        .get(b"Root")
        .and_then(|o| o.as_reference())
        .ok()
        .ok_or("No root")?;

    if let Some(Object::Dictionary(ref mut root_dict)) = doc.objects.get_mut(&root_id) {
        let mut fields = match root_dict.get(b"AcroForm") {
            Ok(Object::Dictionary(form_dict)) => match form_dict.get(b"Fields") {
                Ok(Object::Array(arr)) => arr.clone(),
                _ => Vec::new(),
            },
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
