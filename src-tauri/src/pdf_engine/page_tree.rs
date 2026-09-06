use super::common::{save_doc, OID};
use lopdf::{Dictionary, Document, Object};
use std::collections::{HashMap, HashSet};

/// Attributes that can be inherited through intermediate `/Pages` nodes in a PDF Page Tree (ISO 32000-1 §7.7.3.3).
pub const INHERITABLE_PAGE_ATTRS: &[&[u8]] = &[
    b"Resources",
    b"MediaBox",
    b"CropBox",
    b"Rotate",
    b"BleedBox",
    b"TrimBox",
    b"ArtBox",
];

/// Collects all logical page object IDs in tree preorder traversal.
/// Handles arbitrarily nested `/Pages` trees correctly.
pub fn get_logical_page_ids(doc: &Document) -> Vec<OID> {
    let mut page_ids = Vec::new();
    let root_id = match doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        Ok(id) => id,
        Err(_) => return page_ids,
    };

    let pages_id = match doc.objects.get(&root_id).and_then(|o| o.as_dict().ok()) {
        Some(dict) => match dict.get(b"Pages").and_then(|p| p.as_reference()) {
            Ok(pid) => pid,
            Err(_) => return page_ids,
        },
        None => return page_ids,
    };

    let mut visited = HashSet::new();
    traverse_pages_node(doc, pages_id, &mut page_ids, &mut visited);
    page_ids
}

fn traverse_pages_node(
    doc: &Document,
    node_id: OID,
    out_pages: &mut Vec<OID>,
    visited: &mut HashSet<OID>,
) {
    if !visited.insert(node_id) {
        return; // Break cycles
    }

    let node_dict = match doc.objects.get(&node_id).and_then(|o| o.as_dict().ok()) {
        Some(d) => d,
        None => return,
    };

    let type_name = node_dict.get(b"Type").ok().and_then(|t| t.as_name().ok());
    if type_name == Some(b"Page") {
        out_pages.push(node_id);
        return;
    }

    if let Ok(Object::Array(kids)) = node_dict.get(b"Kids") {
        for kid in kids {
            if let Ok(kid_id) = kid.as_reference() {
                traverse_pages_node(doc, kid_id, out_pages, visited);
            }
        }
    }
}

/// Materializes inheritable attributes (Resources, MediaBox, CropBox, Rotate, etc.)
/// directly onto a Page dictionary by walking up its `/Parent` chain.
pub fn materialize_inherited_page_attrs(doc: &Document, page_id: OID) -> Dictionary {
    let mut materialized = Dictionary::new();

    let page_dict = match doc.objects.get(&page_id).and_then(|o| o.as_dict().ok()) {
        Some(d) => d.clone(),
        None => return materialized,
    };

    // First clone existing page dict entries
    for (k, v) in &page_dict {
        materialized.set(k.clone(), v.clone());
    }

    // Now look for missing inheritable attributes by walking up parents
    for &attr in INHERITABLE_PAGE_ATTRS {
        if materialized.get(attr).is_err() {
            let mut curr = page_dict
                .get(b"Parent")
                .ok()
                .and_then(|p| p.as_reference().ok());
            let mut visited = HashSet::new();

            while let Some(pid) = curr {
                if !visited.insert(pid) {
                    break;
                }
                if let Some(parent_dict) = doc.objects.get(&pid).and_then(|o| o.as_dict().ok()) {
                    if let Ok(val) = parent_dict.get(attr) {
                        materialized.set(attr.to_vec(), val.clone());
                        break;
                    }
                    curr = parent_dict
                        .get(b"Parent")
                        .ok()
                        .and_then(|p| p.as_reference().ok());
                } else {
                    break;
                }
            }
        }
    }

    // Default MediaBox if none found in ancestry
    if materialized.get(b"MediaBox").is_err() {
        materialized.set(
            "MediaBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(595.0),
                Object::Real(842.0),
            ]),
        );
    }

    // Default Resources if none found in ancestry (required by PDF spec on Page unless inherited)
    if materialized.get(b"Resources").is_err() {
        materialized.set("Resources", Object::Dictionary(Dictionary::new()));
    }

    materialized
}

