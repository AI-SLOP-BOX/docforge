use lopdf::{Dictionary, Document, Object, Stream};
use std::collections::{BTreeMap, HashMap};

/// Representation of a parsed TrueType font for embedding and CID mapping.
pub struct ParsedTrueTypeFont {
    pub font_name: String,
    pub font_data: Vec<u8>,
    pub units_per_em: u16,
    pub ascent: i16,
    pub descent: i16,
    pub cap_height: i16,
    pub bbox: [f32; 4],
    pub char_to_gid: HashMap<u32, u16>,
    pub gid_to_width: Vec<u16>,
}

impl ParsedTrueTypeFont {
    /// Parses essential tables (head, hhea, hmtx, cmap, OS/2) from TrueType byte data.
    pub fn parse(font_data: Vec<u8>, font_name: &str) -> Result<Self, String> {
        if font_data.len() < 12 {
            return Err("TTF data too short".into());
        }

        let num_tables = u16::from_be_bytes([font_data[4], font_data[5]]) as usize;
        let mut tables = HashMap::new();

        let mut offset = 12;
        for _ in 0..num_tables {
            if offset + 16 > font_data.len() {
                break;
            }
            let tag = &font_data[offset..offset + 4];
            let tab_offset = u32::from_be_bytes([
                font_data[offset + 8],
                font_data[offset + 9],
                font_data[offset + 10],
                font_data[offset + 11],
            ]) as usize;
            let tab_length = u32::from_be_bytes([
                font_data[offset + 12],
                font_data[offset + 13],
                font_data[offset + 14],
                font_data[offset + 15],
            ]) as usize;
            tables.insert(tag.to_vec(), (tab_offset, tab_length));
            offset += 16;
        }

        // 1. head table -> unitsPerEm
        let (head_off, _) = tables.get(b"head".as_ref()).ok_or("Missing head table")?;
        let units_per_em = u16::from_be_bytes([font_data[head_off + 18], font_data[head_off + 19]]);
        let x_min = i16::from_be_bytes([font_data[head_off + 36], font_data[head_off + 37]]);
        let y_min = i16::from_be_bytes([font_data[head_off + 38], font_data[head_off + 39]]);
        let x_max = i16::from_be_bytes([font_data[head_off + 40], font_data[head_off + 41]]);
        let y_max = i16::from_be_bytes([font_data[head_off + 42], font_data[head_off + 43]]);

        let scale = 1000.0 / (units_per_em as f32);
        let bbox = [
            (x_min as f32) * scale,
            (y_min as f32) * scale,
            (x_max as f32) * scale,
            (y_max as f32) * scale,
        ];

        // 2. hhea table -> ascender, descender, numberOfHMetrics
        let (hhea_off, _) = tables.get(b"hhea".as_ref()).ok_or("Missing hhea table")?;
        let ascender = i16::from_be_bytes([font_data[hhea_off + 4], font_data[hhea_off + 5]]);
        let descender = i16::from_be_bytes([font_data[hhea_off + 6], font_data[hhea_off + 7]]);
        let num_h_metrics =
            u16::from_be_bytes([font_data[hhea_off + 34], font_data[hhea_off + 35]]) as usize;

        // 3. hmtx table -> widths
        let (hmtx_off, _) = tables.get(b"hmtx".as_ref()).ok_or("Missing hmtx table")?;
        let mut gid_to_width = Vec::with_capacity(num_h_metrics);
        for i in 0..num_h_metrics {
            let metric_off = hmtx_off + i * 4;
            if metric_off + 2 <= font_data.len() {
                let w = u16::from_be_bytes([font_data[metric_off], font_data[metric_off + 1]]);
                gid_to_width.push(w);
            }
        }

        // 4. cmap table -> unicode to glyph ID mapping
        let (cmap_off, _) = tables.get(b"cmap".as_ref()).ok_or("Missing cmap table")?;
        let num_subtables =
            u16::from_be_bytes([font_data[cmap_off + 2], font_data[cmap_off + 3]]) as usize;
        let mut char_to_gid = HashMap::new();

        // Search for Unicode subtables (platform 0 or platform 3)
        let mut chosen_subtable_off = None;
        let mut chosen_format = 0;

        for s in 0..num_subtables {
            let rec_off = cmap_off + 4 + s * 8;
            if rec_off + 8 > font_data.len() {
                break;
            }
            let plat = u16::from_be_bytes([font_data[rec_off], font_data[rec_off + 1]]);
            let enc = u16::from_be_bytes([font_data[rec_off + 2], font_data[rec_off + 3]]);
            let sub_offset = u32::from_be_bytes([
                font_data[rec_off + 4],
                font_data[rec_off + 5],
                font_data[rec_off + 6],
                font_data[rec_off + 7],
            ]) as usize;

            let abs_sub_off = cmap_off + sub_offset;
            if abs_sub_off + 2 <= font_data.len() {
                let fmt = u16::from_be_bytes([font_data[abs_sub_off], font_data[abs_sub_off + 1]]);
                if (plat == 0 || (plat == 3 && (enc == 1 || enc == 10))) && (fmt == 4 || fmt == 12)
                {
                    chosen_subtable_off = Some(abs_sub_off);
                    chosen_format = fmt;
                    if fmt == 12 {
                        // format 12 covers full Unicode
                        break;
                    }
                }
            }
        }

        if let Some(sub_off) = chosen_subtable_off {
            if chosen_format == 4 {
                parse_cmap_format_4(&font_data[sub_off..], &mut char_to_gid);
            } else if chosen_format == 12 {
                parse_cmap_format_12(&font_data[sub_off..], &mut char_to_gid);
            }
        }

        let cap_height = ((ascender as f32) * 0.7) as i16;

        Ok(Self {
            font_name: font_name.to_string(),
            font_data,
            units_per_em,
            ascent: ((ascender as f32) * scale) as i16,
            descent: ((descender as f32) * scale) as i16,
            cap_height: ((cap_height as f32) * scale) as i16,
            bbox,
            char_to_gid,
            gid_to_width,
        })
    }

