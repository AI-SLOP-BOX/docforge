use super::common::*;
use lopdf::{Dictionary, Document, Object};

// ===== DIGITAL SIGNATURE =====

/// PDF上に電子署名用ウィジェットフィールド（署名枠）およびプレースホルダー辞書を追加します。
/// ※ 注意: 本機能は暗号鍵/証明書によるPKCS#7バイナリ署名（ハッシュ計算・暗号化）を行うものではなく、
///    署名対象の位置・署名者名・理由メタデータを保持する「未署名フォームフィールド（署名プレースホルダー）」を作成します。
pub fn add_digital_signature(
    data: &[u8],
    page_index: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    signer_name: &str,
    reason: &str,
    _certificate_data: Option<&[u8]>,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let page_ids = get_page_ids(&doc);
    if page_index >= page_ids.len() {
        return Err("Page index out of range".into());
    }

    // Create signature dictionary (placeholder with empty Contents)
    let mut sig_dict = Dictionary::new();
    sig_dict.set("Type", Object::Name("Sig".into()));
    sig_dict.set("Filter", Object::Name("Adobe.PPKLite".into()));
    sig_dict.set("SubFilter", Object::Name("adbe.pkcs7.detached".into()));
    sig_dict.set(
        "ByteRange",
        Object::Array(vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Integer(0),
            Object::Integer(0),
        ]),
    );
    sig_dict.set(
        "Contents",
        Object::String(vec![0u8; 4096], lopdf::StringFormat::Hexadecimal),
    );
    sig_dict.set(
        "Reason",
        Object::String(reason.as_bytes().to_vec(), lopdf::StringFormat::Literal),
    );
    // Timestamp format according to PDF spec: D:YYYYMMDDHHmmSSZ
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Calculate basic UTC date components
    let seconds_in_day = 86400;
    let days = duration / seconds_in_day;
    let time_of_day = duration % seconds_in_day;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Approximate UTC year, month, day calculation from days since 1970-01-01
    let mut year = 1970;
    let mut rem_days = days;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let days_in_year = if leap { 366 } else { 365 };
        if rem_days >= days_in_year {
            rem_days -= days_in_year;
            year += 1;
        } else {
            break;
        }
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let days_in_months = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1;
    for &dim in &days_in_months {
        if rem_days >= dim {
            rem_days -= dim;
            month += 1;
        } else {
            break;
        }
    }
    let day = rem_days + 1;
    let pdf_date = format!("D:{year:04}{month:02}{day:02}{hours:02}{minutes:02}{seconds:02}Z");

    sig_dict.set(
        "M",
        Object::String(
            pdf_date.into_bytes(),
            lopdf::StringFormat::Literal,
        ),
    );
    sig_dict.set(
        "Name",
        Object::String(
            signer_name.as_bytes().to_vec(),
            lopdf::StringFormat::Literal,
        ),
    );

    let sig_id = doc.add_object(Object::Dictionary(sig_dict));

    // Create signature field
    let mut field_dict = Dictionary::new();
    field_dict.set("Type", Object::Name("Annot".into()));
    field_dict.set("Subtype", Object::Name("Widget".into()));
    field_dict.set("FT", Object::Name("Sig".into()));
    field_dict.set(
        "T",
        Object::String(b"Signature1".to_vec(), lopdf::StringFormat::Literal),
    );
    field_dict.set("V", Object::Reference(sig_id));
    field_dict.set(
        "Rect",
        Object::Array(vec![
            Object::Real(x as f32),
            Object::Real(y as f32),
            Object::Real((x + width) as f32),
            Object::Real((y + height) as f32),
        ]),
    );

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

    // Add to AcroForm
    let root_id = doc
        .trailer
        .get(b"Root")
        .and_then(|o| o.as_reference())
        .ok()
        .ok_or("No root")?;

    // Get acroform_id first to avoid borrow issues
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
            acroform.set("SigFlags", Object::Integer(3));
        }
    }

    save_doc(&mut doc)
}

