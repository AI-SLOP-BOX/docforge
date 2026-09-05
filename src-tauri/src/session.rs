use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);
// Soft memory limit for undo/redo snapshots per document.
// Eviction drops oldest history when exceeded, but always preserves at least 1 undo state
// so that single large documents (>512MB) do not immediately lose undo capability.
const TARGET_HISTORY_MEMORY_PER_DOC: usize = 512 * 1024 * 1024; // 512MB target threshold

#[derive(Clone)]
pub enum EditCommand {
    RotatePage {
        page: usize,
        from_degrees: i32,
        to_degrees: i32,
    },
    FullSnapshot {
        description: String,
        data: Vec<u8>,
    },
}

impl EditCommand {
    pub fn byte_size(&self) -> usize {
        match self {
            EditCommand::RotatePage { .. } => std::mem::size_of::<Self>(),
            EditCommand::FullSnapshot { data, description } => {
                data.len() + description.len() + std::mem::size_of::<Self>()
            }
        }
    }
}

pub struct DocumentSession {
    pub id: String,
    pub doc: lopdf::Document,
    pub undo_stack: Vec<EditCommand>,
    pub redo_stack: Vec<EditCommand>,
    pub dirty: bool,
    pub total_history_bytes: usize,
    cached_bytes: Option<Vec<u8>>,
}

impl DocumentSession {
    pub fn new(id: String, doc: lopdf::Document) -> Self {
        Self {
            id,
            doc,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            dirty: false,
            total_history_bytes: 0,
            cached_bytes: None,
        }
    }

    /// Mutate the inner Document with automatic cache invalidation and dirty marking.
    /// This prevents cache consistency bugs when modifying the document model.
    /// Even if the mutator returns Err, any partial modification to `doc` invalidates `cached_bytes`
    /// to eliminate phantom/stale cache risk.
    pub fn mutate<F, R>(&mut self, f: F) -> Result<R, String>
    where
        F: FnOnce(&mut lopdf::Document) -> Result<R, String>,
    {
        let res = f(&mut self.doc);
        self.dirty = true;
        self.cached_bytes = None;
        res
    }

    pub fn push_undo(&mut self, cmd: EditCommand) {
        self.push_undo_internal(cmd, true);
    }

    pub fn push_undo_internal(&mut self, cmd: EditCommand, clear_redo: bool) {
        let size = cmd.byte_size();
        if clear_redo {
            for c in self.redo_stack.drain(..) {
                self.total_history_bytes = self.total_history_bytes.saturating_sub(c.byte_size());
            }
        }
        self.total_history_bytes += size;
        self.undo_stack.push(cmd);
        self.dirty = true;
        self.cached_bytes = None;

        self.evict_oldest_history();
    }

    pub fn invalidate_cache(&mut self) {
        self.cached_bytes = None;
    }

    // Evict oldest entries from undo_stack if total undo + redo exceeds target budget.
    // Notice: This is a memory budget (soft threshold), not a hard cap.
    // We always preserve at least 1 undo entry so that even large files (>512MB) retain immediate undo capability.
    fn evict_oldest_history(&mut self) {
        while self.total_history_bytes > TARGET_HISTORY_MEMORY_PER_DOC && self.undo_stack.len() > 1 {
            let evicted = self.undo_stack.remove(0);
            self.total_history_bytes = self.total_history_bytes.saturating_sub(evicted.byte_size());
        }
    }

    /// Access cached or serialized bytes by reference without allocating or cloning the entire PDF buffer.
    pub fn with_bytes<F, R>(&mut self, f: F) -> Result<R, String>
    where
        F: FnOnce(&[u8]) -> Result<R, String>,
    {
        if self.cached_bytes.is_none() {
            let mut buf = Vec::new();
            self.doc
                .save_to(&mut buf)
                .map_err(|e| format!("Failed to serialize PDF: {e}"))?;
            self.cached_bytes = Some(buf);
        }
        let bytes = self.cached_bytes.as_ref().unwrap();
        f(bytes.as_slice())
    }

    pub fn save_to_bytes(&mut self) -> Result<Vec<u8>, String> {
        self.with_bytes(|b| Ok(b.to_vec()))
    }

