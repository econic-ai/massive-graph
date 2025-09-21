use massive_graph_core::types::{
    storage::{DocumentHeaderChunkRef, DocumentHeaderStorage, WireFormat},
    document::DocumentHeader,
    DocId, UserId,
};

fn main() {
    // Example showing how DocumentHeader parses all values upfront
    
    // Simulate wire format bytes (in practice, these come from storage)
    let mut wire_bytes = vec![0u8; 64];
    
    // Write some test data at the defined offsets
    // Wire version at offset 0 (u16)
    wire_bytes[0..2].copy_from_slice(&1u16.to_le_bytes());
    
    // DocId at offset 2 (16 bytes)
    let doc_id = DocId::new();
    unsafe {
        let doc_id_bytes = std::slice::from_raw_parts(
            &doc_id as *const DocId as *const u8,
            std::mem::size_of::<DocId>()
        );
        wire_bytes[2..18].copy_from_slice(doc_id_bytes);
    }
    
    // Document type at offset 18 (1 byte)
    wire_bytes[18] = 1; // DocumentType::Document
    
    // Owner ID at offset 19 (16 bytes)
    let owner_id = UserId::new();
    unsafe {
        let owner_id_bytes = std::slice::from_raw_parts(
            &owner_id as *const UserId as *const u8,
            std::mem::size_of::<UserId>()
        );
        wire_bytes[19..35].copy_from_slice(owner_id_bytes);
    }
    
    // Created at timestamp at offset 35 (8 bytes)
    let created_at = 1234567890u64;
    wire_bytes[35..43].copy_from_slice(&created_at.to_le_bytes());
    
    // Parse the header - all values are parsed upfront
    let header = DocumentHeader::from_bytes(&wire_bytes);
    
    // Access the parsed values - no more byte slicing needed
    println!("Wire version: {}", header.wire_version());
    println!("Document ID: {:?}", header.doc_id());
    println!("Document type: {:?}", header.doc_type());
    println!("Owner ID: {:?}", header.owner_id());
    println!("Created at: {}", header.created_at());
    
    // The header is now a simple struct with all values in memory
    // No lifetime complications, just owned data
}

