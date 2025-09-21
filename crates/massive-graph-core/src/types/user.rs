
use crate::types::{DocId, UserId};

/// Thin view of a user document reference in chunk storage.
pub struct UserDocumentRef<'a> {
    /// Zero-copy bytes that identify/describe the document reference.
    pub bytes: &'a [u8],
    /// The document identifier for this entry.
    pub doc_id: DocId,
}

/// Ephemeral, zero-copy user view with a per-user document stream and cursor.
pub struct UserView {
    /// The user identifier this view belongs to.
    user_id: UserId,
    // /// Head of the user's document stream (kept for diagnostics/reconstruction paths).
    // doc_head: *mut UserDocNode<'a>,
    // /// Cursor for resumable traversal.
    // doc_cursor: AtomicPtr<UserDocNode<'a>>,
}

impl<'a> UserView {
    /// Create a new user view from a user id and stream head.
    pub fn new(user_id: UserId) -> Self {
        // Self { user_id, doc_head, doc_cursor: AtomicPtr::new(doc_head) }
        Self { user_id }
    }

    /// The user identifier for this view.
    pub fn user_id(&self) -> UserId { self.user_id }

    // /// Head pointer accessor for diagnostics.
    // pub fn doc_head(&self) -> *mut UserDocNode<'a> { self.doc_head }

    // /// Get the current cursor for the user's document stream.
    // pub fn doc_cursor(&self) -> *mut UserDocNode<'a> { self.doc_cursor.load(Ordering::Acquire) }

    // /// Build next batch of user documents into an existing vector to reuse capacity; updates the cursor.
    // pub fn build_next_user_docs_into(&self, stream: &UserDocStream<'a>, max_scan: usize, out: &mut Vec<*mut UserDocNode<'a>>) {
    //     let start = self.doc_cursor.load(Ordering::Acquire);
    //     let next = stream.build_next_batch_into(start, max_scan, out);
    //     self.doc_cursor.store(next, Ordering::Release);
    // }
}