    pub fn undo(&mut self) -> Result<bool, String> {
        let cmd = match self.undo_stack.last() {
            Some(c) => c.clone(),
            None => return Ok(false),
        };

        // Execute undo operation first without mutating history stacks
        let redo_cmd = match cmd {
            EditCommand::RotatePage {
                page,
                from_degrees,
                to_degrees,
            } => {
                let delta = from_degrees - to_degrees;
                crate::pdf_engine::rotate_page_in_doc(&mut self.doc, page, delta)?;
                EditCommand::RotatePage {
                    page,
                    from_degrees,
                    to_degrees,
                }
            }
            EditCommand::FullSnapshot { description, ref data } => {
                let mut current = Vec::new();
                self.doc
                    .save_to(&mut current)
                    .map_err(|e| format!("Failed to serialize current state during undo: {e}"))?;
                let restored = lopdf::Document::load_mem(data)
                    .map_err(|e| format!("Failed to restore snapshot: {e}"))?;
                self.doc = restored;
                EditCommand::FullSnapshot {
                    description,
                    data: current,
                }
            }
        };

        // Once successful, commit transition from undo_stack to redo_stack
        let popped = self.undo_stack.pop().unwrap();
        self.total_history_bytes = self.total_history_bytes.saturating_sub(popped.byte_size());

        self.total_history_bytes += redo_cmd.byte_size();
        self.redo_stack.push(redo_cmd);
        self.cached_bytes = None;
        self.evict_oldest_history();
        Ok(true)
    }

    pub fn redo(&mut self) -> Result<bool, String> {
        let cmd = match self.redo_stack.last() {
            Some(c) => c.clone(),
            None => return Ok(false),
        };

        // Execute redo operation first without mutating history stacks
        let undo_cmd = match cmd {
            EditCommand::RotatePage {
                page,
                from_degrees,
                to_degrees,
            } => {
                let delta = to_degrees - from_degrees;
                crate::pdf_engine::rotate_page_in_doc(&mut self.doc, page, delta)?;
                EditCommand::RotatePage {
                    page,
                    from_degrees,
                    to_degrees,
                }
            }
            EditCommand::FullSnapshot { description, ref data } => {
                let mut current = Vec::new();
                self.doc
                    .save_to(&mut current)
                    .map_err(|e| format!("Failed to serialize current state during redo: {e}"))?;
                let restored = lopdf::Document::load_mem(data)
                    .map_err(|e| format!("Failed to restore snapshot: {e}"))?;
                self.doc = restored;
                EditCommand::FullSnapshot {
                    description,
                    data: current,
                }
            }
        };

        // Once successful, commit transition from redo_stack to undo_stack
        let popped = self.redo_stack.pop().unwrap();
        self.total_history_bytes = self.total_history_bytes.saturating_sub(popped.byte_size());

        self.push_undo_internal(undo_cmd, false);
        self.cached_bytes = None;
        Ok(true)
    }
}

#[derive(Default)]
pub struct SessionManager {
    sessions: RwLock<HashMap<String, Arc<RwLock<DocumentSession>>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    pub fn create_session(&self, data: &[u8]) -> Result<String, String> {
        let doc =
            lopdf::Document::load_mem(data).map_err(|e| format!("Failed to load PDF: {e}"))?;
        let id = format!("doc_{}", SESSION_COUNTER.fetch_add(1, Ordering::SeqCst));
        let session = Arc::new(RwLock::new(DocumentSession::new(id.clone(), doc)));

        let mut lock = self.sessions.write().map_err(|e| e.to_string())?;
        lock.insert(id.clone(), session);
        Ok(id)
    }

    pub fn get_session(&self, id: &str) -> Result<Arc<RwLock<DocumentSession>>, String> {
        let lock = self.sessions.read().map_err(|e| e.to_string())?;
        lock.get(id)
            .cloned()
            .ok_or_else(|| format!("Session {id} not found"))
    }