/// Ensures a pristine, ISO 32000-1 compliant Catalog and Pages root structure.
/// Structure:
/// Trailer
///  └─ /Root -> Catalog
///         └─ /Pages -> Pages (with empty or preserved kids)
pub fn ensure_catalog_and_pages_root(doc: &mut Document) -> (OID, OID) {
    let root_ref = doc
        .trailer
        .get(b"Root")
        .ok()
        .and_then(|o| o.as_reference().ok());

    let (catalog_id, pages_id) = match root_ref {
        Some(cid) => {
            if let Some(Object::Dictionary(ref mut cat_dict)) = doc.objects.get_mut(&cid) {
                // If it's already a Catalog
                if cat_dict.get(b"Type").ok().and_then(|t| t.as_name().ok()) == Some(b"Catalog") {
                    let existing_pid = cat_dict.get(b"Pages").and_then(|p| p.as_reference()).ok();
                    let pid = match existing_pid {
                        Some(pid) => pid,
                        None => {
                            let mut pdict = Dictionary::new();
                            pdict.set("Type", Object::Name("Pages".into()));
                            pdict.set("Kids", Object::Array(vec![]));
                            pdict.set("Count", Object::Integer(0));
                            let new_pid = doc.add_object(Object::Dictionary(pdict));
                            if let Some(Object::Dictionary(ref mut d)) = doc.objects.get_mut(&cid) {
                                d.set("Pages", Object::Reference(new_pid));
                            }
                            new_pid
                        }
                    };
                    (cid, pid)
                } else if cat_dict.get(b"Type").ok().and_then(|t| t.as_name().ok())
                    == Some(b"Pages")
                {
                    // Trailer /Root was pointing directly to Pages! Fix this severe PDF spec violation.
                    let old_pages_id = cid;
                    let mut new_cat = Dictionary::new();
                    new_cat.set("Type", Object::Name("Catalog".into()));
                    new_cat.set("Pages", Object::Reference(old_pages_id));
                    let new_cid = doc.add_object(Object::Dictionary(new_cat));
                    doc.trailer.set("Root", Object::Reference(new_cid));
                    (new_cid, old_pages_id)
                } else {
                    // Unknown or corrupt root object; wrap/fix it
                    let mut pdict = Dictionary::new();
                    pdict.set("Type", Object::Name("Pages".into()));
                    pdict.set("Kids", Object::Array(vec![]));
                    pdict.set("Count", Object::Integer(0));
                    let new_pid = doc.add_object(Object::Dictionary(pdict));

                    if let Some(Object::Dictionary(ref mut d)) = doc.objects.get_mut(&cid) {
                        d.set("Type", Object::Name("Catalog".into()));
                        d.set("Pages", Object::Reference(new_pid));
                    }
                    (cid, new_pid)
                }
            } else {
                // Catalog object missing from doc.objects
                let mut pdict = Dictionary::new();
                pdict.set("Type", Object::Name("Pages".into()));
                pdict.set("Kids", Object::Array(vec![]));
                pdict.set("Count", Object::Integer(0));
                let pid = doc.add_object(Object::Dictionary(pdict));

                let mut cdict = Dictionary::new();
                cdict.set("Type", Object::Name("Catalog".into()));
                cdict.set("Pages", Object::Reference(pid));
                let new_cid = doc.add_object(Object::Dictionary(cdict));
                doc.trailer.set("Root", Object::Reference(new_cid));
                (new_cid, pid)
            }
        }
        None => {
            let mut pdict = Dictionary::new();
            pdict.set("Type", Object::Name("Pages".into()));
            pdict.set("Kids", Object::Array(vec![]));
            pdict.set("Count", Object::Integer(0));
            let pid = doc.add_object(Object::Dictionary(pdict));

            let mut cdict = Dictionary::new();
            cdict.set("Type", Object::Name("Catalog".into()));
            cdict.set("Pages", Object::Reference(pid));
            let new_cid = doc.add_object(Object::Dictionary(cdict));
            doc.trailer.set("Root", Object::Reference(new_cid));
            (new_cid, pid)
        }
    };

    (catalog_id, pages_id)
}

/// Recursively copies an object and all referenced descendant objects from `source_doc` into `dest_doc`.
/// Maintains an `id_map` of `old_OID -> new_OID` to resolve shared and cyclical object graphs without duplication.
pub fn copy_object_graph(
    source_doc: &Document,
    dest_doc: &mut Document,
    root_obj_id: OID,
    id_map: &mut HashMap<OID, OID>,
) -> OID {
    if let Some(&new_id) = id_map.get(&root_obj_id) {
        return new_id;
    }

    // Allocate placeholder ID in dest_doc to prevent infinite recursion on cyclical references
    let temp_obj = Object::Null;
    let new_id = dest_doc.add_object(temp_obj);
    id_map.insert(root_obj_id, new_id);

    // Deep copy and remap the object
    if let Some(src_obj) = source_doc.objects.get(&root_obj_id) {
        let mut cloned_obj = src_obj.clone();
        remap_and_copy_children(source_doc, dest_doc, &mut cloned_obj, id_map);
        dest_doc.objects.insert(new_id, cloned_obj);
    }

    new_id
}

