use lopdf::{Dictionary, Document, Object, Stream};

/// Result of setting up a Unicode font pipeline in a PDF document.
pub struct UnicodeFontSetup {
    pub font_name: String,
    pub font_id: (u32, u16),
}

/// Ensures a Type0 / CIDFontType2 Unicode font with Identity-H encoding and
/// an Identity ToUnicode CMap is registered in the PDF document.
///
/// This implements the Unicode Font Pipeline requirement:
/// Type0 Font -> DescendantFonts [CIDFontType2] -> FontDescriptor -> Identity-H -> ToUnicode CMap.
pub fn ensure_unicode_font(doc: &mut Document, font_name: &str) -> (u32, u16) {
    // 1. Check if the font is already registered
    for (&oid, obj) in &doc.objects {
        if let Object::Dictionary(dict) = obj {
            if dict.get(b"Type").ok().and_then(|o| o.as_name().ok()) == Some(b"Font")
                && dict.get(b"BaseFont").ok().and_then(|o| o.as_name().ok())
                    == Some(font_name.as_bytes())
                && dict.get(b"Subtype").ok().and_then(|o| o.as_name().ok()) == Some(b"Type0")
            {
                return oid;
            }
        }
    }

    // 2. Build FontDescriptor
    let mut descriptor = Dictionary::new();
    descriptor.set("Type", Object::Name("FontDescriptor".into()));
    descriptor.set("FontName", Object::Name(font_name.into()));
    descriptor.set("Flags", Object::Integer(4)); // Symbolic font
    descriptor.set(
        "FontBBox",
        Object::Array(vec![
            Object::Real(-437.0),
            Object::Real(-340.0),
            Object::Real(1147.0),
            Object::Real(852.0),
        ]),
    );
    descriptor.set("ItalicAngle", Object::Real(0.0));
    descriptor.set("Ascent", Object::Real(852.0));
    descriptor.set("Descent", Object::Real(-340.0));
    descriptor.set("CapHeight", Object::Real(737.0));
    descriptor.set("StemV", Object::Real(80.0));

    let descriptor_id = doc.add_object(Object::Dictionary(descriptor));

    // 3. Build CIDSystemInfo
    let mut cid_sys_info = Dictionary::new();
    cid_sys_info.set(
        "Registry",
        Object::String(b"Adobe".to_vec(), lopdf::StringFormat::Literal),
    );
    cid_sys_info.set(
        "Ordering",
        Object::String(b"Identity".to_vec(), lopdf::StringFormat::Literal),
    );
    cid_sys_info.set("Supplement", Object::Integer(0));

    // 4. Build CIDFontType2 (DescendantFont)
    let mut cid_font = Dictionary::new();
    cid_font.set("Type", Object::Name("Font".into()));
    cid_font.set("Subtype", Object::Name("CIDFontType2".into()));
    cid_font.set("BaseFont", Object::Name(font_name.into()));
    cid_font.set("CIDSystemInfo", Object::Dictionary(cid_sys_info));
    cid_font.set("FontDescriptor", Object::Reference(descriptor_id));
    cid_font.set("DW", Object::Integer(1000)); // Default glyph width for CJK
                                               // Set widths for ASCII / half-width range (0..=255) to 500
    cid_font.set(
        "W",
        Object::Array(vec![
            Object::Integer(0),
            Object::Integer(255),
            Object::Integer(500),
        ]),
    );

    let cid_font_id = doc.add_object(Object::Dictionary(cid_font));

    // 5. Build ToUnicode CMap stream
    // Mapping UTF-16 code units 1-to-1 in UCS-2 / UTF-16BE
    let cmap_data = b"/CIDInit /ProcSet findresource begin\n\
12 dict begin\n\
begincmap\n\
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
/CMapName /Custom-ToUnicode def\n\
/CMapType 2 def\n\
1 begincodespacerange\n\
<0000> <FFFF>\n\
endcodespacerange\n\
1 beginbfrange\n\
<0000> <FFFF> <0000>\n\
endbfrange\n\
endcmap\n\
CMapName currentdict /CMap defineresource pop\n\
end\n\
end\n";

    let mut cmap_stream = Stream::new(Dictionary::new(), cmap_data.to_vec());
    cmap_stream
        .dict
        .set("Length", Object::Integer(cmap_data.len() as i64));
    let cmap_id = doc.add_object(Object::Stream(cmap_stream));

    // 6. Build Type0 Font
    let mut type0_font = Dictionary::new();
    type0_font.set("Type", Object::Name("Font".into()));
    type0_font.set("Subtype", Object::Name("Type0".into()));
    type0_font.set("BaseFont", Object::Name(font_name.into()));
    type0_font.set("Encoding", Object::Name("Identity-H".into()));
    type0_font.set(
        "DescendantFonts",
        Object::Array(vec![Object::Reference(cid_font_id)]),
    );
    type0_font.set("ToUnicode", Object::Reference(cmap_id));

    doc.add_object(Object::Dictionary(type0_font))
}

/// Encodes a Unicode string into big-endian UTF-16 code unit bytes suitable for
/// Identity-H encoded CIDFont with 1-to-1 ToUnicode mapping.
pub fn encode_unicode_text_to_utf16be_bytes(text: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    for ch in text.encode_utf16() {
        bytes.extend_from_slice(&ch.to_be_bytes());
    }
    bytes
}

/// Formats a Unicode string into a PDF hex string operand: `<HHHHHHHH...>`
pub fn encode_unicode_hex_string(text: &str) -> String {
    let mut hex = String::with_capacity(text.len() * 4 + 2);
    hex.push('<');
    for ch in text.encode_utf16() {
        use std::fmt::Write;
        let _ = write!(&mut hex, "{:04X}", ch);
    }
    hex.push('>');
    hex
}