    /// Gets glyph ID for Unicode char, or 0 if missing.
    pub fn get_gid(&self, ch: char) -> u16 {
        self.char_to_gid.get(&(ch as u32)).copied().unwrap_or(0)
    }

    /// Gets glyph width normalized to 1000 font units.
    pub fn get_glyph_width_1000(&self, gid: u16) -> u16 {
        let raw_w = if (gid as usize) < self.gid_to_width.len() {
            self.gid_to_width[gid as usize]
        } else if let Some(&last) = self.gid_to_width.last() {
            last
        } else {
            self.units_per_em
        };

        let w1000 = (raw_w as f64 * 1000.0 / self.units_per_em as f64).round();
        w1000 as u16
    }
}

fn parse_cmap_format_4(data: &[u8], out: &mut HashMap<u32, u16>) {
    if data.len() < 14 {
        return;
    }
    let seg_count_x2 = u16::from_be_bytes([data[6], data[7]]) as usize;
    let seg_count = seg_count_x2 / 2;
    if data.len() < 16 + seg_count * 8 {
        return;
    }

    let end_codes_off = 14;
    let start_codes_off = end_codes_off + seg_count * 2 + 2;
    let id_deltas_off = start_codes_off + seg_count * 2;
    let id_range_offsets_off = id_deltas_off + seg_count * 2;

    for i in 0..seg_count {
        let end_code =
            u16::from_be_bytes([data[end_codes_off + i * 2], data[end_codes_off + i * 2 + 1]]);
        let start_code = u16::from_be_bytes([
            data[start_codes_off + i * 2],
            data[start_codes_off + i * 2 + 1],
        ]);
        let id_delta =
            i16::from_be_bytes([data[id_deltas_off + i * 2], data[id_deltas_off + i * 2 + 1]]);
        let id_range_offset = u16::from_be_bytes([
            data[id_range_offsets_off + i * 2],
            data[id_range_offsets_off + i * 2 + 1],
        ]);

        if start_code == 0xFFFF {
            break;
        }

        for cp in start_code..=end_code {
            let gid = if id_range_offset == 0 {
                ((cp as i32 + id_delta as i32) & 0xFFFF) as u16
            } else {
                let glyph_idx_off = id_range_offsets_off
                    + i * 2
                    + (id_range_offset as usize)
                    + ((cp - start_code) as usize) * 2;
                if glyph_idx_off + 2 <= data.len() {
                    let raw_gid =
                        u16::from_be_bytes([data[glyph_idx_off], data[glyph_idx_off + 1]]);
                    if raw_gid != 0 {
                        ((raw_gid as i32 + id_delta as i32) & 0xFFFF) as u16
                    } else {
                        0
                    }
                } else {
                    0
                }
            };

            if gid != 0 {
                out.insert(cp as u32, gid);
            }
        }
    }
}

