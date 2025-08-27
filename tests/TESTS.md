# Massive Graph API Tests

## Implemented Tests

### Health Check Flow
- **Health Check** - *Implemented* - Validates server is running and responds with healthy status. Must return 200 with `{"status":"healthy"}`.
- **System Info** - *Implemented* - Validates system information endpoint. Must return system name and capabilities array.

### Flow 1: Document Lifecycle
- **Create Document** - *Implemented* - Creates text document with properties. Must return 201 with valid 16-byte hex ID and exact property matching.
- **Get Document** - *Implemented* - Retrieves created document. Must return exact document data with proper timestamps and versioning.
- **Delete Document** - *Implemented* - Removes document from storage. Must return 204 No Content.
- **Get Deleted Document** - *Implemented* - Attempts to retrieve deleted document. Must return 404 Not Found with proper error structure.

### Flow 2: Delta Operations
- **Create Document with Data** - *Implemented* - Creates JSON document with nested data structure. Must return valid JSON document type with proper ID.
- **Apply Deltas** - *Implemented* - Applies multiple delta operations to document. Must process property_set and property_increment operations with valid delta IDs.
- **Get Latest Document** - *Implemented* - Retrieves document after delta application. Must show all deltas applied to document data with version increment.
- **Get Delta History** - *Implemented* - Retrieves delta history for document. Must return exactly applied deltas with sequence ordering.

### Error Handling Tests
- **Update Document (PUT)** - *Implemented* - Updates document properties. Must increment version and update timestamps with exact property matching.
- **Test Non-existent Document** - *Implemented* - Requests document that doesn't exist. Must return 404 with proper error message.
- **Test Invalid Delta Operations** - *Implemented* - Sends invalid delta operation type. Must return 400 Bad Request with error details.
- **Test Malformed JSON Request** - *Implemented* - Sends malformed JSON. Must return 400 Bad Request for parsing errors.

## Planned Tests

### Flow 3: Document Hierarchies
- **Create Parent Document** - *Planned* - Create document with no parent. Must support null parent_id.
- **Create Child Document** - *Planned* - Create document with parent reference. Must validate parent exists and update parent child_count.
- **Get Document Children** - *Planned* - Retrieve all children of a document. Must return paginated list of child documents.
- **Move Document** - *Planned* - Change document parent. Must update both old and new parent child counts.

### Flow 4: Document Types
- **Create Binary Document** - *Planned* - Create document with binary content. Must handle binary data storage and retrieval.
- **Create Graph Document** - *Planned* - Create hypergraph root container. Must support graph-specific metadata.
- **Create Node Document** - *Planned* - Create graph node. Must support node-specific properties and edge references.
- **Create Edge Document** - *Planned* - Create hyperedge connecting multiple nodes. Must validate connected nodes exist.

### Flow 5: Advanced Delta Operations
- **String Insert/Remove** - *Planned* - Apply text editing deltas. Must support character-level string mutations.
- **Array Operations** - *Planned* - Add/remove/move array elements. Must support array manipulation deltas.
- **Child Operations** - *Planned* - Add/remove/move child documents. Must update parent-child relationships.
- **Stream Operations** - *Planned* - Append to binary/text/document streams. Must handle append-only stream data.

### Flow 6: Stream Documents
- **Create Binary Stream** - *Planned* - Create append-only binary stream. Must support stream entry storage with timestamps.
- **Append to Binary Stream** - *Planned* - Add binary data to stream. Must generate stream entry references.
- **Create Text Stream** - *Planned* - Create append-only text stream. Must handle text entries with metadata.
- **Create Document Stream** - *Planned* - Create stream of document references. Must store document IDs with timestamps.

### Flow 7: Index Operations
- **Create Type Index** - *Planned* - Create index by document type. Must enable fast type-based queries.
- **Create Name Index** - *Planned* - Create index by document names/tags. Must support name-based lookups.
- **Create Property Index** - *Planned* - Create index by property values. Must enable property-based queries.
- **Query by Type** - *Planned* - Search documents by type. Must return filtered results efficiently.

### Flow 8: Concurrency
- **Concurrent Document Creation** - *Planned* - Multiple simultaneous document creations. Must handle concurrent access safely.
- **Concurrent Delta Application** - *Planned* - Multiple deltas on same document. Must apply deltas in sequence order.
- **Document Locking** - *Planned* - Test exclusive document access. Must prevent concurrent modifications.

### Flow 9: Performance
- **Large Document Creation** - *Planned* - Create documents with large property sets. Must handle size limits gracefully.
- **Bulk Delta Operations** - *Planned* - Apply many deltas in single request. Must process efficiently.
- **High Frequency Updates** - *Planned* - Rapid delta application. Must maintain performance under load.

### Flow 10: Security
- **Invalid Document ID** - *Planned* - Request with malformed ID format. Must return 400 Bad Request.
- **SQL Injection Attempts** - *Planned* - Send malicious input in properties. Must sanitize input safely.
- **Oversized Requests** - *Planned* - Send extremely large payloads. Must reject with appropriate limits.

### Flow 11: Wire Format
- **Binary Document Serialization** - *Planned* - Test wire format output. Must produce valid binary representation.
- **Zero-Copy Document Access** - *Planned* - Validate zero-copy parsing. Must access data without full deserialization.
- **Wire Format Backwards Compatibility** - *Planned* - Test version compatibility. Must handle format evolution.