fn remap_and_copy_children(
    source_doc: &Document,
    dest_doc: &mut Document,
    obj: &mut Object,
    id_map: &mut HashMap<OID, OID>,
) {
    match obj {
        Object::Reference(ref mut r) => {
            // Do not copy Page parent references back into source document's old Pages node
            let is_pages_parent = if let Some(target) = source_doc.objects.get(r) {
                if let Ok(d) = target.as_dict() {
                    d.get(b"Type").ok().and_then(|t| t.as_name().ok()) == Some(b"Pages")
                } else {
                    false
                }
            } else {
                false
            };

            if is_pages_parent {
                // Will be re-parented by rebuild_flat_page_tree in destination document
                return;
            }

            let new_oid = copy_object_graph(source_doc, dest_doc, *r, id_map);
            *r = new_oid;
        }
        Object::Array(arr) => {
            for item in arr.iter_mut() {
                remap_and_copy_children(source_doc, dest_doc, item, id_map);
            }
        }
        Object::Dictionary(dict) => {
            let is_page = dict.get(b"Type").ok().and_then(|t| t.as_name().ok()) == Some(b"Page");
            let is_annot = dict.get(b"Type").ok().and_then(|t| t.as_name().ok()) == Some(b"Annot");

            // PDF ISO 32000-1:
            // 1. Only strip /Parent if this dictionary is a Page (which will be reparented by rebuild_flat_page_tree).
            // Form fields (Widget -> Field) MUST keep /Parent.
            if is_page {
                dict.remove(b"Parent");
            }

            // 2. Remove /P back-reference on Annotations so we don't accidentally pull the entire source Page into the child graph
            if is_annot {
                dict.remove(b"P");
            }

            for (_, val) in dict.iter_mut() {
                remap_and_copy_children(source_doc, dest_doc, val, id_map);
            }
        }
        Object::Stream(stream) => {
            let is_page = stream.dict.get(b"Type").ok().and_then(|t| t.as_name().ok()) == Some(b"Page");
            for (key, val) in stream.dict.iter_mut() {
                if is_page && key == b"Parent" {
                    continue;
                }
                remap_and_copy_children(source_doc, dest_doc, val, id_map);
            }
        }
        _ => {}
    }
}

/// Rebuilds the Page Tree of `doc` into a clean, single-level flat hierarchy:
/// Catalog -> Pages -> Kids [ Page 1, Page 2, ... ]
/// Ensures each Page's `/Parent` points directly to the newly unified `/Pages` node,
/// materializes all inherited properties (MediaBox, Resources, Rotate, CropBox, etc.)
/// onto each page dictionary prior to flattening, and updates `/Count` to match the exact number of pages.
pub fn rebuild_flat_page_tree(doc: &mut Document, ordered_page_ids: &[OID]) -> Result<(), String> {
    let (_, pages_id) = ensure_catalog_and_pages_root(doc);

    // 1. Mandatory rule: Materialize inherited attributes onto each remaining page BEFORE severing the tree
    for &pid in ordered_page_ids {
        let materialized = materialize_inherited_page_attrs(doc, pid);
        if let Some(Object::Dictionary(ref mut pdict)) = doc.objects.get_mut(&pid) {
            for (k, v) in materialized {
                if pdict.get(&k).is_err() {
                    pdict.set(k, v);
                }
            }
        }
    }

    // 2. Update each page's /Parent to point to pages_id and ensure /Type is /Page
    for &pid in ordered_page_ids {
        if let Some(Object::Dictionary(ref mut pdict)) = doc.objects.get_mut(&pid) {
            pdict.set("Type", Object::Name("Page".into()));
            pdict.set("Parent", Object::Reference(pages_id));
        }
    }

    // Update the Pages node
    if let Some(Object::Dictionary(ref mut pages_dict)) = doc.objects.get_mut(&pages_id) {
        pages_dict.set("Type", Object::Name("Pages".into()));
        let kids_arr: Vec<Object> = ordered_page_ids
            .iter()
            .map(|&pid| Object::Reference(pid))
            .collect();
        pages_dict.set("Kids", Object::Array(kids_arr));
        pages_dict.set("Count", Object::Integer(ordered_page_ids.len() as i64));
    }

    Ok(())
}