fn parse_cmap_format_12(data: &[u8], out: &mut HashMap<u32, u16>) {
    if data.len() < 16 {
        return;
    }
    let n_groups = u32::from_be_bytes([data[12], data[13], data[14], data[15]]) as usize;
    let mut off = 16;
    for _ in 0..n_groups {
        if off + 12 > data.len() {
            break;
        }
        let start_char =
            u32::from_be_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]);
        let end_char =
            u32::from_be_bytes([data[off + 4], data[off + 5], data[off + 6], data[off + 7]]);
        let start_glyph =
            u32::from_be_bytes([data[off + 8], data[off + 9], data[off + 10], data[off + 11]]);
        off += 12;

        for cp in start_char..=end_char {
            let gid = (start_glyph + (cp - start_char)) as u16;
            out.insert(cp, gid);
        }
    }
}

/// Embedded primary CJK font (IPAexGothic) bundled directly into the binary
const EMBEDDED_IPAEXG_TTF: &[u8] = include_bytes!("../../../assets/fonts/ipaexg.ttf");

/// Primary CJK font loader:
/// 1. Uses the embedded IPAexGothic TTF compiled directly into the binary (guaranteed cross-platform, zero runtime dependency)
/// 2. If an override or external font is needed, checks relative runtime paths and OS fonts.
pub fn load_primary_cjk_font() -> Result<ParsedTrueTypeFont, String> {
    // 1. Check if application has an external override font in runtime resource paths
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let candidates = [
                exe_dir.join("fonts/ipaexg.ttf"),
                exe_dir.join("assets/fonts/ipaexg.ttf"),
                exe_dir.join("../Resources/fonts/ipaexg.ttf"), // macOS .app bundle
                exe_dir.join("../assets/fonts/ipaexg.ttf"),
            ];
            for path in candidates {
                if let Ok(bytes) = std::fs::read(&path) {
                    if let Ok(parsed) = ParsedTrueTypeFont::parse(bytes, "IPAexGothic") {
                        return Ok(parsed);
                    }
                }
            }
        }
    }

    // 2. Primary fallback: Parse the embedded IPAexGothic TTF
    if let Ok(parsed) = ParsedTrueTypeFont::parse(EMBEDDED_IPAEXG_TTF.to_vec(), "IPAexGothic") {
        return Ok(parsed);
    }

    // 3. Fallback to OS system fonts
    #[cfg(target_os = "macos")]
    let sys_candidates = [
        "/System/Library/Fonts/Supplemental/AppleGothic.ttf",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
    ];
    #[cfg(target_os = "windows")]
    let sys_candidates = [
        "C:\\Windows\\Fonts\\msgothic.ttc",
        "C:\\Windows\\Fonts\\meiryo.ttc",
        "C:\\Windows\\Fonts\\yu-gothic.ttc",
    ];
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let sys_candidates = [
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    ];

    for path in sys_candidates {
        if let Ok(bytes) = std::fs::read(path) {
            let stem = std::path::Path::new(path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("NagisaCJKFont");
            if let Ok(parsed) = ParsedTrueTypeFont::parse(bytes, stem) {
                return Ok(parsed);
            }
        }
    }

    Err("No suitable TrueType CJK font found".into())
}