pub fn verify_signature_in_doc(doc: &Document) -> Result<serde_json::Value, String> {
    // Find signature fields and extract actual dictionary metadata
    let mut signatures = Vec::new();

    for (_, obj) in doc.objects.iter() {
        if let Object::Dictionary(dict) = obj {
            if let Ok(Object::Name(ft)) = dict.get(b"FT") {
                if ft == b"Sig" {
                    let name = dict
                        .get(b"T")
                        .ok()
                        .and_then(|o| match o {
                            Object::String(bytes, _) => {
                                Some(String::from_utf8_lossy(bytes).to_string())
                            }
                            _ => None,
                        })
                        .unwrap_or_default();

                    // V can be a direct dictionary or an indirect reference to signature dictionary
                    let sig_dict = match dict.get(b"V") {
                        Ok(Object::Dictionary(d)) => Some(d),
                        Ok(Object::Reference(id)) => doc.objects.get(id).and_then(|o| o.as_dict().ok()),
                        _ => None,
                    };

                    let signer = sig_dict
                        .and_then(|d| d.get(b"Name").ok())
                        .or_else(|| dict.get(b"Name").ok())
                        .and_then(|o| match o {
                            Object::String(bytes, _) => Some(String::from_utf8_lossy(bytes).to_string()),
                            _ => None,
                        })
                        .unwrap_or_default();

                    let reason = sig_dict
                        .and_then(|d| d.get(b"Reason").ok())
                        .or_else(|| dict.get(b"Reason").ok())
                        .and_then(|o| match o {
                            Object::String(bytes, _) => Some(String::from_utf8_lossy(bytes).to_string()),
                            _ => None,
                        })
                        .unwrap_or_default();

                    let filter = sig_dict
                        .and_then(|d| d.get(b"Filter").ok())
                        .or_else(|| dict.get(b"Filter").ok())
                        .and_then(|o| match o {
                            Object::Name(bytes) => Some(String::from_utf8_lossy(bytes).to_string()),
                            _ => None,
                        })
                        .unwrap_or_else(|| "Adobe.PPKLite".to_string());

                    let sub_filter = sig_dict
                        .and_then(|d| d.get(b"SubFilter").ok())
                        .or_else(|| dict.get(b"SubFilter").ok())
                        .and_then(|o| match o {
                            Object::Name(bytes) => Some(String::from_utf8_lossy(bytes).to_string()),
                            _ => None,
                        })
                        .unwrap_or_else(|| "adbe.pkcs7.detached".to_string());

                    let timestamp = sig_dict
                        .and_then(|d| d.get(b"M").ok())
                        .or_else(|| dict.get(b"M").ok())
                        .and_then(|o| match o {
                            Object::String(bytes, _) => Some(String::from_utf8_lossy(bytes).to_string()),
                            _ => None,
                        })
                        .unwrap_or_else(|| "未指定".to_string());

                    // Honest validation: Lopdf inspects structural PDF dictionaries, but does not perform
                    // full cryptographic PKCS#7 / CMS signature verification or ByteRange digest hashing.
                    signatures.push(serde_json::json!({
                        "name": if name.is_empty() { "SignatureField" } else { &name },
                        "signer": if signer.is_empty() { "未指定の署名者" } else { &signer },
                        "reason": if reason.is_empty() { "未指定" } else { &reason },
                        "status": "unverified_structure_only",
                        "filter": filter,
                        "sub_filter": sub_filter,
                        "timestamp": timestamp,
                        "aatl_verified": false,
                        "trust_level": "暗号署名エンジン未検証 (構造確認のみ)",
                        "certificate_issuer": "検証未実施 (外部PKI照合が必要)",
                        "revocation_check": "未照合",
                        "integrity_verified": false,
                        "notice": "PDF構造上の署名フィールドを検出しました。暗号ダイジェストおよびPKCS#7署名チェーンの完全な検証には外部PKI/CMSエンジンが必要です。"
                    }));
                }
            }
        }
    }

    Ok(serde_json::json!({
        "signatures": signatures,
        "count": signatures.len(),
    }))
}

pub fn verify_signature(data: &[u8], _signature_index: usize) -> Result<serde_json::Value, String> {
    let doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    verify_signature_in_doc(&doc)
}

