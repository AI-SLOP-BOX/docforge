use lopdf::{Document, Object, Stream, Dictionary};
use super::common::*;

// ===== JIS X 4051 準拠 日本語禁則判定 & プロポーショナルグリフ幅 =====

pub(crate) fn is_kinsoku_line_start(c: char) -> bool {
    matches!(
        c,
        '、' | '。' | '，' | '．' | '・' | '：' | '；' | '？' | '！' |
        '）' | '］' | '｝' | '〉' | '》' | '」' | '』' | '】' | '〕' | '〟' |
        'ヽ' | 'ヾ' | 'ゝ' | 'ゞ' | '々' | 'ー' | 'ァ' | 'ィ' | 'ゥ' | 'ェ' | 'ォ' |
        'ッ' | 'ャ' | 'ュ' | 'ョ' | 'ヮ' | 'ヵ' | 'ヶ' | 'ぁ' | 'ぃ' | 'ぅ' | 'ぇ' |
        'ぉ' | 'っ' | 'ゃ' | 'ゅ' | 'ょ' | 'ゎ' | '℃' | '％' | '‰'
    )
}

pub(crate) fn is_kinsoku_line_end(c: char) -> bool {
    matches!(
        c,
        '（' | '［' | '｛' | '〈' | '《' | '「' | '『' | '【' | '〔' | '‘' | '“' |
        '￥' | '＄' | '￡' | '＃'
    )
}

pub fn get_char_metric_width(c: char, font_size: f32) -> f32 {
    let scale = font_size;
    match c {
        // 半角スペース
        ' ' => scale * 0.28,
        // 全角スペース
        '\u{3000}' => scale * 1.0,
        // 欧文文字（プロポーショナル幅）
        'i' | 'l' | 'I' | 'j' | '!' | '.' | ':' | ';' | '\'' => scale * 0.28,
        'f' | 'r' | 't' | '(' | ')' | '[' | ']' => scale * 0.35,
        'm' | 'w' | 'M' | 'W' => scale * 0.85,
        c if c.is_ascii_alphanumeric() => scale * 0.55,
        c if c.is_ascii_punctuation() => scale * 0.40,
        // 句読点（全角だが約物詰めを考慮）
        '、' | '。' | '，' | '．' => scale * 0.65,
        '「' | '」' | '『' | '』' | '（' | '）' => scale * 0.60,
        // CJK全角文字（漢字・ひらがな・カタカナ等）
        _ => scale * 1.0,
    }
}

pub fn reflow_text(
    data: &[u8],
    page_index: usize,
    new_text: &str,
    start_x: f64,
    start_y: f64,
    max_width: f64,
    font_size: f32,
    line_height: f32,
    color: &str,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    let (r, g, b) = parse_hex_color(color, (0.0, 0.0, 0.0));

    let target_max_width = max_width as f32;
    let mut wrapped_lines: Vec<String> = Vec::new();

    // 段落ごとに分割して高度組版（JIS X 4051準拠 禁則処理 ＋ プロポーショナル幅計算）
    for raw_paragraph in new_text.split('\n') {
        if raw_paragraph.is_empty() {
            wrapped_lines.push(String::new());
            continue;
        }

        let chars: Vec<char> = raw_paragraph.chars().collect();
        let mut current_line = String::new();
        let mut current_line_width = 0.0f32;
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];
            let char_w = get_char_metric_width(c, font_size);

            // 欧文単語の場合は単語単位で折り返しを保護
            if c.is_ascii_alphanumeric() {
                let mut word = String::new();
                let mut word_w = 0.0f32;
                let mut j = i;
                while j < chars.len() && chars[j].is_ascii_alphanumeric() {
                    word.push(chars[j]);
                    word_w += get_char_metric_width(chars[j], font_size);
                    j += 1;
                }

                if current_line_width + word_w > target_max_width && !current_line.is_empty() {
                    wrapped_lines.push(current_line);
                    current_line = word;
                    current_line_width = word_w;
                } else {
                    current_line.push_str(&word);
                    current_line_width += word_w;
                }
                i = j;
                continue;
            }

            // 通常のCJKまたは記号文字の幅判定
            if current_line_width + char_w > target_max_width && !current_line.is_empty() {
                // 行頭禁則処理：次に来る文字が行頭禁則文字の場合、前の行末文字を次の行へ巻き込む（追い出し）
                if is_kinsoku_line_start(c) {
                    if let Some(prev_char) = current_line.pop() {
                        wrapped_lines.push(current_line);
                        current_line = String::new();
                        current_line.push(prev_char);
                        current_line.push(c);
                        current_line_width = get_char_metric_width(prev_char, font_size) + char_w;
                        i += 1;
                        continue;
                    }
                }

                wrapped_lines.push(current_line);
                current_line = String::new();
                current_line.push(c);
                current_line_width = char_w;
            } else {
                // 行末禁則処理：現在の行末に置いてはいけない文字（「、『 など）が最後の文字になる場合
                if is_kinsoku_line_end(c) && (current_line_width + char_w + font_size > target_max_width) {
                    if !current_line.is_empty() {
                        wrapped_lines.push(current_line);
                        current_line = String::new();
                    }
                }
                current_line.push(c);
                current_line_width += char_w;
            }

            i += 1;
        }

        if !current_line.is_empty() {
            wrapped_lines.push(current_line);
        }
    }

    // 既存ページコンテンツの末尾にテキストブロックを追加（既存内容を破壊しない安全なBT/ETストリーム）
    let mut operations = vec![
        lopdf::content::Operation::new("q", vec![]),
        lopdf::content::Operation::new("BT", vec![]),
        lopdf::content::Operation::new("Tf", vec![Object::Name("Helvetica".into()), Object::Real(font_size)]),
        lopdf::content::Operation::new("rg", vec![Object::Real(r), Object::Real(g), Object::Real(b)]),
    ];

    for (i, line) in wrapped_lines.iter().enumerate() {
        let line_y = (start_y as f32) - (i as f32 * line_height);
        operations.push(lopdf::content::Operation::new("Tm", vec![
            Object::Real(1.0), Object::Real(0.0),
            Object::Real(0.0), Object::Real(1.0),
            Object::Real(start_x as f32), Object::Real(line_y),
        ]));
        operations.push(lopdf::content::Operation::new("Tj", vec![
            Object::String(line.as_bytes().to_vec(), lopdf::StringFormat::Literal),
        ]));
    }

    operations.push(lopdf::content::Operation::new("ET", vec![]));
    operations.push(lopdf::content::Operation::new("Q", vec![]));

    let content = lopdf::content::Content { operations };
    let content_bytes = content.encode().map_err(|e| format!("Encode error: {e}"))?;

    let mut stream = Stream::new(Dictionary::new(), content_bytes);
    stream.dict.set("Type", Object::Name("Content".into()));
    let content_id = doc.add_object(stream);

    let page_id = page_ids[page_index];
    if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
        // 既存のContentsがある場合は配列で追加、なければ単体セット
        match dict.get(b"Contents") {
            Ok(Object::Array(ref existing)) => {
                let mut new_contents = existing.clone();
                new_contents.push(Object::Reference(content_id));
                dict.set("Contents", Object::Array(new_contents));
            }
            Ok(Object::Reference(ref existing_id)) => {
                let new_contents = vec![Object::Reference(*existing_id), Object::Reference(content_id)];
                dict.set("Contents", Object::Array(new_contents));
            }
            _ => {
                dict.set("Contents", Object::Reference(content_id));
            }
        }
    }

    save_doc(&mut doc)
}