    pub fn close_session(&self, id: &str) -> bool {
        if let Ok(mut lock) = self.sessions.write() {
            lock.remove(id).is_some()
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_pdf() -> Vec<u8> {
        let mut doc = lopdf::Document::with_version("1.7");
        let pages_id = doc.add_object(lopdf::Object::Dictionary(lopdf::Dictionary::new()));

        let content = b"BT /F1 12 Tf 50 50 Td (Hello World) Tj ET";
        let content_id = doc.add_object(lopdf::Object::Stream(lopdf::Stream::new(
            lopdf::Dictionary::new(),
            content.to_vec(),
        )));

        let mut page_dict = lopdf::Dictionary::new();
        page_dict.set("Type", lopdf::Object::Name("Page".into()));
        page_dict.set("Parent", lopdf::Object::Reference(pages_id));
        page_dict.set("MediaBox", lopdf::Object::Array(vec![lopdf::Object::Real(0.0), lopdf::Object::Real(0.0), lopdf::Object::Real(100.0), lopdf::Object::Real(100.0)]));
        page_dict.set("Contents", lopdf::Object::Reference(content_id));
        let page_id = doc.add_object(lopdf::Object::Dictionary(page_dict));

        let mut pages_dict = lopdf::Dictionary::new();
        pages_dict.set("Type", lopdf::Object::Name("Pages".into()));
        pages_dict.set("Count", lopdf::Object::Integer(1));
        pages_dict.set("Kids", lopdf::Object::Array(vec![lopdf::Object::Reference(page_id)]));
        doc.objects.insert(pages_id, lopdf::Object::Dictionary(pages_dict));

        let mut catalog = lopdf::Dictionary::new();
        catalog.set("Type", lopdf::Object::Name("Catalog".into()));
        catalog.set("Pages", lopdf::Object::Reference(pages_id));
        let cat_id = doc.add_object(lopdf::Object::Dictionary(catalog));
        doc.trailer.set("Root", lopdf::Object::Reference(cat_id));

        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    #[test]
    fn test_session_creation_and_delta_undo_redo() {
        let pdf = dummy_pdf();
        let manager = SessionManager::new();
        let id = manager.create_session(&pdf).expect("Create session");

        let session_arc = manager.get_session(&id).expect("Get session");
        let mut session = session_arc.write().unwrap();

        // 1. Initial rotation should be 0
        let page_ids = crate::pdf_engine::get_page_ids(&session.doc);
        assert_eq!(page_ids.len(), 1);

        // 2. Rotate page by 90 deg using lightweight delta command
        crate::pdf_engine::rotate_page_in_doc(&mut session.doc, 0, 90).unwrap();
        session.push_undo(EditCommand::RotatePage {
            page: 0,
            from_degrees: 0,
            to_degrees: 90,
        });
        assert!(session.dirty);
        assert_eq!(session.undo_stack.len(), 1);

        // 3. Undo rotation
        let undone = session.undo().expect("Undo");
        assert!(undone);
        assert_eq!(session.redo_stack.len(), 1);

        // Check page rotation reverted to 0
        let rot = session.doc.objects.get(&page_ids[0]).and_then(|o| o.as_dict().ok()).and_then(|d| d.get(b"Rotate").ok()).and_then(|r| r.as_i64().ok()).unwrap_or(0);
        assert_eq!(rot, 0);

        // 4. Redo rotation
        let redone = session.redo().expect("Redo");
        assert!(redone);
        let rot2 = session.doc.objects.get(&page_ids[0]).and_then(|o| o.as_dict().ok()).and_then(|d| d.get(b"Rotate").ok()).and_then(|r| r.as_i64().ok()).unwrap_or(0);
        assert_eq!(rot2, 90);
    }

    #[test]
    fn test_multi_step_undo_redo_preservation() {
        let pdf = dummy_pdf();
        let manager = SessionManager::new();
        let id = manager.create_session(&pdf).expect("Create session");
        let session_arc = manager.get_session(&id).expect("Get session");
        let mut session = session_arc.write().unwrap();
        let page_ids = crate::pdf_engine::get_page_ids(&session.doc);

        // Perform 3 consecutive 90 degree rotations: 0 -> 90 -> 180 -> 270
        for i in 0..3 {
            let from = (i * 90) % 360;
            let to = ((i + 1) * 90) % 360;
            crate::pdf_engine::rotate_page_in_doc(&mut session.doc, 0, 90).unwrap();
            session.push_undo(EditCommand::RotatePage {
                page: 0,
                from_degrees: from,
                to_degrees: to,
            });
        }
        assert_eq!(session.undo_stack.len(), 3);

        // Check rotation is 270
        let rot = session.doc.objects.get(&page_ids[0]).and_then(|o| o.as_dict().ok()).and_then(|d| d.get(b"Rotate").ok()).and_then(|r| r.as_i64().ok()).unwrap_or(0);
        assert_eq!(rot, 270);

        // Undo 3 times: 270 -> 180 -> 90 -> 0
        assert!(session.undo().unwrap());
        assert!(session.undo().unwrap());
        assert!(session.undo().unwrap());
        assert_eq!(session.redo_stack.len(), 3);

        let rot0 = session.doc.objects.get(&page_ids[0]).and_then(|o| o.as_dict().ok()).and_then(|d| d.get(b"Rotate").ok()).and_then(|r| r.as_i64().ok()).unwrap_or(0);
        assert_eq!(rot0, 0);

        // Multi-step Redo 3 times: 0 -> 90 -> 180 -> 270
        // If redo stack was accidentally cleared on redo, this would fail on 2nd or 3rd redo
        assert!(session.redo().unwrap());
        assert_eq!(session.redo_stack.len(), 2);

        assert!(session.redo().unwrap());
        assert_eq!(session.redo_stack.len(), 1);

        assert!(session.redo().unwrap());
        assert_eq!(session.redo_stack.len(), 0);

        let rot_final = session.doc.objects.get(&page_ids[0]).and_then(|o| o.as_dict().ok()).and_then(|d| d.get(b"Rotate").ok()).and_then(|r| r.as_i64().ok()).unwrap_or(0);
        assert_eq!(rot_final, 270);
    }

    #[test]
    fn test_undo_atomicity_on_corrupt_snapshot() {
        let pdf = dummy_pdf();
        let manager = SessionManager::new();
        let id = manager.create_session(&pdf).expect("Create session");
        let session_arc = manager.get_session(&id).expect("Get session");
        let mut session = session_arc.write().unwrap();

        // Push a corrupt snapshot command
        session.push_undo(EditCommand::FullSnapshot {
            description: "Corrupted edit".into(),
            data: b"not a valid pdf".to_vec(),
        });
        assert_eq!(session.undo_stack.len(), 1);
        assert_eq!(session.redo_stack.len(), 0);

        // Attempt undo: restoring corrupt data fails
        let res = session.undo();
        assert!(res.is_err());

        // Atomicity check: because undo failed, the command was NOT lost from the stack
        assert_eq!(session.undo_stack.len(), 1);
        assert_eq!(session.redo_stack.len(), 0);
    }

    #[test]
    fn test_serialization_cache_invalidation() {
        let pdf = dummy_pdf();
        let manager = SessionManager::new();
        let id = manager.create_session(&pdf).expect("Create session");
        let session_arc = manager.get_session(&id).expect("Get session");
        let mut session = session_arc.write().unwrap();

        // First save serializes and caches
        let b1 = session.save_to_bytes().unwrap();
        // Second save reuses cached bytes without re-serializing
        let b2 = session.save_to_bytes().unwrap();
        assert_eq!(b1.len(), b2.len());

        // Modifying doc invalidates cache
        crate::pdf_engine::rotate_page_in_doc(&mut session.doc, 0, 90).unwrap();
        session.push_undo(EditCommand::RotatePage {
            page: 0,
            from_degrees: 0,
            to_degrees: 90,
        });

        // After modification, save_to_bytes re-serializes with updated state
        let b3 = session.save_to_bytes().unwrap();
        assert_ne!(b1, b3);
    }

    #[test]
    fn test_mutate_error_invalidates_cache_and_marks_dirty() {
        let pdf = dummy_pdf();
        let manager = SessionManager::new();
        let id = manager.create_session(&pdf).expect("Create session");
        let session_arc = manager.get_session(&id).expect("Get session");
        let mut session = session_arc.write().unwrap();

        // Warm up cache
        let _ = session.save_to_bytes().unwrap();
        assert!(session.cached_bytes.is_some());
        session.dirty = false;

        // mutate with closure returning Err
        let err_res: Result<(), String> = session.mutate(|_doc| {
            Err("Partial failure during operation".to_string())
        });
        assert!(err_res.is_err());

        // Cache must be invalidated and dirty marked to prevent phantom cache
        assert!(session.cached_bytes.is_none());
        assert!(session.dirty);
    }

    #[test]
    fn test_rotate_textedit_undo_regression() {
        let pdf = dummy_pdf();
        let manager = SessionManager::new();
        let id = manager.create_session(&pdf).expect("Create session");
        let session_arc = manager.get_session(&id).expect("Get session");

        // Helper to extract rotation of page 0
        let get_rotation = |doc: &lopdf::Document| -> i32 {
            let p_ids = doc.page_iter().collect::<Vec<_>>();
            doc.objects.get(&p_ids[0])
                .and_then(|o| o.as_dict().ok())
                .and_then(|d| d.get(b"Rotate").ok())
                .and_then(|r| r.as_i64().ok())
                .unwrap_or(0) as i32
        };

        // Helper to extract text from page 0
        let get_text = |doc: &lopdf::Document| -> String {
            let p_ids = doc.page_iter().collect::<Vec<_>>();
            let p0 = p_ids[0];
            let dict = doc.objects.get(&p0).unwrap().as_dict().unwrap();
            let c_id = dict.get(b"Contents").unwrap().as_reference().unwrap();
            let stream = doc.objects.get(&c_id).unwrap().as_stream().unwrap();
            let content = lopdf::content::Content::decode(&stream.content).unwrap();
            let mut s = String::new();
            for op in content.operations {
                if op.operator == "Tj" {
                    if let Some(lopdf::Object::String(bytes, _)) = op.operands.first() {
                        s.push_str(&String::from_utf8_lossy(bytes));
                    }
                }
            }
            s
        };

        // 0. Initial State: rotation = 0, text = "Hello World"
        {
            let s = session_arc.read().unwrap();
            assert_eq!(get_rotation(&s.doc), 0);
            assert_eq!(get_text(&s.doc), "Hello World");
        }

        // 1. Rotate page (fast path in session) -> rotation = 90, text = "Hello World"
        {
            let mut s = session_arc.write().unwrap();
            let from_deg = get_rotation(&s.doc);
            crate::pdf_engine::rotate_page_in_doc(&mut s.doc, 0, 90).unwrap();
            s.push_undo(EditCommand::RotatePage {
                page: 0,
                from_degrees: from_deg,
                to_degrees: from_deg + 90,
            });
            assert_eq!(get_rotation(&s.doc), 90);
            assert_eq!(get_text(&s.doc), "Hello World");
        }

        // 2. TextEdit using real `crate::pdf_engine::edit_text_block` on session bytes
        // and updating session in-place with FullSnapshot -> rotation = 90, text = "Hello Antigravity"
        {
            let mut s = session_arc.write().unwrap();
            let current_bytes = s.save_to_bytes().unwrap();

            // Run the actual engine function `edit_text_block`
            let updated_bytes = crate::pdf_engine::edit_text_block(
                &current_bytes,
                0,
                0,
                "Hello Antigravity",
            ).expect("edit_text_block should succeed");

            s.push_undo(EditCommand::FullSnapshot {
                description: "Edit text block #0".into(),
                data: current_bytes,
            });
            s.doc = lopdf::Document::load_mem(&updated_bytes).unwrap();
            s.dirty = true;
            s.invalidate_cache();

            // CRITICAL ASSERTION: Prior rotation (90 deg) must NOT be lost after text edit!
            assert_eq!(get_rotation(&s.doc), 90);
            assert_eq!(get_text(&s.doc), "Hello Antigravity");
            assert_eq!(s.undo_stack.len(), 2);
        }

        // 3. Undo #1 (Reverts TextEdit) -> rotation = 90, text = "Hello World"
        {
            let mut s = session_arc.write().unwrap();
            assert!(s.undo().unwrap());
            assert_eq!(s.undo_stack.len(), 1);
            assert_eq!(s.redo_stack.len(), 1);

            // Verified: text reverted, but rotation remains 90
            assert_eq!(get_rotation(&s.doc), 90);
            assert_eq!(get_text(&s.doc), "Hello World");
        }

        // 4. Undo #2 (Reverts Rotate) -> rotation = 0, text = "Hello World"
        {
            let mut s = session_arc.write().unwrap();
            assert!(s.undo().unwrap());
            assert_eq!(s.undo_stack.len(), 0);
            assert_eq!(s.redo_stack.len(), 2);

            // Verified: back to initial state
            assert_eq!(get_rotation(&s.doc), 0);
            assert_eq!(get_text(&s.doc), "Hello World");
        }

        // 5. Redo #1 (Reapplies Rotate) -> rotation = 90, text = "Hello World"
        {
            let mut s = session_arc.write().unwrap();
            assert!(s.redo().unwrap());
            assert_eq!(s.undo_stack.len(), 1);
            assert_eq!(s.redo_stack.len(), 1);

            assert_eq!(get_rotation(&s.doc), 90);
            assert_eq!(get_text(&s.doc), "Hello World");
        }

        // 6. Redo #2 (Reapplies TextEdit) -> rotation = 90, text = "Hello Antigravity"
        {
            let mut s = session_arc.write().unwrap();
            assert!(s.redo().unwrap());
            assert_eq!(s.undo_stack.len(), 2);
            assert_eq!(s.redo_stack.len(), 0);

            assert_eq!(get_rotation(&s.doc), 90);
            assert_eq!(get_text(&s.doc), "Hello Antigravity");
        }

        // 7. Save / Serialize & Re-load from raw bytes -> verify persisted Document
        {
            let mut s = session_arc.write().unwrap();
            let saved_bytes = s.save_to_bytes().expect("Serialization must succeed");
            let reloaded_doc = lopdf::Document::load_mem(&saved_bytes).expect("Reloading must succeed");

            assert_eq!(get_rotation(&reloaded_doc), 90, "Serialized document must preserve 90 deg rotation");
            assert_eq!(get_text(&reloaded_doc), "Hello Antigravity", "Serialized document must preserve edited text");
        }
    }
}