/// Robust page extraction:
/// - Builds a brand new Document with proper Catalog and Pages root
/// - Materializes inherited page properties (Resources, MediaBox, CropBox, Rotate)
/// - Deep copies the complete object graph reachable from each extracted page
/// - Sets up clean, valid `/Kids` and `/Parent` relationships
pub fn extract_pages_robust(data: &[u8], indices: &[usize]) -> Result<Vec<u8>, String> {
    let src_doc =
        Document::load_mem(data).map_err(|e| format!("Failed to load source PDF: {e}"))?;
    let src_page_ids = get_logical_page_ids(&src_doc);

    let mut dest_doc = Document::with_version("1.7");
    let (_, dest_pages_id) = ensure_catalog_and_pages_root(&mut dest_doc);

    let mut id_map = HashMap::new();
    let mut dest_page_ids = Vec::new();

    for &idx in indices {
        if idx >= src_page_ids.len() {
            continue;
        }
        let src_pid = src_page_ids[idx];

        // 1. Materialize all inherited attributes
        let mut page_dict = materialize_inherited_page_attrs(&src_doc, src_pid);

        // 2. Clear out old Parent
        page_dict.set("Parent", Object::Reference(dest_pages_id));
        page_dict.set("Type", Object::Name("Page".into()));

        // 3. Deep-copy all referenced child objects from page dictionary
        let mut page_obj = Object::Dictionary(page_dict);
        remap_and_copy_children(&src_doc, &mut dest_doc, &mut page_obj, &mut id_map);

        // 4. Add new page to dest_doc
        let new_pid = dest_doc.add_object(page_obj);
        dest_page_ids.push(new_pid);
    }

    if dest_page_ids.is_empty() {
        return Err("No valid pages selected for extraction".to_string());
    }

    // 5. Finalize flat page tree
    rebuild_flat_page_tree(&mut dest_doc, &dest_page_ids)?;
    dest_doc.prune_objects();
    save_doc(&mut dest_doc)
}

/// Robust PDF merge:
/// - Reads each PDF, extracts its logical pages with inherited attributes materialized
/// - Deep copies each page and its reachable object graph into the unified document
/// - Constructs a single, clean flat Page Tree where every Page's `/Parent` points to `/Pages`
/// - Eliminates dangling or circular Pages nodes
pub fn merge_pdfs_robust(paths: &[String]) -> Result<Vec<u8>, String> {
    if paths.is_empty() {
        return Err("No files to merge".into());
    }

    let mut merged_doc = Document::with_version("1.7");
    let (_, dest_pages_id) = ensure_catalog_and_pages_root(&mut merged_doc);
    let mut dest_page_ids = Vec::new();

    for path in paths {
        let other_doc = Document::load(path).map_err(|e| format!("Failed to load {path}: {e}"))?;
        let other_page_ids = get_logical_page_ids(&other_doc);

        let mut id_map = HashMap::new();

        for src_pid in other_page_ids {
            let mut page_dict = materialize_inherited_page_attrs(&other_doc, src_pid);
            page_dict.set("Parent", Object::Reference(dest_pages_id));
            page_dict.set("Type", Object::Name("Page".into()));

            let mut page_obj = Object::Dictionary(page_dict);
            remap_and_copy_children(&other_doc, &mut merged_doc, &mut page_obj, &mut id_map);

            let new_pid = merged_doc.add_object(page_obj);
            dest_page_ids.push(new_pid);
        }
    }

    rebuild_flat_page_tree(&mut merged_doc, &dest_page_ids)?;
    merged_doc.prune_objects();
    save_doc(&mut merged_doc)
}

