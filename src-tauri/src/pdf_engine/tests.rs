#[cfg(test)]
mod tests {
    use crate::pdf_engine::*;
    use lopdf::{Dictionary, Document, Object};

    fn find_tool(name: &str) -> Option<std::path::PathBuf> {
        // 1. Check PATH env variable
        if let Some(paths) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&paths) {
                let exe_name = if cfg!(windows) {
                    format!("{}.exe", name)
                } else {
                    name.to_string()
                };
                let full_path = dir.join(&exe_name);
                if full_path.is_file() {
                    return Some(full_path);
                }
            }
        }
        // 2. Check standard installation directories
        let common_dirs = [
            "/usr/bin",
            "/usr/local/bin",
            "/opt/homebrew/bin",
            "/bin",
            "/snap/bin",
        ];
        for dir in &common_dirs {
            let p = std::path::Path::new(dir).join(name);
            if p.is_file() {
                return Some(p);
            }
        }
        None
    }

    fn create_test_pdf(num_pages: usize) -> Vec<u8> {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.add_object(Object::Dictionary(Dictionary::new()));
        let mut kids = Vec::new();

        for i in 0..num_pages {
            let content = format!("BT /F1 12 Tf 50 750 Td (Page {}) Tj ET", i + 1);
            let content_id = doc.add_object(Object::Stream(lopdf::Stream::new(
                Dictionary::new(),
                content.into_bytes(),
            )));

            let mut f1_dict = Dictionary::new();
            f1_dict.set("Type", Object::Name("Font".into()));
            f1_dict.set("Subtype", Object::Name("Type1".into()));
            f1_dict.set("BaseFont", Object::Name("Helvetica".into()));
            let f1_id = doc.add_object(Object::Dictionary(f1_dict));

            let mut font_dict = Dictionary::new();
            font_dict.set("F1", Object::Reference(f1_id));

            let mut res_dict = Dictionary::new();
            res_dict.set("Font", Object::Dictionary(font_dict));

            let mut page_dict = Dictionary::new();
            page_dict.set("Type", Object::Name("Page".into()));
            page_dict.set("Parent", Object::Reference(pages_id));
            page_dict.set(
                "MediaBox",
                Object::Array(vec![
                    Object::Real(0.0),
                    Object::Real(0.0),
                    Object::Real(595.0),
                    Object::Real(842.0),
                ]),
            );
            page_dict.set("Resources", Object::Dictionary(res_dict));
            page_dict.set("Contents", Object::Reference(content_id));

            let page_id = doc.add_object(Object::Dictionary(page_dict));
            kids.push(Object::Reference(page_id));
        }

        let mut pages_dict = Dictionary::new();
        pages_dict.set("Type", Object::Name("Pages".into()));
        pages_dict.set("Kids", Object::Array(kids));
        pages_dict.set("Count", Object::Integer(num_pages as i64));
        doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

        let mut root_dict = Dictionary::new();
        root_dict.set("Type", Object::Name("Catalog".into()));
        root_dict.set("Pages", Object::Reference(pages_id));
        let root_id = doc.add_object(Object::Dictionary(root_dict));
        doc.trailer.set("Root", Object::Reference(root_id));

        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    #[test]
    fn test_page_count_and_inspection() {
        let pdf = create_test_pdf(3);
        let count = get_page_count_from_data(&pdf).expect("Page count should succeed");
        assert_eq!(count, 3);
    }

    #[test]
    fn test_page_deletion_and_rotation() {
        let pdf = create_test_pdf(3);
        let rotated = rotate_page(&pdf, 0, 90).expect("Rotate should succeed");
        assert!(!rotated.is_empty());

        let deleted = delete_page(&pdf, 1).expect("Delete should succeed");
        let new_count = get_page_count_from_data(&deleted).expect("Get new count");
        assert_eq!(new_count, 2);
    }

    #[test]
    fn test_empty_or_corrupt_recovery() {
        let corrupt_data = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Count 0 /Kids [] >>\nendobj\nxref\n0 3\n0000000000 65535 f \n0000009999 00000 n \n0000009999 00000 n \ntrailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n999999\n%%EOF";
        let repaired =
            repair_corrupt_pdf(corrupt_data).expect("Repair should salvage catalog/pages");
        assert!(!repaired.is_empty());
    }

    #[test]
    fn test_document_comparison() {
        let doc1 = create_test_pdf(1);
        let doc2 = create_test_pdf(1);
        let report = compare_pdf_documents(&doc1, &doc2).expect("Compare should succeed");
        assert_eq!(report.total_changes, 0);
    }

    #[test]
    fn test_pdf_x_conversion_and_validation() {
        let pdf = create_test_pdf(2);
        let pdfx = convert_to_pdfx_standard(&pdf, "PDF/X-1a", "Japan Color 2001 Coated")
            .expect("Convert to PDF/X-1a should succeed");
        let validation =
            validate_pdfx_compliance(&pdfx, "PDF/X-1a").expect("Validation should succeed");
        assert!(
            validation.is_compliant,
            "PDF/X-1a converted document should be compliant"
        );
    }

    #[test]
    fn test_page_numbers_injection() {
        let pdf = create_test_pdf(2);
        let numbered = add_page_numbers(&pdf, "bottom-center", 10.0, 1).expect("Add page numbers");
        assert!(!numbered.is_empty());
        let count = get_page_count_from_data(&numbered).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_redact_area_preserves_other_content() {
        let pdf = create_test_pdf(1);
        // Initial PDF contains "Page 1"
        let initial_doc = Document::load_mem(&pdf).expect("Load initial PDF");
        let initial_page_ids = get_page_ids(&initial_doc);
        let initial_contents = initial_doc
            .get_page_content(initial_page_ids[0])
            .expect("Get content");
        let initial_text = String::from_utf8_lossy(&initial_contents);
        assert!(
            initial_text.contains("Page 1"),
            "Original content must contain Page 1"
        );

        // Redact a small box at (10, 10, 50, 50), not overlapping the text at (50, 750)
        let redacted = redact_area(&pdf, 0, 10.0, 10.0, 50.0, 50.0, "#000000")
            .expect("Redact area should succeed");

        let redacted_doc = Document::load_mem(&redacted).expect("Load redacted PDF");
        let redacted_page_ids = get_page_ids(&redacted_doc);
        let redacted_contents = redacted_doc
            .get_page_content(redacted_page_ids[0])
            .expect("Get redacted content");
        let redacted_text = String::from_utf8_lossy(&redacted_contents);

        // Crucial check: the existing page content must be PRESERVED, not replaced by just a black box!
        assert!(
            redacted_text.contains("Page 1"),
            "Existing page content must be preserved after redact_area"
        );
        assert!(
            redacted_text.contains("re"),
            "Redaction rectangle must be appended"
        );
    }

    #[test]
    fn test_redact_purges_string_completely() {
        let secret = "SECRET-123456";
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.add_object(Object::Dictionary(Dictionary::new()));

        let content = format!("BT /F1 12 Tf 50 750 Td ({secret}) Tj ET");
        let content_id = doc.add_object(Object::Stream(lopdf::Stream::new(
            Dictionary::new(),
            content.into_bytes(),
        )));

        let mut page_dict = Dictionary::new();
        page_dict.set("Type", Object::Name("Page".into()));
        page_dict.set("Parent", Object::Reference(pages_id));
        page_dict.set(
            "MediaBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(595.0),
                Object::Real(842.0),
            ]),
        );
        page_dict.set("Contents", Object::Reference(content_id));

        let page_id = doc.add_object(Object::Dictionary(page_dict));

        let mut pages_dict = Dictionary::new();
        pages_dict.set("Type", Object::Name("Pages".into()));
        pages_dict.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
        pages_dict.set("Count", Object::Integer(1));
        doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

        let mut root_dict = Dictionary::new();
        root_dict.set("Type", Object::Name("Catalog".into()));
        root_dict.set("Pages", Object::Reference(pages_id));
        let root_id = doc.add_object(Object::Dictionary(root_dict));
        doc.trailer.set("Root", Object::Reference(root_id));

        let mut initial_pdf = Vec::new();
        doc.save_to(&mut initial_pdf).unwrap();

        // 1. Verify secret is present before redaction
        assert!(
            String::from_utf8_lossy(&initial_pdf).contains(secret),
            "Secret must exist in initial raw PDF bytes"
        );

        // 2. Perform deep text redaction
        let purged_pdf =
            redact_text_deep(&initial_pdf, secret, "#000000").expect("Deep redaction must succeed");

        // 3. Byte-level verification: raw byte search
        assert!(
            !String::from_utf8_lossy(&purged_pdf).contains(secret),
            "Physical raw PDF bytes must NOT contain secret after deep redaction"
        );

        // 4. Object stream level verification: lopdf document decode
        let purged_doc = Document::load_mem(&purged_pdf).expect("Load purged PDF");
        for (_, object) in purged_doc.objects.iter() {
            if let Object::Stream(ref stream) = object {
                if let Ok(decoded) = stream.decompressed_content() {
                    assert!(
                        !String::from_utf8_lossy(&decoded).contains(secret),
                        "Decompressed stream must NOT contain secret after deep redaction"
                    );
                }
            }
        }
    }

    #[test]
    fn test_searchable_pdf_structure_and_fallback() {
        let temp_output =
            std::env::temp_dir().join(format!("searchable_test_{}.pdf", std::process::id()));
        let output_path = temp_output.to_string_lossy().to_string();

        let ocr_text = "Line 1: Scanned invoice text\nLine 2: Total amount 50000 JPY";
        crate::ocr_engine::create_searchable_pdf(&[], ocr_text, &output_path)
            .expect("Searchable PDF creation should succeed");

        let data = std::fs::read(&output_path).expect("Read output PDF");
        let _ = std::fs::remove_file(&output_path);

        let doc = Document::load_mem(&data).expect("Must parse as valid PDF");

        // Check Catalog Root exists
        let root_ref = doc.trailer.get(b"Root").expect("Must have Root in trailer");
        let root_id = root_ref.as_reference().expect("Root must be reference");
        let root_dict = doc
            .objects
            .get(&root_id)
            .and_then(|o| o.as_dict().ok())
            .expect("Root must be dict");
        assert_eq!(
            root_dict.get(b"Type").unwrap().as_name().unwrap(),
            b"Catalog"
        );

        // Check Pages dictionary
        let pages_ref = root_dict.get(b"Pages").expect("Catalog must have Pages");
        let pages_id = pages_ref.as_reference().expect("Pages must be reference");
        let pages_dict = doc
            .objects
            .get(&pages_id)
            .and_then(|o| o.as_dict().ok())
            .expect("Pages must be dict");
        assert_eq!(
            pages_dict.get(b"Type").unwrap().as_name().unwrap(),
            b"Pages"
        );

        let page_ids = get_page_ids(&doc);
        assert_eq!(page_ids.len(), 1);

        // Verify page has Parent reference and Resources with Font
        let page_dict = doc
            .objects
            .get(&page_ids[0])
            .and_then(|o| o.as_dict().ok())
            .expect("Page must be dict");
        assert_eq!(
            page_dict.get(b"Parent").unwrap().as_reference().unwrap(),
            pages_id
        );
        assert!(
            page_dict.get(b"Resources").is_ok(),
            "Page must have Resources dict"
        );
    }

    #[test]
    fn test_signature_inspection_honesty() {
        let pdf = create_test_pdf(1);
        let signed = add_digital_signature(
            &pdf,
            0,
            100.0,
            100.0,
            200.0,
            50.0,
            "Alice Developer",
            "Code Review Approval",
            None,
        )
        .expect("Add digital signature structure");

        let doc = Document::load_mem(&signed).expect("Load signed PDF");
        let result = verify_signature_in_doc(&doc).expect("Verify signature structure");

        let sigs = result["signatures"]
            .as_array()
            .expect("Must have signatures array");
        assert_eq!(sigs.len(), 1);
        let sig0 = &sigs[0];

        // Verify that fields extracted match what was inserted
        assert_eq!(sig0["signer"], "Alice Developer");
        assert_eq!(sig0["reason"], "Code Review Approval");
        // Verify honesty: Lopdf must NOT claim cryptographic validity or fake issuer
        assert_eq!(sig0["status"], "unverified_structure_only");
        assert_eq!(sig0["integrity_verified"], false);
        assert_eq!(sig0["aatl_verified"], false);
    }

    #[test]
    fn test_unlock_encrypted_pdf_rejection() {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.add_object(Object::Dictionary(Dictionary::new()));
        let mut root_dict = Dictionary::new();
        root_dict.set("Type", Object::Name("Catalog".into()));
        root_dict.set("Pages", Object::Reference(pages_id));
        let root_id = doc.add_object(Object::Dictionary(root_dict));
        doc.trailer.set("Root", Object::Reference(root_id));

        // Insert fake /Encrypt dictionary in trailer
        let mut encrypt_dict = Dictionary::new();
        encrypt_dict.set("Filter", Object::Name("Standard".into()));
        encrypt_dict.set("V", Object::Integer(2));
        encrypt_dict.set("R", Object::Integer(3));
        let encrypt_id = doc.add_object(Object::Dictionary(encrypt_dict));
        doc.trailer.set("Encrypt", Object::Reference(encrypt_id));

        let mut encrypted_pdf_bytes = Vec::new();
        doc.save_to(&mut encrypted_pdf_bytes).unwrap();

        // Calling unlock_pdf MUST return an Err refusing to corrupt the PDF
        let unlock_result = unlock_pdf(&encrypted_pdf_bytes, "secret");
        assert!(
            unlock_result.is_err(),
            "unlock_pdf must reject blind trailer stripping when Encrypt dictionary is present"
        );
    }

    #[test]
    fn test_redact_text_replaces_stream_content() {
        let text_to_replace = "CONFIDENTIAL-DATA";
        let replacement = "REDACTED";

        let mut doc = Document::with_version("1.7");
        let pages_id = doc.add_object(Object::Dictionary(Dictionary::new()));
        let content = format!("BT /F1 12 Tf 50 750 Td ({text_to_replace}) Tj ET");
        let content_id = doc.add_object(Object::Stream(lopdf::Stream::new(
            Dictionary::new(),
            content.into_bytes(),
        )));

        let mut page_dict = Dictionary::new();
        page_dict.set("Type", Object::Name("Page".into()));
        page_dict.set("Parent", Object::Reference(pages_id));
        page_dict.set(
            "MediaBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(595.0),
                Object::Real(842.0),
            ]),
        );
        page_dict.set("Contents", Object::Reference(content_id));
        let page_id = doc.add_object(Object::Dictionary(page_dict));

        let mut pages_dict = Dictionary::new();
        pages_dict.set("Type", Object::Name("Pages".into()));
        pages_dict.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
        pages_dict.set("Count", Object::Integer(1));
        doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

        let mut root_dict = Dictionary::new();
        root_dict.set("Type", Object::Name("Catalog".into()));
        root_dict.set("Pages", Object::Reference(pages_id));
        let root_id = doc.add_object(Object::Dictionary(root_dict));
        doc.trailer.set("Root", Object::Reference(root_id));

        let mut pdf_data = Vec::new();
        doc.save_to(&mut pdf_data).unwrap();

        let redacted = redact_text(&pdf_data, text_to_replace, replacement)
            .expect("redact_text should replace content in body stream");

        assert!(
            !String::from_utf8_lossy(&redacted).contains(text_to_replace),
            "Original text must no longer be present"
        );
        assert!(
            String::from_utf8_lossy(&redacted).contains(replacement),
            "Replacement text must be present"
        );
    }

    #[test]
    fn test_redact_text_compressed_stream() {
        let text_to_replace = "TOP-SECRET-DEFLATE";
        let replacement = "PURGED";

        let mut doc = Document::with_version("1.7");
        let pages_id = doc.add_object(Object::Dictionary(Dictionary::new()));
        let content = format!("BT /F1 12 Tf 50 750 Td ({text_to_replace}) Tj ET");

        // Compress content stream with FlateDecode
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(content.as_bytes()).unwrap();
        let compressed_bytes = encoder.finish().unwrap();

        let mut stream_dict = Dictionary::new();
        stream_dict.set("Filter", Object::Name("FlateDecode".into()));
        let stream = lopdf::Stream::new(stream_dict, compressed_bytes);
        assert_eq!(
            stream.dict.get(b"Filter").unwrap().as_name().unwrap(),
            b"FlateDecode"
        );
        let content_id = doc.add_object(Object::Stream(stream));

        let mut page_dict = Dictionary::new();
        page_dict.set("Type", Object::Name("Page".into()));
        page_dict.set("Parent", Object::Reference(pages_id));
        page_dict.set(
            "MediaBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(595.0),
                Object::Real(842.0),
            ]),
        );
        page_dict.set("Contents", Object::Reference(content_id));
        let page_id = doc.add_object(Object::Dictionary(page_dict));

        let mut pages_dict = Dictionary::new();
        pages_dict.set("Type", Object::Name("Pages".into()));
        pages_dict.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
        pages_dict.set("Count", Object::Integer(1));
        doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

        let mut root_dict = Dictionary::new();
        root_dict.set("Type", Object::Name("Catalog".into()));
        root_dict.set("Pages", Object::Reference(pages_id));
        let root_id = doc.add_object(Object::Dictionary(root_dict));
        doc.trailer.set("Root", Object::Reference(root_id));

        let mut pdf_data = Vec::new();
        doc.save_to(&mut pdf_data).unwrap();

        // Perform redact_text on FlateDecode-compressed stream
        let redacted = redact_text(&pdf_data, text_to_replace, replacement)
            .expect("redact_text must succeed even on compressed stream");

        let res_doc = Document::load_mem(&redacted).expect("Parse output PDF");
        let pids = get_page_ids(&res_doc);
        let content_bytes = res_doc.get_page_content(pids[0]).expect("Get page content");
        let decompressed = String::from_utf8_lossy(&content_bytes);

        assert!(
            !decompressed.contains(text_to_replace),
            "Decompressed stream must not contain target text"
        );
        assert!(
            decompressed.contains(replacement),
            "Decompressed stream must contain replacement text"
        );
    }

    #[test]
    fn test_empty_mock_security_responses() {
        // Confirm no fake digital IDs or fake certificates are returned
        let ids = list_digital_ids().expect("list_digital_ids must succeed");
        assert!(
            ids.is_empty(),
            "list_digital_ids must be empty when no certificates enrolled"
        );

        let certs = list_certificates().expect("list_certificates must succeed");
        assert!(certs.is_empty(), "list_certificates must be empty");

        let ts_res = add_timestamp(&[], "http://tsa.example.com");
        assert!(
            ts_res.is_err(),
            "add_timestamp must reject without real TSA connection"
        );
    }

    #[test]
    fn test_ink_coverage_300_percent_threshold() {
        // Create a PDF with CMYK paint operators
        // Case 1: C=0.5, M=0.5, Y=0.5, K=0.5 -> Total 2.0 (200%), must NOT warn
        let mut doc1 = Document::with_version("1.7");
        let content1 = b"0.5 0.5 0.5 0.5 k 0 0 100 100 re f";
        let stream1 = lopdf::Stream::new(Dictionary::new(), content1.to_vec());
        let s_id1 = doc1.add_object(Object::Stream(stream1));

        let mut page1 = Dictionary::new();
        page1.set("Type", Object::Name("Page".into()));
        page1.set("Contents", Object::Reference(s_id1));
        page1.set(
            "MediaBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(595.0),
                Object::Real(842.0),
            ]),
        );
        let p_id1 = doc1.add_object(Object::Dictionary(page1));

        let mut pages1 = Dictionary::new();
        pages1.set("Type", Object::Name("Pages".into()));
        pages1.set("Kids", Object::Array(vec![Object::Reference(p_id1)]));
        pages1.set("Count", Object::Integer(1));
        let pages_id1 = doc1.add_object(Object::Dictionary(pages1));

        let mut cat1 = Dictionary::new();
        cat1.set("Type", Object::Name("Catalog".into()));
        cat1.set("Pages", Object::Reference(pages_id1));
        let cat_id1 = doc1.add_object(Object::Dictionary(cat1));
        doc1.trailer.set("Root", Object::Reference(cat_id1));

        let mut pdf_bytes1 = Vec::new();
        doc1.save_to(&mut pdf_bytes1).unwrap();

        let res1 = check_ink_coverage(&pdf_bytes1, 0).expect("check_ink_coverage page 0");
        assert_eq!(
            res1["warning"], false,
            "200% ink coverage must not trigger >300% warning: {:?}",
            res1
        );
        assert!((res1["max_coverage"].as_f64().unwrap() - 200.0).abs() < 1e-2);

        // Case 2: C=0.9, M=0.9, Y=0.8, K=0.6 -> Total 3.2 (320%), MUST warn
        let mut doc2 = Document::with_version("1.7");
        let content2 = b"0.9 0.9 0.8 0.6 k 0 0 100 100 re f";
        let stream2 = lopdf::Stream::new(Dictionary::new(), content2.to_vec());
        let s_id2 = doc2.add_object(Object::Stream(stream2));

        let mut page2 = Dictionary::new();
        page2.set("Type", Object::Name("Page".into()));
        page2.set("Contents", Object::Reference(s_id2));
        page2.set(
            "MediaBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(595.0),
                Object::Real(842.0),
            ]),
        );
        let p_id2 = doc2.add_object(Object::Dictionary(page2));

        let mut pages2 = Dictionary::new();
        pages2.set("Type", Object::Name("Pages".into()));
        pages2.set("Kids", Object::Array(vec![Object::Reference(p_id2)]));
        pages2.set("Count", Object::Integer(1));
        let pages_id2 = doc2.add_object(Object::Dictionary(pages2));

        let mut cat2 = Dictionary::new();
        cat2.set("Type", Object::Name("Catalog".into()));
        cat2.set("Pages", Object::Reference(pages_id2));
        let cat_id2 = doc2.add_object(Object::Dictionary(cat2));
        doc2.trailer.set("Root", Object::Reference(cat_id2));

        let mut pdf_bytes2 = Vec::new();
        doc2.save_to(&mut pdf_bytes2).unwrap();

        let res2 = check_ink_coverage(&pdf_bytes2, 0).expect("check_ink_coverage page 0");
        assert_eq!(
            res2["warning"], true,
            "320% ink coverage must trigger >300% warning: {:?}",
            res2
        );
    }

    #[test]
    fn test_add_page_numbers_on_flatedecode_stream() {
        // Construct a PDF with a compressed FlateDecode content stream
        let mut doc = Document::with_version("1.7");
        let raw_stream = b"BT /F1 12 Tf 50 700 Td (Initial Text) Tj ET";
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut encoder, raw_stream).unwrap();
        let compressed_bytes = encoder.finish().unwrap();

        let mut stream = lopdf::Stream::new(Dictionary::new(), compressed_bytes);
        stream
            .dict
            .set("Filter", Object::Name("FlateDecode".into()));
        let s_id = doc.add_object(Object::Stream(stream));

        let mut page = Dictionary::new();
        page.set("Type", Object::Name("Page".into()));
        page.set("Contents", Object::Reference(s_id));
        page.set(
            "MediaBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(595.0),
                Object::Real(842.0),
            ]),
        );
        let p_id = doc.add_object(Object::Dictionary(page));

        let mut pages = Dictionary::new();
        pages.set("Type", Object::Name("Pages".into()));
        pages.set("Kids", Object::Array(vec![Object::Reference(p_id)]));
        pages.set("Count", Object::Integer(1));
        let pages_id = doc.add_object(Object::Dictionary(pages));

        let mut cat = Dictionary::new();
        cat.set("Type", Object::Name("Catalog".into()));
        cat.set("Pages", Object::Reference(pages_id));
        let cat_id = doc.add_object(Object::Dictionary(cat));
        doc.trailer.set("Root", Object::Reference(cat_id));

        let mut pdf_data = Vec::new();
        doc.save_to(&mut pdf_data).unwrap();

        // Add page numbers
        let numbered = add_page_numbers(&pdf_data, "bottom-center", 12.0, 1)
            .expect("add_page_numbers must succeed");

        // Inspect output PDF: Contents should now be an array and original stream intact
        let out_doc = Document::load_mem(&numbered).expect("Load numbered PDF");
        let out_page = out_doc.get_dictionary(p_id).expect("Get page dict");

        // Contents must be Array or separate stream, never raw append to Flate stream
        let contents_obj = out_page.get(b"Contents").expect("Contents entry exists");
        match contents_obj {
            Object::Array(arr) => {
                assert_eq!(
                    arr.len(),
                    2,
                    "Contents should have 2 streams: original and new page number stream"
                );
            }
            _ => panic!("Expected Contents to be an Array of streams"),
        }

        // Font resource for Helvetica must be present in Resources
        let resources = out_page.get(b"Resources").expect("Resources exist");
        let res_dict = resources.as_dict().expect("Resources dict");
        assert!(
            res_dict.get(b"Font").is_ok(),
            "Font dict must be present in Resources"
        );
    }

    #[test]
    fn test_repair_pdf_creates_valid_catalog() {
        let corrupt_data = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Count 0 /Kids [] >>\nendobj\nxref\n0 3\n0000000000 65535 f \n0000009999 00000 n \n0000009999 00000 n \ntrailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n999999\n%%EOF";
        let repaired = repair_pdf(corrupt_data).expect("repair_pdf must succeed");
        let repaired_doc = Document::load_mem(&repaired).expect("Repaired doc must parse");

        let root_ref = repaired_doc.trailer.get(b"Root").expect("Root in trailer");
        let root_id = root_ref.as_reference().expect("Root is reference");
        let root_dict = repaired_doc
            .get_dictionary(root_id)
            .expect("Catalog dictionary");
        assert_eq!(
            root_dict.get(b"Type").unwrap().as_name().unwrap(),
            b"Catalog",
            "Root must point to a Catalog dictionary, not a Page!"
        );
    }

    #[test]
    fn test_xfdf_xml_escaping_and_unescaping() {
        let pdf = create_test_pdf(1);
        let comment_text = "Review & approval <urgent> \"2026\" 'important'";

        // Add sticky note annotation with special XML characters
        let with_annot = add_sticky_note(&pdf, 0, 100.0, 100.0, comment_text, "#FF0000")
            .expect("add_sticky_note");

        let exported_xfdf = export_xfdf(&with_annot).expect("export_xfdf");
        assert!(
            exported_xfdf.contains("&amp;"),
            "XML must escape '&' to '&amp;':\n{}",
            exported_xfdf
        );
        assert!(
            exported_xfdf.contains("&lt;urgent&gt;"),
            "XML must escape '<' and '>':\n{}",
            exported_xfdf
        );
        assert!(
            exported_xfdf.contains("&quot;"),
            "XML must escape quotes:\n{}",
            exported_xfdf
        );

        // Import back and verify unescaping and position preservation
        let imported_pdf = import_xfdf(&pdf, &exported_xfdf).expect("import_xfdf");
        let annots = get_annotations(&imported_pdf).expect("get_annotations");
        assert!(!annots.is_empty());
        let imported_c = annots[0]["contents"].as_str().unwrap();
        assert_eq!(
            imported_c, comment_text,
            "Imported contents must match unescaped text"
        );
        let imported_x = annots[0]["x"].as_f64().unwrap();
        let imported_y = annots[0]["y"].as_f64().unwrap();
        assert_eq!(
            imported_x, 100.0,
            "Imported annotation X must match exported left"
        );
        assert_eq!(
            imported_y, 100.0,
            "Imported annotation Y must match exported top"
        );
    }

    #[test]
    fn test_add_page_numbers_preserves_indirect_and_inherited_resources() {
        let mut doc = Document::with_version("1.7");

        // Create an indirect Resources dictionary containing an existing font /F1 and XObject
        let mut font_f1 = Dictionary::new();
        font_f1.set("Type", Object::Name(b"Font".to_vec()));
        font_f1.set("Subtype", Object::Name(b"Type1".to_vec()));
        font_f1.set("BaseFont", Object::Name(b"Times-Roman".to_vec()));
        let f1_id = doc.add_object(Object::Dictionary(font_f1));

        let mut indirect_res = Dictionary::new();
        let mut f_dict = Dictionary::new();
        f_dict.set("F1", Object::Reference(f1_id));
        indirect_res.set("Font", Object::Dictionary(f_dict));
        let res_id = doc.add_object(Object::Dictionary(indirect_res));

        // Create a page referencing Resources indirectly via Reference
        let mut page = Dictionary::new();
        page.set("Type", Object::Name("Page".into()));
        page.set("Resources", Object::Reference(res_id));
        page.set(
            "MediaBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(595.0),
                Object::Real(842.0),
            ]),
        );
        let p_id = doc.add_object(Object::Dictionary(page));

        let mut pages = Dictionary::new();
        pages.set("Type", Object::Name("Pages".into()));
        pages.set("Kids", Object::Array(vec![Object::Reference(p_id)]));
        pages.set("Count", Object::Integer(1));
        let pages_id = doc.add_object(Object::Dictionary(pages));

        let mut cat = Dictionary::new();
        cat.set("Type", Object::Name("Catalog".into()));
        cat.set("Pages", Object::Reference(pages_id));
        let cat_id = doc.add_object(Object::Dictionary(cat));
        doc.trailer.set("Root", Object::Reference(cat_id));

        let mut pdf_data = Vec::new();
        doc.save_to(&mut pdf_data).unwrap();

        let numbered =
            add_page_numbers(&pdf_data, "bottom-center", 10.0, 1).expect("add_page_numbers");
        let out_doc = Document::load_mem(&numbered).expect("load numbered doc");
        let page_dict = out_doc.get_dictionary(p_id).expect("get page");
        let res = page_dict.get(b"Resources").expect("Resources entry");
        let res_dict = match res {
            Object::Dictionary(d) => d,
            Object::Reference(id) => out_doc.get_dictionary(*id).expect("get indirect dict"),
            _ => panic!("Expected dictionary or reference"),
        };
        let font_dict = res_dict
            .get(b"Font")
            .expect("Font subdict")
            .as_dict()
            .expect("Font dict");
        assert!(
            font_dict.get(b"F1").is_ok(),
            "Pre-existing F1 font must NOT be wiped out!"
        );
        assert!(
            font_dict.get(b"DocForgeHelv").is_ok(),
            "DocForgeHelv font must be added!"
        );
    }

    #[test]
    fn test_sanitize_document_purges_names_tree() {
        let mut doc = Document::with_version("1.7");

        // Build /Names -> /JavaScript and /EmbeddedFiles
        let mut js_dict = Dictionary::new();
        js_dict.set(
            "Names",
            Object::Array(vec![Object::String(
                b"TestJS".to_vec(),
                lopdf::StringFormat::Literal,
            )]),
        );
        let js_id = doc.add_object(Object::Dictionary(js_dict));

        let mut ef_dict = Dictionary::new();
        ef_dict.set(
            "Names",
            Object::Array(vec![Object::String(
                b"Malware.exe".to_vec(),
                lopdf::StringFormat::Literal,
            )]),
        );
        let ef_id = doc.add_object(Object::Dictionary(ef_dict));

        let mut names_dict = Dictionary::new();
        names_dict.set("JavaScript", Object::Reference(js_id));
        names_dict.set("EmbeddedFiles", Object::Reference(ef_id));
        let names_id = doc.add_object(Object::Dictionary(names_dict));

        let mut page = Dictionary::new();
        page.set("Type", Object::Name("Page".into()));
        page.set(
            "MediaBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(595.0),
                Object::Real(842.0),
            ]),
        );
        let p_id = doc.add_object(Object::Dictionary(page));

        let mut pages = Dictionary::new();
        pages.set("Type", Object::Name("Pages".into()));
        pages.set("Kids", Object::Array(vec![Object::Reference(p_id)]));
        pages.set("Count", Object::Integer(1));
        let pages_id = doc.add_object(Object::Dictionary(pages));

        let mut cat = Dictionary::new();
        cat.set("Type", Object::Name("Catalog".into()));
        cat.set("Pages", Object::Reference(pages_id));
        cat.set("Names", Object::Reference(names_id));
        let cat_id = doc.add_object(Object::Dictionary(cat));
        doc.trailer.set("Root", Object::Reference(cat_id));

        let mut pdf_data = Vec::new();
        doc.save_to(&mut pdf_data).unwrap();

        let (sanitized_bytes, summary) = sanitize_document(&pdf_data).expect("sanitize_document");
        assert!(
            summary.javascript_removed,
            "Must report javascript removed from Names tree"
        );
        assert!(
            summary.attachments_removed > 0,
            "Must report attachments removed from Names tree"
        );

        let clean_doc = Document::load_mem(&sanitized_bytes).expect("load clean doc");
        let root = clean_doc
            .trailer
            .get(b"Root")
            .unwrap()
            .as_reference()
            .unwrap();
        let root_dict = clean_doc.get_dictionary(root).unwrap();

        if let Ok(n_ref) = root_dict.get(b"Names").and_then(|o| o.as_reference()) {
            let n_dict = clean_doc.get_dictionary(n_ref).unwrap();
            assert!(
                !n_dict.has(b"JavaScript"),
                "Names.JavaScript must be removed!"
            );
            assert!(
                !n_dict.has(b"EmbeddedFiles"),
                "Names.EmbeddedFiles must be removed!"
            );
        }
    }

    #[test]
    fn test_deep_redact_physical_image_raster_eradication() {
        // Create an image with known pixels (e.g. 100x100 all white 255)
        let mut img = image::RgbImage::new(100, 100);
        for pixel in img.pixels_mut() {
            *pixel = image::Rgb([255, 255, 255]);
        }
        let temp_img_path =
            std::env::temp_dir().join(format!("redact_test_img_{}.png", std::process::id()));
        img.save(&temp_img_path).expect("Save test image");

        let temp_pdf_path =
            std::env::temp_dir().join(format!("redact_test_pdf_{}.pdf", std::process::id()));
        crate::pdf_engine::convert::images_to_pdf(
            &[temp_img_path.to_string_lossy().to_string()],
            &temp_pdf_path.to_string_lossy().to_string(),
        )
        .expect("Create PDF with image");

        let initial_pdf = std::fs::read(&temp_pdf_path).expect("Read test PDF");
        let _ = std::fs::remove_file(&temp_img_path);
        let _ = std::fs::remove_file(&temp_pdf_path);

        let _initial_doc = Document::load_mem(&initial_pdf).expect("Load initial PDF");

        // Perform deep redaction on page 0 overlapping part of the image
        // Placed width = 100 * 72 / 96 = 75 pt, height = 75 pt
        // Redact rectangle [10, 10, 30, 30] in black #000000
        let redacted_pdf = deep_redact(&initial_pdf, 0, 10.0, 10.0, 30.0, 30.0, "#000000")
            .expect("Deep redaction must succeed");

        let redacted_doc = Document::load_mem(&redacted_pdf).expect("Load redacted PDF");
        // Inspect the image stream in the redacted doc
        let mut found_modified_image = false;
        for (_id, obj) in redacted_doc.objects.iter() {
            if let Object::Stream(stream) = obj {
                let subtype = stream
                    .dict
                    .get(b"Subtype")
                    .ok()
                    .and_then(|s| s.as_name().ok());
                if subtype == Some(b"Image") {
                    let decoded_bytes = stream
                        .decompressed_content()
                        .unwrap_or_else(|_| stream.content.clone());
                    let load_res = image::load_from_memory(&decoded_bytes)
                        .or_else(|_| image::load_from_memory(&stream.content));
                    if let Ok(dyn_img) = load_res {
                        let rgb = dyn_img.to_rgb8();
                        let has_black = rgb.pixels().any(|p| p[0] == 0 && p[1] == 0 && p[2] == 0);
                        assert!(
                            has_black,
                            "Image raster must physically contain erased solid black pixels!"
                        );
                        found_modified_image = true;
                    }
                }
            }
        }
        assert!(
            found_modified_image,
            "Must find and verify the redacted image XObject"
        );
    }

    #[test]
    fn test_pdf_x_embeds_real_icc_profile_stream() {
        let mut doc = Document::with_version("1.6");
        let mut page = Dictionary::new();
        page.set("Type", Object::Name("Page".into()));
        page.set(
            "MediaBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(595.0),
                Object::Real(842.0),
            ]),
        );
        let p_id = doc.add_object(Object::Dictionary(page));

        let mut pages = Dictionary::new();
        pages.set("Type", Object::Name("Pages".into()));
        pages.set("Kids", Object::Array(vec![Object::Reference(p_id)]));
        pages.set("Count", Object::Integer(1));
        let pages_id = doc.add_object(Object::Dictionary(pages));

        let mut cat = Dictionary::new();
        cat.set("Type", Object::Name("Catalog".into()));
        cat.set("Pages", Object::Reference(pages_id));
        let cat_id = doc.add_object(Object::Dictionary(cat));
        doc.trailer.set("Root", Object::Reference(cat_id));

        let mut pdf_data = Vec::new();
        doc.save_to(&mut pdf_data).unwrap();

        // Convert to PDF/X-4
        let pdfx_bytes = crate::pdf_engine::pdf_x::convert_to_pdfx_standard(
            &pdf_data,
            "PDF/X-4",
            "Japan Color 2001 Coated",
        )
        .expect("convert_to_pdfx_standard");

        let pdfx_doc = Document::load_mem(&pdfx_bytes).expect("Load converted PDF/X");
        let root = pdfx_doc
            .trailer
            .get(b"Root")
            .unwrap()
            .as_reference()
            .unwrap();
        let root_dict = pdfx_doc.get_dictionary(root).unwrap();

        let intents = root_dict.get(b"OutputIntents").unwrap().as_array().unwrap();
        assert!(!intents.is_empty(), "Must have OutputIntents");
        let intent_ref = intents[0].as_reference().unwrap();
        let intent_dict = pdfx_doc.get_dictionary(intent_ref).unwrap();

        assert_eq!(
            intent_dict.get(b"S").unwrap().as_name().unwrap(),
            b"GTS_PDFX"
        );
        let dest_prof_ref = intent_dict
            .get(b"DestOutputProfile")
            .unwrap()
            .as_reference()
            .unwrap();

        // Verify DestOutputProfile is an actual Stream with /N 4
        let stream = pdfx_doc
            .get_object(dest_prof_ref)
            .unwrap()
            .as_stream()
            .unwrap();
        assert_eq!(stream.dict.get(b"N").unwrap().as_i64().unwrap(), 4);
        assert!(
            !stream.content.is_empty(),
            "ICC profile stream content must not be empty"
        );

        // Validate via validate_pdfx_compliance
        let report = crate::pdf_engine::pdf_x::validate_pdfx_compliance(&pdfx_bytes, "PDF/X-4")
            .expect("validate_pdfx_compliance");
        assert!(
            report.is_compliant,
            "PDF/X-4 must pass preflight compliance check"
        );
        assert!(
            report
                .passed_checks
                .iter()
                .any(|c| c.contains("DestOutputProfile")),
            "Passed checks must report embedded DestOutputProfile ICC stream"
        );
    }

    #[test]
    fn test_tsv_geometry_parsing() {
        let sample_tsv = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n\
1\t1\t0\t0\t0\t0\t0\t0\t500\t800\t-1\t\n\
5\t1\t1\t1\t1\t1\t50\t100\t80\t20\t95\tDocForge\n\
5\t1\t1\t1\t1\t2\t140\t100\t60\t20\t92\tSuite";

        let (text, avg_conf, suspects, words) = crate::ocr_engine::parse_tsv_words(sample_tsv);
        assert_eq!(text.trim(), "DocForge Suite");
        assert!(avg_conf > 90.0);
        assert_eq!(suspects.len(), 0);
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "DocForge");
        assert_eq!(words[0].left, 50.0);
        assert_eq!(words[0].top, 100.0);
        assert_eq!(words[0].width, 80.0);
        assert_eq!(words[0].height, 20.0);
        assert_eq!(words[1].text, "Suite");
    }

    #[test]
    fn test_phase1_contents_preservation_across_all_ops() {
        // Build test PDF with initial text and vector shapes
        let initial_pdf = create_test_pdf(1);

        // 1. add_text should preserve existing page contents
        let with_text = add_text(&initial_pdf, 0, "Added Label", 50.0, 500.0, 14.0, "#FF0000")
            .expect("add_text must succeed");
        let doc1 = Document::load_mem(&with_text).expect("load with_text");
        let pids1 = get_page_ids(&doc1);
        let pdict1 = doc1.get_dictionary(pids1[0]).expect("page dict 1");
        let contents1 = pdict1.get(b"Contents").expect("contents 1");
        // Must be Array with 2 elements (original + added)
        assert!(matches!(contents1, Object::Array(arr) if arr.len() == 2));

        // 2. add_watermark should preserve existing page contents
        let with_wm = add_watermark(
            &with_text,
            "CONFIDENTIAL",
            0.5,
            45.0,
            30.0,
            "#888888",
            true,
            &[],
        )
        .expect("add_watermark must succeed");
        let doc2 = Document::load_mem(&with_wm).expect("load with_wm");
        let pdict2 = doc2.get_dictionary(pids1[0]).expect("page dict 2");
        let contents2 = pdict2.get(b"Contents").expect("contents 2");
        assert!(matches!(contents2, Object::Array(arr) if arr.len() == 3));

        // 3. add_header_footer should preserve existing contents
        let with_hf = add_header_footer(&with_wm, "Header {page}/{total}", "Footer", 10.0, 20.0)
            .expect("add_header_footer must succeed");
        let doc3 = Document::load_mem(&with_hf).expect("load with_hf");
        let pdict3 = doc3.get_dictionary(pids1[0]).expect("page dict 3");
        let contents3 = pdict3.get(b"Contents").expect("contents 3");
        assert!(matches!(contents3, Object::Array(arr) if arr.len() == 4));

        // 4. add_bates_number should preserve existing contents
        let with_bates = add_bates_number(&with_hf, "BATES-", 100, 10.0, 20.0)
            .expect("add_bates_number must succeed");
        let doc4 = Document::load_mem(&with_bates).expect("load with_bates");
        let pdict4 = doc4.get_dictionary(pids1[0]).expect("page dict 4");
        let contents4 = pdict4.get(b"Contents").expect("contents 4");
        assert!(matches!(contents4, Object::Array(arr) if arr.len() == 5));
    }

    #[test]
    fn test_phase1_protect_pdf_safety_rejection() {
        let initial_pdf = create_test_pdf(1);
        let res = protect_pdf(&initial_pdf, "secret_password");
        assert!(
            res.is_err(),
            "protect_pdf must refuse to emit broken pseudo-encrypted PDF"
        );
        let err_msg = res.unwrap_err();
        assert!(err_msg.contains("Standard Security Handler") || err_msg.contains("暗号化"));
    }

    #[test]
    fn test_phase2_portfolio_valid_catalog_pages_and_names() {
        let tmp_file1 = std::env::temp_dir().join("portfolio_item1.txt");
        let tmp_file2 = std::env::temp_dir().join("portfolio_item2.txt");
        std::fs::write(&tmp_file1, b"Hello file 1").unwrap();
        std::fs::write(&tmp_file2, b"Hello file 2").unwrap();

        let out_path = std::env::temp_dir().join("portfolio_test_out.pdf");
        let paths = vec![
            tmp_file1.to_string_lossy().to_string(),
            tmp_file2.to_string_lossy().to_string(),
        ];

        create_pdf_portfolio(&paths, &out_path.to_string_lossy()).expect("create_pdf_portfolio");
        let pdf_bytes = std::fs::read(&out_path).expect("read portfolio");
        let _ = std::fs::remove_file(&out_path);
        let _ = std::fs::remove_file(&tmp_file1);
        let _ = std::fs::remove_file(&tmp_file2);

        let doc = Document::load_mem(&pdf_bytes).expect("Load portfolio PDF");
        let root_ref = doc
            .trailer
            .get(b"Root")
            .expect("Root must exist")
            .as_reference()
            .unwrap();
        let root_dict = doc.get_dictionary(root_ref).expect("Catalog dictionary");

        // 1. /Type /Catalog
        assert_eq!(
            root_dict.get(b"Type").unwrap().as_name().unwrap(),
            b"Catalog"
        );

        // 2. /Pages must exist
        let pages_ref = root_dict
            .get(b"Pages")
            .expect("Pages must exist")
            .as_reference()
            .unwrap();
        let pages_dict = doc.get_dictionary(pages_ref).expect("Pages dictionary");
        assert_eq!(
            pages_dict.get(b"Type").unwrap().as_name().unwrap(),
            b"Pages"
        );

        // 3. /Collection must exist
        let coll_ref = root_dict
            .get(b"Collection")
            .expect("Collection must exist")
            .as_reference()
            .unwrap();
        let coll_dict = doc.get_dictionary(coll_ref).expect("Collection dictionary");
        assert_eq!(
            coll_dict.get(b"Type").unwrap().as_name().unwrap(),
            b"Collection"
        );

        // 4. /Names -> /EmbeddedFiles name tree
        let names_ref = root_dict
            .get(b"Names")
            .expect("Names must exist")
            .as_reference()
            .unwrap();
        let names_dict = doc.get_dictionary(names_ref).expect("Names dictionary");
        let ef_tree_ref = names_dict
            .get(b"EmbeddedFiles")
            .expect("EmbeddedFiles tree")
            .as_reference()
            .unwrap();
        let ef_tree = doc.get_dictionary(ef_tree_ref).expect("EF tree dictionary");
        let ef_names = ef_tree
            .get(b"Names")
            .expect("Names array")
            .as_array()
            .unwrap();
        assert_eq!(
            ef_names.len(),
            4,
            "2 files = 4 array items (name + filespec ref)"
        );
    }

    #[test]
    fn test_phase2_bookmark_tree_outlines_hierarchy() {
        let initial_pdf = create_test_pdf(3);

        // Add 2 bookmarks
        let bm1 = add_bookmark(&initial_pdf, "Chapter 1", 0).expect("add bookmark 1");
        let bm2 = add_bookmark(&bm1, "Chapter 2", 1).expect("add bookmark 2");

        let doc = Document::load_mem(&bm2).expect("Load bookmarked PDF");
        let root_ref = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
        let root_dict = doc.get_dictionary(root_ref).unwrap();

        let outlines_ref = root_dict
            .get(b"Outlines")
            .expect("Outlines must exist in Catalog")
            .as_reference()
            .unwrap();
        let outlines = doc
            .get_dictionary(outlines_ref)
            .expect("Outlines dictionary");
        assert_eq!(
            outlines.get(b"Type").unwrap().as_name().unwrap(),
            b"Outlines"
        );
        assert_eq!(outlines.get(b"Count").unwrap().as_i64().unwrap(), 2);

        let first_ref = outlines
            .get(b"First")
            .expect("First item")
            .as_reference()
            .unwrap();
        let last_ref = outlines
            .get(b"Last")
            .expect("Last item")
            .as_reference()
            .unwrap();
        assert_ne!(
            first_ref, last_ref,
            "Two bookmarks must have distinct First and Last"
        );

        let first_dict = doc.get_dictionary(first_ref).unwrap();
        let last_dict = doc.get_dictionary(last_ref).unwrap();

        assert_eq!(
            first_dict.get(b"Title").unwrap().as_str().unwrap(),
            b"Chapter 1"
        );
        assert_eq!(
            last_dict.get(b"Title").unwrap().as_str().unwrap(),
            b"Chapter 2"
        );

        assert_eq!(
            first_dict.get(b"Parent").unwrap().as_reference().unwrap(),
            outlines_ref
        );
        assert_eq!(
            last_dict.get(b"Parent").unwrap().as_reference().unwrap(),
            outlines_ref
        );

        assert_eq!(
            first_dict.get(b"Next").unwrap().as_reference().unwrap(),
            last_ref
        );
        assert_eq!(
            last_dict.get(b"Prev").unwrap().as_reference().unwrap(),
            first_ref
        );
    }

    #[test]
    fn test_phase2_annotation_quadpoints_generation() {
        let initial_pdf = create_test_pdf(1);

        // Add Highlight
        let highlighted = add_highlight(&initial_pdf, 0, 72.0, 700.0, 150.0, 18.0, "#FFFF00")
            .expect("add_highlight");
        let doc1 = Document::load_mem(&highlighted).expect("load highlighted");
        let pids1 = get_page_ids(&doc1);
        let pdict1 = doc1.get_dictionary(pids1[0]).unwrap();
        let annots1 = pdict1.get(b"Annots").unwrap().as_array().unwrap();
        let annot1_ref = annots1[0].as_reference().unwrap();
        let annot1_dict = doc1.get_dictionary(annot1_ref).unwrap();

        assert_eq!(
            annot1_dict.get(b"Subtype").unwrap().as_name().unwrap(),
            b"Highlight"
        );
        let qp = annot1_dict
            .get(b"QuadPoints")
            .expect("QuadPoints must exist")
            .as_array()
            .unwrap();
        assert_eq!(qp.len(), 8, "QuadPoints must be 8 numbers");

        // Add Underline
        let underlined =
            add_underline(&initial_pdf, 0, 72.0, 700.0, 150.0, "#FF0000").expect("add_underline");
        let doc2 = Document::load_mem(&underlined).expect("load underlined");
        let pdict2 = doc2.get_dictionary(pids1[0]).unwrap();
        let annots2 = pdict2.get(b"Annots").unwrap().as_array().unwrap();
        let annot2_ref = annots2[0].as_reference().unwrap();
        let annot2_dict = doc2.get_dictionary(annot2_ref).unwrap();

        assert_eq!(
            annot2_dict.get(b"Subtype").unwrap().as_name().unwrap(),
            b"Underline"
        );
        let qp2 = annot2_dict
            .get(b"QuadPoints")
            .expect("QuadPoints must exist")
            .as_array()
            .unwrap();
        assert_eq!(qp2.len(), 8, "QuadPoints must be 8 numbers");
    }

    #[test]
    fn test_phase3_unicode_font_pipeline_cjk_extraction_and_pdftotext() {
        let initial_pdf = create_test_pdf(1);

        // Mandatory regression strings:
        // 1. "これはうんちです"
        // 2. "文書作成テスト"
        // 3. "こんにちは世界"
        // 4. "PDFテスト 123 ABC"
        let strings_to_test = vec![
            "これはうんちです",
            "文書作成テスト",
            "こんにちは世界",
            "PDFテスト 123 ABC",
        ];

        let mut current_pdf = initial_pdf;
        let mut y = 650.0;
        for s in &strings_to_test {
            current_pdf = add_text(&current_pdf, 0, s, 50.0, y, 14.0, "#000000")
                .expect("add_text with Unicode string must succeed");
            y -= 40.0;
        }

        // Verify Type0 and ToUnicode CMap in PDF object graph
        let doc = Document::load_mem(&current_pdf).expect("Load Unicode PDF");
        let mut found_type0 = false;
        let mut found_tounicode_cmap = false;

        for (_, obj) in &doc.objects {
            if let Object::Dictionary(dict) = obj {
                if dict.get(b"Type").ok().and_then(|o| o.as_name().ok()) == Some(b"Font")
                    && dict.get(b"Subtype").ok().and_then(|o| o.as_name().ok()) == Some(b"Type0")
                {
                    found_type0 = true;
                    if dict.get(b"ToUnicode").is_ok() {
                        found_tounicode_cmap = true;
                    }
                }
            }
        }

        assert!(found_type0, "Must have Type0 Font in document");
        assert!(
            found_tounicode_cmap,
            "Must have ToUnicode CMap attached to Type0 Font"
        );

        // External verification via pdftotext CLI
        let tmp_pdf_path =
            std::env::temp_dir().join(format!("cjk_test_{}.pdf", std::process::id()));
        std::fs::write(&tmp_pdf_path, &current_pdf).unwrap();

        if let Some(tool) = find_tool("pdftotext") {
            let pdftotext_res = std::process::Command::new(tool)
                .arg(&tmp_pdf_path)
                .arg("-")
                .output();

            if let Ok(output) = pdftotext_res {
                if output.status.success() {
                    let extracted_text = String::from_utf8_lossy(&output.stdout);
                    for target_str in &strings_to_test {
                        assert!(
                            extracted_text.contains(target_str),
                            "pdftotext output must contain exact Unicode string '{target_str}'. Got: {extracted_text}"
                        );
                    }
                }
            }
        } else {
            eprintln!("pdftotext not found on system; skipped external text extraction assertion");
        }

        let _ = std::fs::remove_file(&tmp_pdf_path);
    }

    #[test]
    fn test_unchi_survives_save_reload_without_mutation() {
        let initial_pdf = create_test_pdf(1);
        let unchi_fixture = "これはうんちです";

        // 1. Render/add text
        let pdf_with_unchi = add_text(
            &initial_pdf,
            0,
            unchi_fixture,
            100.0,
            500.0,
            16.0,
            "#000000",
        )
        .expect("add_text with unchi fixture must succeed");

        // 2. Save to disk and reload
        let tmp_pdf_path =
            std::env::temp_dir().join(format!("unchi_fixture_{}.pdf", std::process::id()));
        std::fs::write(&tmp_pdf_path, &pdf_with_unchi).expect("Write to disk");

        let reloaded_bytes = std::fs::read(&tmp_pdf_path).expect("Reload from disk");
        let reloaded_doc = Document::load_mem(&reloaded_bytes).expect("Parse reloaded PDF");

        // Verify structure survives reload
        let pids = get_page_ids(&reloaded_doc);
        assert!(!pids.is_empty(), "Reloaded PDF must have pages");

        // 3. Extract text via external pdftotext
        if let Some(tool) = find_tool("pdftotext") {
            let pdftotext_res = std::process::Command::new(tool)
                .arg(&tmp_pdf_path)
                .arg("-")
                .output()
                .expect("pdftotext execution");

            let _ = std::fs::remove_file(&tmp_pdf_path);

            assert!(pdftotext_res.status.success(), "pdftotext must succeed");
            let extracted_text = String::from_utf8_lossy(&pdftotext_res.stdout);

            // 4. Strict assertions: must be exact fixture, not mutated
            assert!(
                extracted_text.contains(unchi_fixture),
                "Saved and reloaded PDF must contain unmutated '{unchi_fixture}'. Got: '{extracted_text}'"
            );
            assert!(
                !extracted_text.contains("ウンチ"),
                "Forbidden mutation: Katakana ウンチ detected!"
            );
            assert!(
                !extracted_text.contains("うんち "),
                "Forbidden mutation: Trailing space in うんち detected!"
            );
            assert!(
                !extracted_text.contains(" うんち"),
                "Forbidden mutation: Leading space in うんち detected!"
            );
        } else {
            let _ = std::fs::remove_file(&tmp_pdf_path);
            eprintln!("pdftotext not found on system; skipped external text extraction assertion");
        }
    }

    #[test]
    fn test_hostile_nested_page_tree_ops() {
        use crate::pdf_engine::page_tree::*;
        use lopdf::Stream;

        // Construct a hostile 2-level nested page tree with inherited attributes:
        // Root Pages (MediaBox = [0, 0, 600, 800])
        //   ├─ Intermediate Pages A (Rotate = 90, MediaBox = [0, 0, 500, 700])
        //   │    ├─ Page 1 (has Image XObject and Flate stream)
        //   │    └─ Page 2 (Contents Array of 2 streams)
        //   └─ Intermediate Pages B (Rotate = 180)
        //        ├─ Page 3 (Annotation)
        //        └─ Page 4 (plain)
        let mut doc = Document::with_version("1.7");

        // MediaBox inherited at root Pages
        let root_pages_id = doc.new_object_id();

        // Intermediate Pages A
        let pages_a_id = doc.new_object_id();
        // Intermediate Pages B
        let pages_b_id = doc.new_object_id();

        // 1. Page 1: with FlateDecode Content and Form/Image XObject
        let img_stream_bytes = vec![0xFF; 64];
        let mut img_dict = Dictionary::new();
        img_dict.set("Type", Object::Name("XObject".into()));
        img_dict.set("Subtype", Object::Name("Image".into()));
        img_dict.set("Width", Object::Integer(8));
        img_dict.set("Height", Object::Integer(8));
        img_dict.set("ColorSpace", Object::Name("DeviceRGB".into()));
        img_dict.set("BitsPerComponent", Object::Integer(8));
        let img_id = doc.add_object(Stream::new(img_dict, img_stream_bytes));

        let mut xobj_dict = Dictionary::new();
        xobj_dict.set("Im1", Object::Reference(img_id));
        let mut res1_dict = Dictionary::new();
        res1_dict.set("XObject", Object::Dictionary(xobj_dict));

        let c1 = lopdf::content::Content {
            operations: vec![
                lopdf::content::Operation::new("q", vec![]),
                lopdf::content::Operation::new("Do", vec![Object::Name("Im1".into())]),
                lopdf::content::Operation::new("Q", vec![]),
            ],
        };
        let c1_bytes = c1.encode().unwrap();
        let c1_stream = Stream::new(Dictionary::new(), c1_bytes);
        let c1_id = doc.add_object(c1_stream);

        let mut p1_dict = Dictionary::new();
        p1_dict.set("Type", Object::Name("Page".into()));
        p1_dict.set("Parent", Object::Reference(pages_a_id));
        p1_dict.set("Resources", Object::Dictionary(res1_dict));
        p1_dict.set("Contents", Object::Reference(c1_id));
        let p1_id = doc.add_object(Object::Dictionary(p1_dict));

        // 2. Page 2: Contents Array of 2 streams
        let sa = doc.add_object(Stream::new(
            Dictionary::new(),
            b"q 1 0 0 1 10 10 cm Q\n".to_vec(),
        ));
        let sb = doc.add_object(Stream::new(
            Dictionary::new(),
            b"q 1 0 0 1 20 20 cm Q\n".to_vec(),
        ));
        let mut p2_dict = Dictionary::new();
        p2_dict.set("Type", Object::Name("Page".into()));
        p2_dict.set("Parent", Object::Reference(pages_a_id));
        p2_dict.set(
            "Contents",
            Object::Array(vec![Object::Reference(sa), Object::Reference(sb)]),
        );
        let p2_id = doc.add_object(Object::Dictionary(p2_dict));

        // 3. Page 3: with Annotation
        let mut annot_dict = Dictionary::new();
        annot_dict.set("Type", Object::Name("Annot".into()));
        annot_dict.set("Subtype", Object::Name("Text".into()));
        annot_dict.set(
            "Rect",
            Object::Array(vec![
                Object::Integer(10),
                Object::Integer(10),
                Object::Integer(50),
                Object::Integer(50),
            ]),
        );
        annot_dict.set(
            "Contents",
            Object::String(b"Hostile Annotation".to_vec(), lopdf::StringFormat::Literal),
        );
        let annot_id = doc.add_object(Object::Dictionary(annot_dict));

        let mut p3_dict = Dictionary::new();
        p3_dict.set("Type", Object::Name("Page".into()));
        p3_dict.set("Parent", Object::Reference(pages_b_id));
        p3_dict.set("Annots", Object::Array(vec![Object::Reference(annot_id)]));
        let p3_id = doc.add_object(Object::Dictionary(p3_dict));

        // 4. Page 4: plain
        let mut p4_dict = Dictionary::new();
        p4_dict.set("Type", Object::Name("Page".into()));
        p4_dict.set("Parent", Object::Reference(pages_b_id));
        let p4_id = doc.add_object(Object::Dictionary(p4_dict));

        // Pages A
        let mut pa_dict = Dictionary::new();
        pa_dict.set("Type", Object::Name("Pages".into()));
        pa_dict.set("Parent", Object::Reference(root_pages_id));
        pa_dict.set(
            "Kids",
            Object::Array(vec![Object::Reference(p1_id), Object::Reference(p2_id)]),
        );
        pa_dict.set("Count", Object::Integer(2));
        pa_dict.set("Rotate", Object::Integer(90));
        pa_dict.set(
            "MediaBox",
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Integer(500),
                Object::Integer(700),
            ]),
        );
        doc.objects.insert(pages_a_id, Object::Dictionary(pa_dict));

        // Pages B
        let mut pb_dict = Dictionary::new();
        pb_dict.set("Type", Object::Name("Pages".into()));
        pb_dict.set("Parent", Object::Reference(root_pages_id));
        pb_dict.set(
            "Kids",
            Object::Array(vec![Object::Reference(p3_id), Object::Reference(p4_id)]),
        );
        pb_dict.set("Count", Object::Integer(2));
        pb_dict.set("Rotate", Object::Integer(180));
        doc.objects.insert(pages_b_id, Object::Dictionary(pb_dict));

        // Root Pages
        let mut rpages_dict = Dictionary::new();
        rpages_dict.set("Type", Object::Name("Pages".into()));
        rpages_dict.set(
            "Kids",
            Object::Array(vec![
                Object::Reference(pages_a_id),
                Object::Reference(pages_b_id),
            ]),
        );
        rpages_dict.set("Count", Object::Integer(4));
        rpages_dict.set(
            "MediaBox",
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Integer(600),
                Object::Integer(800),
            ]),
        );
        doc.objects
            .insert(root_pages_id, Object::Dictionary(rpages_dict));

        // Catalog
        let mut cat_dict = Dictionary::new();
        cat_dict.set("Type", Object::Name("Catalog".into()));
        cat_dict.set("Pages", Object::Reference(root_pages_id));
        let cat_id = doc.add_object(Object::Dictionary(cat_dict));

        doc.trailer.set("Root", Object::Reference(cat_id));

        let mut hostile_pdf_bytes = Vec::new();
        doc.save_to(&mut hostile_pdf_bytes)
            .expect("Save hostile PDF");

        // Verify logical page resolution across 2-level nested tree
        let parsed_doc = Document::load_mem(&hostile_pdf_bytes).expect("Load hostile PDF");
        let logical_ids = get_logical_page_ids(&parsed_doc);
        assert_eq!(logical_ids.len(), 4, "Must resolve 4 logical pages");
        assert_eq!(logical_ids, vec![p1_id, p2_id, p3_id, p4_id]);

        // TEST 1: Delete logical page 2 (which is p3_id in nested Pages B!)
        let after_del_bytes = delete_page(&hostile_pdf_bytes, 2).expect("Delete logical page 2");
        let del_doc = Document::load_mem(&after_del_bytes).expect("Load after delete");
        let del_page_ids = get_logical_page_ids(&del_doc);
        assert_eq!(del_page_ids.len(), 3, "Page count must now be 3");

        // TEST 2: Extract pages [0, 2] from hostile PDF (p1 with image, and p3 with annot/rotate 180)
        let extracted_bytes = extract_pages(&hostile_pdf_bytes, &[0, 2]).expect("Extract pages");
        let ext_doc = Document::load_mem(&extracted_bytes).expect("Load extracted PDF");
        let ext_page_ids = get_logical_page_ids(&ext_doc);
        assert_eq!(ext_page_ids.len(), 2, "Extracted doc must have 2 pages");

        // Verify inherited MediaBox and Rotate were materialized on extracted page 0
        let ext_p0 = ext_doc
            .objects
            .get(&ext_page_ids[0])
            .unwrap()
            .as_dict()
            .unwrap();
        assert!(
            ext_p0.get(b"MediaBox").is_ok(),
            "Extracted page 0 must have materialized MediaBox"
        );
        assert_eq!(
            ext_p0.get(b"Rotate").unwrap().as_i64().unwrap(),
            90,
            "Extracted page 0 must have materialized Rotate = 90"
        );
        // Verify XObject resource was copied over
        assert!(ext_p0.get(b"Resources").is_ok(), "Resources must be copied");

        // Verify extracted page 1 (was logical page 2 in Pages B)
        let ext_p1 = ext_doc
            .objects
            .get(&ext_page_ids[1])
            .unwrap()
            .as_dict()
            .unwrap();
        assert_eq!(
            ext_p1.get(b"Rotate").unwrap().as_i64().unwrap(),
            180,
            "Extracted page 1 must have materialized Rotate = 180"
        );
        assert!(
            ext_p1.get(b"Annots").is_ok(),
            "Annots must be preserved on extracted page"
        );

        // TEST 3: Merge hostile PDF with extracted PDF
        let merged_bytes =
            merge_pdf_buffers_robust(&[&hostile_pdf_bytes, &extracted_bytes]).expect("Merge PDFs");
        let merged_doc = Document::load_mem(&merged_bytes).expect("Load merged PDF");
        let merged_ids = get_logical_page_ids(&merged_doc);
        assert_eq!(merged_ids.len(), 6, "Merged PDF must have 4 + 2 = 6 pages");

        // Verify every page in merged document has /Parent pointing to the canonical /Pages
        let cat_ref = merged_doc
            .trailer
            .get(b"Root")
            .unwrap()
            .as_reference()
            .unwrap();
        let cat = merged_doc.objects.get(&cat_ref).unwrap().as_dict().unwrap();
        assert_eq!(cat.get(b"Type").unwrap().as_name().unwrap(), b"Catalog");
        let pages_ref = cat.get(b"Pages").unwrap().as_reference().unwrap();
        for &pid in &merged_ids {
            let pdict = merged_doc.objects.get(&pid).unwrap().as_dict().unwrap();
            assert_eq!(
                pdict.get(b"Parent").unwrap().as_reference().unwrap(),
                pages_ref,
                "Every page's /Parent must point to canonical /Pages"
            );
        }

        // TEST 4: Reorder pages on hostile PDF
        let reordered_bytes = reorder_pages(&hostile_pdf_bytes, 3, 0).expect("Reorder pages");
        let reord_doc = Document::load_mem(&reordered_bytes).expect("Load reordered PDF");
        let reord_ids = get_logical_page_ids(&reord_doc);
        assert_eq!(reord_ids.len(), 4, "Reordered PDF must have 4 pages");

        // External tool verification via qpdf --check and pdfinfo
        let tmp_merged =
            std::env::temp_dir().join(format!("hostile_merged_{}.pdf", std::process::id()));
        std::fs::write(&tmp_merged, &merged_bytes).unwrap();

        if let Some(tool) = find_tool("qpdf") {
            let qpdf_status = std::process::Command::new(tool)
                .arg("--check")
                .arg(&tmp_merged)
                .status();
            if let Ok(st) = qpdf_status {
                assert!(st.success(), "qpdf --check must succeed on merged hostile PDF");
            }
        } else {
            eprintln!("qpdf not found on system; skipped hostile PDF qpdf check");
        }

        let _ = std::fs::remove_file(&tmp_merged);
    }

    #[test]
    fn test_unicode_render_regression_full() {
        let initial_pdf = create_test_pdf(1);
        let mandatory_lines = vec![
            "これはうんちです",
            "こんにちは世界",
            "文書作成テスト",
            "PDFテスト 123 ABC",
            "漢字・ひらがな・カタカナ",
        ];

        let mut current_pdf = initial_pdf;
        let mut y = 700.0;
        for line in &mandatory_lines {
            current_pdf = add_text(&current_pdf, 0, line, 50.0, y, 16.0, "#1A202C")
                .expect("add_text must succeed for all mandatory lines");
            y -= 45.0;
        }

        let tmp_pdf =
            std::env::temp_dir().join(format!("unicode_render_test_{}.pdf", std::process::id()));
        std::fs::write(&tmp_pdf, &current_pdf).unwrap();

        // 1. Check with qpdf
        if let Some(tool) = find_tool("qpdf") {
            let qpdf_res = std::process::Command::new(tool)
                .arg("--check")
                .arg(&tmp_pdf)
                .output();
            if let Ok(res) = qpdf_res {
                assert!(
                    res.status.success(),
                    "qpdf --check must pass for Unicode PDF"
                );
            }
        } else {
            eprintln!("qpdf not found on system; skipped unicode qpdf check");
        }

        // 2. Extract with pdftotext
        if let Some(tool) = find_tool("pdftotext") {
            let pdftotext_res = std::process::Command::new(tool)
                .arg(&tmp_pdf)
                .arg("-")
                .output();
            if let Ok(res) = pdftotext_res {
                assert!(res.status.success(), "pdftotext must succeed");
                let extracted = String::from_utf8_lossy(&res.stdout);
                for line in &mandatory_lines {
                    assert!(
                        extracted.contains(line),
                        "Extracted text must contain '{line}'. Got:\n{extracted}"
                    );
                }
            }
        } else {
            eprintln!("pdftotext not found on system; skipped unicode pdftotext check");
        }

        // 3. Render with pdftoppm to PNG and verify rendering success
        let tmp_png_prefix =
            std::env::temp_dir().join(format!("rendered_page_{}", std::process::id()));
        if let Some(tool) = find_tool("pdftoppm") {
            let pdftoppm_res = std::process::Command::new(tool)
                .arg("-png")
                .arg("-r")
                .arg("150")
                .arg(&tmp_pdf)
                .arg(&tmp_png_prefix)
                .status();
            if let Ok(st) = pdftoppm_res {
                assert!(st.success(), "pdftoppm must succeed");
                let expected_png = format!("{}-1.png", tmp_png_prefix.display());
                assert!(
                    std::path::Path::new(&expected_png).exists(),
                    "Rendered PNG must be produced by pdftoppm"
                );
                let metadata = std::fs::metadata(&expected_png).unwrap();
                assert!(
                    metadata.len() > 1000,
                    "Rendered PNG must not be empty or blank"
                );
                let _ = std::fs::remove_file(&expected_png);
            }
        } else {
            eprintln!("pdftoppm not found on system; skipped unicode pdftoppm check");
        }

        let _ = std::fs::remove_file(&tmp_pdf);
    }

    #[test]
    fn test_delete_page_retains_inherited_attributes() {
        use crate::pdf_engine::page_tree::*;

        // Create a tree where an intermediate Pages node sets Rotate = 90 and MediaBox = [0, 0, 400, 600]
        let mut doc = Document::with_version("1.7");
        let (_root_id, pages_id) = ensure_catalog_and_pages_root(&mut doc);

        let p1_id = doc.add_object(Object::Dictionary(Dictionary::new()));
        let p2_id = doc.add_object(Object::Dictionary(Dictionary::new()));

        let mut inter_dict = Dictionary::new();
        inter_dict.set("Type", Object::Name(b"Pages".to_vec()));
        inter_dict.set("Parent", Object::Reference(pages_id));
        inter_dict.set("Rotate", Object::Integer(90));
        inter_dict.set(
            "MediaBox",
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Integer(400),
                Object::Integer(600),
            ]),
        );
        inter_dict.set(
            "Kids",
            Object::Array(vec![Object::Reference(p1_id), Object::Reference(p2_id)]),
        );
        inter_dict.set("Count", Object::Integer(2));
        let inter_id = doc.add_object(Object::Dictionary(inter_dict));

        // Connect p1 and p2 to inter_id
        if let Some(Object::Dictionary(ref mut p1)) = doc.objects.get_mut(&p1_id) {
            p1.set("Type", Object::Name(b"Page".to_vec()));
            p1.set("Parent", Object::Reference(inter_id));
        }
        if let Some(Object::Dictionary(ref mut p2)) = doc.objects.get_mut(&p2_id) {
            p2.set("Type", Object::Name(b"Page".to_vec()));
            p2.set("Parent", Object::Reference(inter_id));
        }

        // Connect root pages to intermediate
        if let Some(Object::Dictionary(ref mut root_pages)) = doc.objects.get_mut(&pages_id) {
            root_pages.set("Kids", Object::Array(vec![Object::Reference(inter_id)]));
            root_pages.set("Count", Object::Integer(2));
        }

        let mut raw_bytes = Vec::new();
        doc.save_to(&mut raw_bytes).expect("Save raw hostile doc");

        // Now delete page index 1 (p2). Page 0 (p1) must retain Rotate = 90 and MediaBox!
        let modified_bytes = delete_page(&raw_bytes, 1).expect("delete_page must succeed");
        let modified_doc = Document::load_mem(&modified_bytes).expect("Reload after delete");

        let logical_pages = get_logical_page_ids(&modified_doc);
        assert_eq!(logical_pages.len(), 1, "Must have exactly 1 page remaining");

        let remaining_pdict = modified_doc
            .objects
            .get(&logical_pages[0])
            .and_then(|o| o.as_dict().ok())
            .expect("Remaining page dictionary");

        let rot = remaining_pdict
            .get(b"Rotate")
            .ok()
            .and_then(|r| r.as_i64().ok())
            .unwrap_or(0);
        assert_eq!(
            rot, 90,
            "Remaining page must retain inherited Rotate = 90 after sibling deletion"
        );

        let mbox = remaining_pdict
            .get(b"MediaBox")
            .ok()
            .and_then(|m| m.as_array().ok());
        assert!(
            mbox.is_some(),
            "Remaining page must retain inherited MediaBox after sibling deletion"
        );
    }

    #[test]
    fn test_copy_object_graph_preserves_widget_parent_and_skips_annot_p() {
        use crate::pdf_engine::page_tree::copy_object_graph;
        use std::collections::HashMap;

        let mut src_doc = Document::with_version("1.7");
        let mut dest_doc = Document::with_version("1.7");
        // Add dummy objects in dest_doc so its OIDs don't coincidentally match src_doc
        dest_doc.add_object(Object::Integer(42));
        dest_doc.add_object(Object::Integer(43));

        // Create a Source Page
        let src_page_id = src_doc.add_object(Object::Dictionary(Dictionary::new()));

        // Create an AcroForm Field and a Widget with /Parent pointing to Field
        let mut field_dict = Dictionary::new();
        field_dict.set("Type", Object::Name(b"Field".to_vec()));
        field_dict.set("T", Object::string_literal("UserName"));
        let field_id = src_doc.add_object(Object::Dictionary(field_dict));

        let mut widget_dict = Dictionary::new();
        widget_dict.set("Type", Object::Name(b"Annot".to_vec()));
        widget_dict.set("Subtype", Object::Name(b"Widget".to_vec()));
        widget_dict.set("Parent", Object::Reference(field_id));
        widget_dict.set("P", Object::Reference(src_page_id)); // Back-reference to Page
        let widget_id = src_doc.add_object(Object::Dictionary(widget_dict));

        let mut id_map = HashMap::new();
        let dest_widget_id = copy_object_graph(&src_doc, &mut dest_doc, widget_id, &mut id_map);

        let copied_widget = dest_doc
            .objects
            .get(&dest_widget_id)
            .and_then(|o| o.as_dict().ok())
            .expect("Copied widget dict");

        // 1. /Parent MUST NOT have been skipped because it's a form widget!
        assert!(
            copied_widget.get(b"Parent").is_ok(),
            "Widget /Parent must NOT be stripped by copy_object_graph"
        );
        let new_parent_ref = copied_widget.get(b"Parent").unwrap().as_reference().unwrap();
        assert_ne!(
            new_parent_ref, field_id,
            "Parent field reference must be remapped to new object in dest_doc"
        );
        assert!(
            dest_doc.objects.contains_key(&new_parent_ref),
            "dest_doc must contain the copied parent Field object"
        );

        // 2. /P on Annot MUST have been skipped to prevent source page inclusion!
        assert!(
            copied_widget.get(b"P").is_err(),
            "Annot /P back-reference to Page must be stripped to prevent recursive source page inclusion"
        );
    }
}