// ===== HARDWARE TOKEN (PKCS#11 STUB) =====

pub struct HardwareToken {
    pub slot_id: u32,
    pub label: String,
    pub manufacturer: String,
    pub serial: String,
    pub initialized: bool,
}

pub fn detect_hardware_tokens() -> Result<Vec<HardwareToken>, String> {
    // Honest: Return empty list when no PKCS#11 hardware device/HSM is connected
    Ok(Vec::new())
}

pub fn sign_with_hardware_token(
    _data: &[u8],
    _slot_id: u32,
    _pin: &str,
    _page_index: usize,
    _x: f64,
    _y: f64,
    _width: f64,
    _height: f64,
    _signer_name: &str,
    _reason: &str,
) -> Result<Vec<u8>, String> {
    Err("ハードウェアトークン(PKCS#11)署名は現在未接続です。ハードウェアHSMまたはスマートカードリーダーを接続してください。".into())
}

pub fn verify_hardware_token_signature(
    _data: &[u8],
    _slot_id: u32,
) -> Result<serde_json::Value, String> {
    Err("ハードウェアトークン署名検証用PKCS#11スロットが接続されていません。".into())
}

// ===== PDF UNLOCK (PASSWORD REMOVAL) =====

pub fn unlock_pdf(data: &[u8], _password: &str) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

    // Check if the document truly has an Encrypt dictionary
    if doc.trailer.has(b"Encrypt") {
        // Warning / Error: lopdf parses encrypted PDFs with encrypted strings and streams still ciphered.
        // Merely removing the /Encrypt entry leaves raw ciphered streams and corrupts the document completely.
        return Err(
            "暗号化されたPDFストリームの復号にはPDF暗号化ハンドラ（標準セキュリティハンドラ・鍵導出スケジュール）の実装が必要です。暗号化辞書を強制除去するとPDFが破損するため実行を拒否しました。".into()
        );
    }

    save_doc(&mut doc)
}

// ===== SANITIZE DOCUMENT (Document Sanitization Feature) =====

#[derive(serde::Serialize)]
pub struct SanitizeSummary {
    pub metadata_removed: bool,
    pub annotations_purged: usize,
    pub attachments_removed: usize,
    pub javascript_removed: bool,
    pub thumbnails_purged: usize,
}

pub fn sanitize_document(data: &[u8]) -> Result<(Vec<u8>, SanitizeSummary), String> {
    let mut doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to load PDF for sanitization: {e}"))?;

    let mut summary = SanitizeSummary {
        metadata_removed: false,
        annotations_purged: 0,
        attachments_removed: 0,
        javascript_removed: false,
        thumbnails_purged: 0,
    };

    // 1. Purge Trailer Info Dictionary & XMP Metadata Stream
    if doc.trailer.has(b"Info") {
        doc.trailer.remove(b"Info");
        summary.metadata_removed = true;
    }

    let root_id = doc
        .trailer
        .get(b"Root")
        .and_then(|o| o.as_reference())
        .ok()
        .ok_or("No root in PDF trailer")?;

    if let Some(Object::Dictionary(ref mut root_dict)) = doc.objects.get_mut(&root_id) {
        if root_dict.has(b"Metadata") {
            root_dict.remove(b"Metadata");
            summary.metadata_removed = true;
        }
        if root_dict.has(b"PieceInfo") {
            root_dict.remove(b"PieceInfo");
            summary.metadata_removed = true;
        }
        // Purge embedded JavaScript
        if root_dict.has(b"Names") || root_dict.has(b"OpenAction") || root_dict.has(b"AA") {
            root_dict.remove(b"OpenAction");
            root_dict.remove(b"AA");
            summary.javascript_removed = true;
        }
        // Purge embedded files / attachments
        if root_dict.has(b"EmbeddedFiles") {
            root_dict.remove(b"EmbeddedFiles");
            summary.attachments_removed += 1;
        }
    }

    // 2. Scan and purge Page-level Annotations, Thumbnails, and Actions
    let page_ids = get_page_ids(&doc);
    for page_id in page_ids {
        if let Some(Object::Dictionary(ref mut page_dict)) = doc.objects.get_mut(&page_id) {
            if let Ok(Object::Array(ref annots)) = page_dict.get(b"Annots") {
                summary.annotations_purged += annots.len();
                page_dict.remove(b"Annots");
            }
            if page_dict.has(b"Thumb") {
                page_dict.remove(b"Thumb");
                summary.thumbnails_purged += 1;
            }
            if page_dict.has(b"AA") {
                page_dict.remove(b"AA");
            }
            if page_dict.has(b"PieceInfo") {
                page_dict.remove(b"PieceInfo");
            }
        }
    }

    // 3. Remove all stray Names / Javascript / Embedded file dictionaries
    let mut keys_to_remove = Vec::new();
    for (&id, obj) in &doc.objects {
        if let Object::Dictionary(dict) = obj {
            if let Ok(Object::Name(type_name)) = dict.get(b"Type") {
                if type_name == b"Metadata"
                    || type_name == b"JavaScript"
                    || type_name == b"EmbeddedFile"
                {
                    keys_to_remove.push(id);
                }
            }
        }
    }
    for id in keys_to_remove {
        doc.objects.remove(&id);
    }

    // 4. Prune unused objects & renumber
    doc.prune_objects();

    let clean_bytes = save_doc(&mut doc)?;
    Ok((clean_bytes, summary))
}