/// Robust PDF merge from in-memory byte buffers:
pub fn merge_pdf_buffers_robust(buffers: &[&[u8]]) -> Result<Vec<u8>, String> {
    if buffers.is_empty() {
        return Err("No buffers to merge".into());
    }

    let mut merged_doc = Document::with_version("1.7");
    let (_, dest_pages_id) = ensure_catalog_and_pages_root(&mut merged_doc);
    let mut dest_page_ids = Vec::new();

    for buf in buffers {
        let other_doc = Document::load_mem(buf).map_err(|e| format!("Failed to load PDF: {e}"))?;
        let other_page_ids = get_logical_page_ids(&other_doc);

        let mut id_map = HashMap::new();

        for src_pid in other_page_ids {
            let mut page_dict = materialize_inherited_page_attrs(&other_doc, src_pid);
            page_dict.set("Parent", Object::Reference(dest_pages_id));
            page_dict.set("Type", Object::Name("Page".into()));

            let mut page_obj = Object::Dictionary(page_dict);
            remap_and_copy_children(&other_doc, &mut merged_doc, &mut page_obj, &mut id_map);

            let new_pid = merged_doc.add_object(page_obj);
            dest_page_ids.push(new_pid);
        }
    }

    rebuild_flat_page_tree(&mut merged_doc, &dest_page_ids)?;
    merged_doc.prune_objects();
    save_doc(&mut merged_doc)
}

/// Robust page deletion:
/// - Resolves logical page IDs across any nested Page Tree
/// - Removes the requested page
/// - Flattens and rebuilds the remaining pages with valid `/Parent`, `/Kids`, and `/Count`
pub fn delete_page_robust(data: &[u8], page_index: usize) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let mut page_ids = get_logical_page_ids(&doc);

    if page_index >= page_ids.len() {
        return Err(format!(
            "Page index {page_index} out of range (total pages: {})",
            page_ids.len()
        ));
    }

    if page_ids.len() <= 1 {
        return Err("Cannot delete the only remaining page in the document".to_string());
    }

    // Remove logical page
    let removed_pid = page_ids.remove(page_index);
    doc.objects.remove(&removed_pid);

    // Rebuild tree
    rebuild_flat_page_tree(&mut doc, &page_ids)?;
    doc.prune_objects();
    save_doc(&mut doc)
}

/// Robust page reordering:
/// - Resolves logical page IDs across any nested Page Tree
/// - Moves logical page from `from_index` to `to_index`
/// - Materializes inherited attributes before flattening to ensure properties are never lost
/// - Rebuilds the Page Tree with clean `/Parent` and `/Kids`
pub fn reorder_pages_robust(
    data: &[u8],
    from_index: usize,
    to_index: usize,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let mut page_ids = get_logical_page_ids(&doc);

    if from_index >= page_ids.len() || to_index >= page_ids.len() {
        return Err(format!(
            "Page index out of range: from={from_index}, to={to_index}, total={}",
            page_ids.len()
        ));
    }

    // Materialize inherited attributes for all pages before restructuring
    for &pid in &page_ids {
        let materialized = materialize_inherited_page_attrs(&doc, pid);
        if let Some(Object::Dictionary(ref mut pdict)) = doc.objects.get_mut(&pid) {
            for (k, v) in materialized {
                if pdict.get(&k).is_err() {
                    pdict.set(k, v);
                }
            }
        }
    }

    let moved_id = page_ids.remove(from_index);
    page_ids.insert(to_index, moved_id);

    rebuild_flat_page_tree(&mut doc, &page_ids)?;
    doc.prune_objects();
    save_doc(&mut doc)
}

/// Robust page duplication:
/// - Duplicates logical page at `page_index`
/// - Deep copies its content and resources
/// - Inserts duplicate right after the original
/// - Rebuilds flat tree
pub fn duplicate_page_robust(data: &[u8], page_index: usize) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
    let mut page_ids = get_logical_page_ids(&doc);

    if page_index >= page_ids.len() {
        return Err(format!(
            "Page index {page_index} out of range (total pages: {})",
            page_ids.len()
        ));
    }

    let src_pid = page_ids[page_index];
    let mut page_dict = materialize_inherited_page_attrs(&doc, src_pid);

    let (_, pages_id) = ensure_catalog_and_pages_root(&mut doc);
    page_dict.set("Parent", Object::Reference(pages_id));

    // Deep copy content streams and local resources for the duplicate
    let mut id_map = HashMap::new();
    let mut page_obj = Object::Dictionary(page_dict);
    let cloned_doc = doc.clone();
    remap_and_copy_children(&cloned_doc, &mut doc, &mut page_obj, &mut id_map);

    let new_pid = doc.add_object(page_obj);
    let insert_at = (page_index + 1).min(page_ids.len());
    page_ids.insert(insert_at, new_pid);

    rebuild_flat_page_tree(&mut doc, &page_ids)?;
    doc.prune_objects();
    save_doc(&mut doc)
}
