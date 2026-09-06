#[cfg(test)]
mod tests {
    use crate::pdf_engine::*;
    use lopdf::{Dictionary, Document, Object};

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

        let sigs = result["signatures"].as_array().expect("Must have signatures array");
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
        assert!(ids.is_empty(), "list_digital_ids must be empty when no certificates enrolled");

        let certs = list_certificates().expect("list_certificates must succeed");
        assert!(certs.is_empty(), "list_certificates must be empty");

        let ts_res = add_timestamp(&[], "http://tsa.example.com");
        assert!(ts_res.is_err(), "add_timestamp must reject without real TSA connection");
    }
}
