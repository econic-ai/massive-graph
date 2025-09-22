//! UserSpace - Single user storage, indexes, and streams
use std::sync::Arc;
use crate::core::utils::current_timestamp;
use crate::storage::ZeroCopyDocumentStorage;
use crate::types::{UserId, DocId};
use crate::DocumentStorage;
use crate::{log_info};
use crate::structures::optimised_index::{OptimisedIndex, Snapshot, MphIndexer, DeltaOp};
use crate::structures::segmented_stream::SegmentedStream;


/// UserSpace - per-user storage, document index, and document streams
/// Holds the user's document lookup index and document append-only stream with cursor.
pub struct UserSpace {
    
    /// The user ID this isolate represents
    user_id: UserId,
    
    /// Document index
    doc_index: OptimisedIndex<DocId, ZeroCopyDocumentStorage>, 

    /// User subscriptions
    subsriptions: Vec<DocId>,
    
    /// Connections
    connections: Vec<DocId>,

    /// Queues
    queues: Vec<DocId>,

    /// Metadata  
    stats: UserSpaceStats,
}

/// user space stats
#[derive(Clone)]
pub struct UserSpaceStats {
    /// The user ID this isolate represents
    user_id: UserId,
    /// The total number of bytes used by the user space
    total_bytes_used: u64,
    /// The quota of bytes for the user space
    quota_bytes: u64,
    /// The timestamp when the user space was created
    created_at: u64,
}

impl UserSpaceStats {
    /// Create a new user space stats
    pub fn new(user_id: UserId, total_bytes_used: u64, quota_bytes: u64) -> Self {
        let created_at = current_timestamp();
        Self { user_id, total_bytes_used, quota_bytes, created_at }
    }
}


impl<'a> UserSpace {
    /// Create a new user space for a specific user
    pub fn new(user_id: UserId) -> Self {
        // Create a sentinel head node for the user's document stream.
        // let sentinel: *mut UserDocNode<'static> = UserDocNode::boxed(UserDocumentRef { bytes: &[], doc_id: DocId::default() });
        // let user_docs_stream = UserDocStream::new(sentinel);
        // let user_view = UserView::new(user_id, sentinel);
        // Minimal empty snapshot + delta stream for placeholder wiring
        struct DummyMph;
        impl MphIndexer<DocId> for DummyMph { fn eval(&self, _key: &DocId) -> usize { 0 } }
        let snapshot = Snapshot {
            version: 0,
            reserved_keys: Arc::from([]),
            reserved_vals: Arc::from([]),
            mph_vals: Arc::from([]),
            mph_indexer: crate::structures::optimised_index::ArcIndexer(Arc::new(DummyMph)),
        };
        let delta_stream = Arc::new(SegmentedStream::<DeltaOp<DocId, ZeroCopyDocumentStorage>>::new());
        Self {
            user_id,
            // user_docs_stream,
            // user_view,
            doc_index: OptimisedIndex::new(snapshot, delta_stream),
            connections: Vec::new(),
            queues: Vec::new(),
            subsriptions: Vec::new(),
            stats: UserSpaceStats::new(user_id, 0, 0)
        }
    }
    
    /// Get the user ID this isolate represents
    pub fn user_id(&self) -> UserId {
        self.user_id
    }
    
    /// Get document count for this user
    pub fn document_count(&self) -> usize {
        self.doc_index.len()
    }
    
    /// Create a document for this user
    pub fn create_document(&self, doc_id: DocId, doc_data: Vec<u8>) -> Result<(), String> {
        log_info!("🔒 UserDocumentSpace::create_document - user: {}, doc: {}, data_size: {}", self.user_id, doc_id, doc_data.len());
        Ok(())
    }
    
    /// Get a document for this user as bytes (compat shim over DocumentView)
    pub fn get_document(&self, doc_id: DocId) -> Option<Arc<ZeroCopyDocumentStorage>> {
        // if let Some(view) = self.doc_index.get(&doc_id) { return Some(view.to_bytes()); }
        self.doc_index.get(&doc_id)
    }
    
    /// Remove a document for this user
    pub fn remove_document(&self, doc_id: DocId) -> () {
        self.doc_index.remove(&doc_id)
    }
    
    /// Check if a document exists for this user
    pub fn document_exists(&self, doc_id: DocId) -> bool {
        self.doc_index.contains_key(&doc_id)
    }
    
    /// Apply a delta to a document for this user
    pub fn apply_delta(&self, doc_id: DocId, delta: Vec<u8>) -> Result<(), String> {
        self.doc_index.get(&doc_id).unwrap().apply_delta(delta)
    }

    /// get stats and runtime info
    pub fn stats(&self) -> UserSpaceStats {
        self.stats.clone()
    }
    

}

