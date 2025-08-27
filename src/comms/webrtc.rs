// use std::sync::Arc;
// use bytes::Bytes;
// use crate::types::{DocId, DeltaId};

// async fn on_rtc_message(
//     app: Arc<DeltaProcessor>,
//     client_id: ClientId,
//     bytes: Bytes,
// ) {
//     tokio::spawn(async move {
//         // Quick structural validation (~100ns)
//         if !is_valid_wire_format(&bytes) {
//             return; // Reject malformed deltas
//         }
        
//         // Parse document ID for routing
//         let doc_id = parse_doc_id(&bytes);
        
//         // Allocate in chunk - immutable from here (~100ns)
//         let chunk_ref = app.chunk_storage.allocate(&bytes);
        
//         // Route to validation worker (sharded)
//         let worker_id = hash(doc_id) % app.core_workers.len();
//         app.core_workers[worker_id]
//             .inbox
//             .push(DeltaTask { chunk_ref, doc_id, client_id })
//             .expect("Worker queue full");
//     });
// }



// struct ClientConnection {
//     peer_connection: Arc<RTCPeerConnection>,
    
//     // Dedicated channel per subscribed document
//     document_channels: HashMap<DocId, Arc<RTCDataChannel>>,
    
//     // Shared channel for low-activity documents
//     shared_channel: Arc<RTCDataChannel>,
    
//     // Statistics for channel promotion/demotion
//     channel_stats: HashMap<DocId, ChannelStats>,
// }

// impl ClientConnection {
//     async fn ensure_dedicated_channel(&mut self, doc_id: DocId) {
//         if !self.document_channels.contains_key(&doc_id) {
//             // Create dedicated DataChannel for this document
//             let channel = self.peer_connection
//                 .create_data_channel(
//                     &format!("doc-{}", doc_id),
//                     Some(RTCDataChannelInit {
//                         ordered: Some(false),        // Unordered for speed
//                         max_retransmits: Some(0),    // Unreliable
//                         protocol: Some("delta-stream"),
//                         ..Default::default()
//                     })
//                 )
//                 .await
//                 .expect("Failed to create channel");
            
//             self.document_channels.insert(doc_id, Arc::new(channel));
//         }
//     }
// }

