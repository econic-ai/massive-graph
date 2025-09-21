use massive_graph_core::types::{
    storage::{DocumentHeaderChunkRef, DocumentHeaderStorage},
    document::DocumentHeader,
};

fn main() {
    // Example of how the wire format system works:
    
    // 1. Create storage for document headers
    let storage = DocumentHeaderStorage::new(1024 * 1024); // 1MB chunks
    
    // 2. Reserve space and write a document header
    let mut write_handle = storage.reserve(64).unwrap(); // Reserve 64 bytes
    
    // 3. Write the wire format data directly to the chunk
    let buffer = write_handle.buffer_mut();
    // ... write header data to buffer ...
    
    // 4. Commit and get a typed chunk reference
    let chunk_ref: DocumentHeaderChunkRef = write_handle.commit();
    
    // 5. Later, read the document header with zero-copy
    let header: DocumentHeader = chunk_ref.read();
    
    // 6. Access fields directly from the wire format
    let doc_id = header.doc_id();
    let created_at = header.created_at();
    
    println!("Document ID: {:?}", doc_id);
    println!("Created at: {}", created_at);
}

