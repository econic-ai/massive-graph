/// Utility functions for common operations across the codebase

use massive_graph_core::types::DocId;

/// Hash a document ID to a u64
pub fn doc_hash_u64(doc_id: &DocId) -> u64 {
    doc_id.as_bytes().iter().fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64))
}

pub fn shard_id_from_doc_hash(doc_hash: u64, shard_count: u16) -> u16 {
    (doc_hash % shard_count as u64) as u16
}