/// Sets up a true, embedded Type0/CIDFontType2 font pipeline for `text` in `doc`.
/// - FontFile2 containing the real TTF font
/// - FontDescriptor linked to FontFile2
/// - CIDFontType2 with proper /W (Widths) and /CIDToGIDMap
/// - Type0 Font with /Encoding /Identity-H
/// - /ToUnicode CMap mapping CID directly back to exact Unicode code point
///
/// Returns `(font_object_id, encoded_cid_bytes)`.
pub fn embed_and_encode_unicode_text(
    doc: &mut Document,
    text: &str,
) -> Result<((u32, u16), Vec<u8>), String> {
    let font = load_primary_cjk_font()?;

    // 1. Build distinct glyph/character mapping for this text
    // CID 1..N allocated sequentially for used characters (CID 0 = .notdef)
    let mut char_to_cid: HashMap<char, u16> = HashMap::new();
    let mut cid_to_char: BTreeMap<u16, char> = BTreeMap::new();
    let mut cid_to_gid: BTreeMap<u16, u16> = BTreeMap::new();
    let mut cid_to_width: BTreeMap<u16, u16> = BTreeMap::new();

    let mut next_cid: u16 = 1;
    let mut encoded_cid_bytes = Vec::new();

    for ch in text.chars() {
        let cid = match char_to_cid.get(&ch) {
            Some(&cid) => cid,
            None => {
                let cid = next_cid;
                next_cid += 1;
                char_to_cid.insert(ch, cid);
                cid_to_char.insert(cid, ch);

                let gid = font.get_gid(ch);
                cid_to_gid.insert(cid, gid);
                cid_to_width.insert(cid, font.get_glyph_width_1000(gid));
                cid
            }
        };

        encoded_cid_bytes.extend_from_slice(&cid.to_be_bytes());
    }

    // 2. Embed FontFile2 stream
    let font_file_len = font.font_data.len();
    let mut ff2_dict = Dictionary::new();
    ff2_dict.set("Length", Object::Integer(font_file_len as i64));
    ff2_dict.set("Length1", Object::Integer(font_file_len as i64));
    let ff2_stream = Stream::new(ff2_dict, font.font_data);
    let ff2_id = doc.add_object(ff2_stream);

    // 3. Build FontDescriptor
    let mut descriptor = Dictionary::new();
    descriptor.set("Type", Object::Name("FontDescriptor".into()));
    descriptor.set("FontName", Object::Name(font.font_name.as_bytes().to_vec()));
    descriptor.set("Flags", Object::Integer(4)); // Symbolic
    descriptor.set(
        "FontBBox",
        Object::Array(vec![
            Object::Real(font.bbox[0]),
            Object::Real(font.bbox[1]),
            Object::Real(font.bbox[2]),
            Object::Real(font.bbox[3]),
        ]),
    );
    descriptor.set("ItalicAngle", Object::Real(0.0));
    descriptor.set("Ascent", Object::Real(font.ascent as f32));
    descriptor.set("Descent", Object::Real(font.descent as f32));
    descriptor.set("CapHeight", Object::Real(font.cap_height as f32));
    descriptor.set("StemV", Object::Real(80.0));
    descriptor.set("FontFile2", Object::Reference(ff2_id));
    let descriptor_id = doc.add_object(Object::Dictionary(descriptor));

    // 4. Build CIDToGIDMap stream
    // Mapping table: 2 bytes per CID from 0 up to max_cid
    let max_cid = cid_to_gid.keys().copied().max().unwrap_or(0);
    let mut cid_to_gid_bytes = vec![0u8; (max_cid as usize + 1) * 2];
    for (&cid, &gid) in &cid_to_gid {
        let off = (cid as usize) * 2;
        cid_to_gid_bytes[off] = (gid >> 8) as u8;
        cid_to_gid_bytes[off + 1] = (gid & 0xFF) as u8;
    }
    let mut c2g_dict = Dictionary::new();
    c2g_dict.set("Length", Object::Integer(cid_to_gid_bytes.len() as i64));
    let c2g_stream = Stream::new(c2g_dict, cid_to_gid_bytes);
    let c2g_id = doc.add_object(c2g_stream);

    // 5. Build CIDFontType2 (DescendantFont)
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

    // Build /W (Widths) array: [ cid [ width ] ... ]
    let mut w_array = Vec::new();
    for (&cid, &w) in &cid_to_width {
        w_array.push(Object::Integer(cid as i64));
        w_array.push(Object::Array(vec![Object::Integer(w as i64)]));
    }

    let mut cid_font = Dictionary::new();
    cid_font.set("Type", Object::Name("Font".into()));
    cid_font.set("Subtype", Object::Name("CIDFontType2".into()));
    cid_font.set("BaseFont", Object::Name(font.font_name.as_bytes().to_vec()));
    cid_font.set("CIDSystemInfo", Object::Dictionary(cid_sys_info));
    cid_font.set("FontDescriptor", Object::Reference(descriptor_id));
    cid_font.set("CIDToGIDMap", Object::Reference(c2g_id));
    cid_font.set("DW", Object::Integer(1000));
    cid_font.set("W", Object::Array(w_array));
    let cid_font_id = doc.add_object(Object::Dictionary(cid_font));

    // 6. Build ToUnicode CMap stream
    let mut cmap_str = String::from(
        "/CIDInit /ProcSet findresource begin\n\
12 dict begin\n\
begincmap\n\
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
/CMapName /Custom-ToUnicode def\n\
/CMapType 2 def\n\
1 begincodespacerange\n\
<0000> <FFFF>\n\
endcodespacerange\n",
    );

    use std::fmt::Write;
    let _ = writeln!(&mut cmap_str, "{} beginbfchar", cid_to_char.len());
    for (&cid, &ch) in &cid_to_char {
        let mut u16_hex = String::new();
        for unit in ch.encode_utf16(&mut [0; 2]) {
            let _ = write!(&mut u16_hex, "{:04X}", unit);
        }
        let _ = writeln!(&mut cmap_str, "<{:04X}> <{}>", cid, u16_hex);
    }
    let _ = write!(
        &mut cmap_str,
        "endbfchar\n\
endcmap\n\
CMapName currentdict /CMap defineresource pop\n\
end\n\
end\n"
    );

    let cmap_bytes = cmap_str.into_bytes();
    let mut cmap_dict = Dictionary::new();
    cmap_dict.set("Length", Object::Integer(cmap_bytes.len() as i64));
    let cmap_stream = Stream::new(cmap_dict, cmap_bytes);
    let cmap_id = doc.add_object(cmap_stream);

    // 7. Build Type0 Font
    let mut type0_font = Dictionary::new();
    type0_font.set("Type", Object::Name("Font".into()));
    type0_font.set("Subtype", Object::Name("Type0".into()));
    type0_font.set("BaseFont", Object::Name(font.font_name.as_bytes().to_vec()));
    type0_font.set("Encoding", Object::Name("Identity-H".into()));
    type0_font.set(
        "DescendantFonts",
        Object::Array(vec![Object::Reference(cid_font_id)]),
    );
    type0_font.set("ToUnicode", Object::Reference(cmap_id));

    let type0_id = doc.add_object(Object::Dictionary(type0_font));

    Ok((type0_id, encoded_cid_bytes))
}