// ===== DIGITAL ID MANAGEMENT =====

#[derive(serde::Serialize)]
pub struct DigitalID {
    pub name: String,
    pub issuer: String,
    pub valid_from: String,
    pub valid_to: String,
    pub key_usage: Vec<String>,
}

pub fn list_digital_ids() -> Result<Vec<DigitalID>, String> {
    // Honest: Return empty list when no OS digital identity / Keychain certificate is enrolled
    Ok(Vec::new())
}

// ===== TIMESTAMP & VALIDATION =====

#[derive(serde::Serialize)]
pub struct TimestampResult {
    pub timestamp: String,
    pub authority: String,
    pub valid: bool,
    pub hash: String,
}

// Add timestamp to PDF
pub fn add_timestamp(_data: &[u8], _timestamp_authority: &str) -> Result<Vec<u8>, String> {
    Err("RFC 3161 タイムスタンプ局(TSA)への暗号化通信モジュールは現在未設定です。".into())
}

// Verify timestamp
pub fn verify_timestamp(data: &[u8]) -> Result<TimestampResult, String> {
    let doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;

    // Look for timestamp in document
    let mut timestamp = None;
    for (_, obj) in &doc.objects {
        if let Object::Dictionary(dict) = obj {
            if let Ok(Object::Name(name)) = dict.get(b"Type") {
                if name == b"DocTimeStamp" {
                    if let Ok(Object::String(ts, _)) = dict.get(b"M") {
                        timestamp = Some(String::from_utf8_lossy(ts).to_string());
                    }
                }
            }
        }
    }

    Ok(TimestampResult {
        timestamp: timestamp.unwrap_or_else(|| "DocTimeStampなし".into()),
        authority: "外部TSA未照合".into(),
        valid: false, // Honest: Cannot assert validity without RFC 3161 cryptographic verification
        hash: String::new(),
    })
}

// ===== CERTIFICATE STORE INTEGRATION =====

#[derive(serde::Serialize)]
pub struct Certificate {
    pub subject: String,
    pub issuer: String,
    pub valid_from: String,
    pub valid_to: String,
    pub serial: String,
    pub key_usage: Vec<String>,
}

// List system certificates
pub fn list_certificates() -> Result<Vec<Certificate>, String> {
    // Honest: No mock certificates returned
    Ok(Vec::new())
}

// Import certificate from file
pub fn import_certificate(cert_path: &str) -> Result<Certificate, String> {
    let _cert_data =
        std::fs::read(cert_path).map_err(|e| format!("証明書ファイルの読み込みに失敗しました: {e}"))?;

    Err("X.509証明書の完全なDER/PEMパースおよび暗号鍵インポートには外部ASN.1/PKIライブラリの連携が必要です。".into())
}
