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
}