/// A prepared Unicode font encoder that shares a single FontDescriptor, FontFile2,
/// CIDToGIDMap, /W and ToUnicode CMap across multiple strings or words on one or more pages.
pub struct UnicodeFontEncoder {
    pub font_id: (u32, u16),
    char_to_cid: HashMap<char, u16>,
}

impl UnicodeFontEncoder {
    /// Encodes a word or string into CID 2-byte hex bytes using the unified mapping.
    pub fn encode_text(&self, text: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        for ch in text.chars() {
            let cid = self.char_to_cid.get(&ch).copied().unwrap_or(0);
            bytes.extend_from_slice(&cid.to_be_bytes());
        }
        bytes
    }
}

/// Creates a unified Unicode font embedded in `doc` covering all characters in `all_text`.
/// Returns a `UnicodeFontEncoder` allowing multiple words/lines to encode to the exact same CID mapping.
pub fn create_unicode_font_encoder(
    doc: &mut Document,
    all_text: &str,
) -> Result<UnicodeFontEncoder, String> {
    let font = load_primary_cjk_font()?;

    let mut char_to_cid: HashMap<char, u16> = HashMap::new();
    let mut cid_to_char: BTreeMap<u16, char> = BTreeMap::new();
    let mut cid_to_gid: BTreeMap<u16, u16> = BTreeMap::new();
    let mut cid_to_width: BTreeMap<u16, u16> = BTreeMap::new();

    let mut next_cid: u16 = 1;
    for ch in all_text.chars() {
        if !char_to_cid.contains_key(&ch) {
            let cid = next_cid;
            next_cid += 1;
            char_to_cid.insert(ch, cid);
            cid_to_char.insert(cid, ch);

            let gid = font.get_gid(ch);
            cid_to_gid.insert(cid, gid);
            cid_to_width.insert(cid, font.get_glyph_width_1000(gid));
        }
    }

    // Embed FontFile2 stream
    let font_file_len = font.font_data.len();
    let mut ff2_dict = Dictionary::new();
    ff2_dict.set("Length", Object::Integer(font_file_len as i64));
    ff2_dict.set("Length1", Object::Integer(font_file_len as i64));
    let ff2_stream = Stream::new(ff2_dict, font.font_data);
    let ff2_id = doc.add_object(ff2_stream);

    // Build FontDescriptor
    let mut descriptor = Dictionary::new();
    descriptor.set("Type", Object::Name("FontDescriptor".into()));
    descriptor.set("FontName", Object::Name(font.font_name.as_bytes().to_vec()));
    descriptor.set("Flags", Object::Integer(4));
    descriptor.set(
        "FontBBox",
        Object::Array(vec![
            Object::Real(font.bbox[0]),
            Object::Real(font.bbox[1]),
            Object::Real(font.bbox[2]),
            Object::Real(font.bbox[3]),
        ]),
    );
    descriptor.set("ItalicAngle", Object::Real(0.0));
    descriptor.set("Ascent", Object::Real(font.ascent as f32));
    descriptor.set("Descent", Object::Real(font.descent as f32));
    descriptor.set("CapHeight", Object::Real(font.cap_height as f32));
    descriptor.set("StemV", Object::Real(80.0));
    descriptor.set("FontFile2", Object::Reference(ff2_id));
    let descriptor_id = doc.add_object(Object::Dictionary(descriptor));

    // Build CIDToGIDMap stream
    let max_cid = cid_to_gid.keys().copied().max().unwrap_or(0);
    let mut cid_to_gid_bytes = vec![0u8; (max_cid as usize + 1) * 2];
    for (&cid, &gid) in &cid_to_gid {
        let off = (cid as usize) * 2;
        cid_to_gid_bytes[off] = (gid >> 8) as u8;
        cid_to_gid_bytes[off + 1] = (gid & 0xFF) as u8;
    }
    let mut c2g_dict = Dictionary::new();
    c2g_dict.set("Length", Object::Integer(cid_to_gid_bytes.len() as i64));
    let c2g_stream = Stream::new(c2g_dict, cid_to_gid_bytes);
    let c2g_id = doc.add_object(c2g_stream);

    // Build CIDFontType2
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

    let mut w_array = Vec::new();
    for (&cid, &w) in &cid_to_width {
        w_array.push(Object::Integer(cid as i64));
        w_array.push(Object::Array(vec![Object::Integer(w as i64)]));
    }

    let mut cid_font = Dictionary::new();
    cid_font.set("Type", Object::Name("Font".into()));
    cid_font.set("Subtype", Object::Name("CIDFontType2".into()));
    cid_font.set("BaseFont", Object::Name(font.font_name.as_bytes().to_vec()));
    cid_font.set("CIDSystemInfo", Object::Dictionary(cid_sys_info));
    cid_font.set("FontDescriptor", Object::Reference(descriptor_id));
    cid_font.set("CIDToGIDMap", Object::Reference(c2g_id));
    cid_font.set("DW", Object::Integer(1000));
    cid_font.set("W", Object::Array(w_array));
    let cid_font_id = doc.add_object(Object::Dictionary(cid_font));

    // Build ToUnicode CMap stream
    let mut cmap_str = String::from(
        "/CIDInit /ProcSet findresource begin\n\
12 dict begin\n\
begincmap\n\
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
/CMapName /Custom-ToUnicode def\n\
/CMapType 2 def\n\
1 begincodespacerange\n\
<0000> <FFFF>\n\
endcodespacerange\n",
    );

    use std::fmt::Write;
    let _ = writeln!(&mut cmap_str, "{} beginbfchar", cid_to_char.len());
    for (&cid, &ch) in &cid_to_char {
        let mut u16_hex = String::new();
        for unit in ch.encode_utf16(&mut [0; 2]) {
            let _ = write!(&mut u16_hex, "{:04X}", unit);
        }
        let _ = writeln!(&mut cmap_str, "<{:04X}> <{}>", cid, u16_hex);
    }
    let _ = write!(
        &mut cmap_str,
        "endbfchar\n\
endcmap\n\
CMapName currentdict /CMap defineresource pop\n\
end\n\
end\n"
    );

    let cmap_bytes = cmap_str.into_bytes();
    let mut cmap_dict = Dictionary::new();
    cmap_dict.set("Length", Object::Integer(cmap_bytes.len() as i64));
    let cmap_stream = Stream::new(cmap_dict, cmap_bytes);
    let cmap_id = doc.add_object(cmap_stream);

    // Build Type0 Font
    let mut type0_font = Dictionary::new();
    type0_font.set("Type", Object::Name("Font".into()));
    type0_font.set("Subtype", Object::Name("Type0".into()));
    type0_font.set("BaseFont", Object::Name(font.font_name.as_bytes().to_vec()));
    type0_font.set("Encoding", Object::Name("Identity-H".into()));
    type0_font.set(
        "DescendantFonts",
        Object::Array(vec![Object::Reference(cid_font_id)]),
    );
    type0_font.set("ToUnicode", Object::Reference(cmap_id));
    let type0_id = doc.add_object(Object::Dictionary(type0_font));

    Ok(UnicodeFontEncoder {
        font_id: type0_id,
        char_to_cid,
    })
}

/// Fallback helper maintaining backwards compatibility for ensure_unicode_font.
pub fn ensure_unicode_font(doc: &mut Document, _font_name: &str) -> (u32, u16) {
    if let Ok((fid, _)) = embed_and_encode_unicode_text(doc, " ") {
        fid
    } else {
        (1, 0)
    }
}

/// Helper encoding Unicode string to UTF-16BE bytes (e.g. for simple/OCR text where font maps Identity).
pub fn encode_unicode_text_to_utf16be_bytes(text: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    for u16_val in text.encode_utf16() {
        bytes.extend_from_slice(&u16_val.to_be_bytes());
    }
    bytes
}
