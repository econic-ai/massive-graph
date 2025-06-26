# Quick API Test Commands

Quick curl commands for testing the Massive Graph API during development.

## Prerequisites

```bash
# Start the server
cargo run

# Install jq for JSON formatting (optional but recommended)
brew install jq
```

## Health Check

```bash
# Check if server is running
curl http://localhost:8080/api/v1/health | jq

# Get system info
curl http://localhost:8080/api/v1/info | jq
```

## Document Operations

### Create Document
```bash
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d '{
    "doc_type": "text",
    "parent_id": null,
    "properties": {
      "title": "My Test Document",
      "content": "Hello, World!",
      "author": "Developer"
    }
  }' | jq
```

### Get Document
```bash
# Replace {id} with actual document ID from create response
curl http://localhost:8080/api/v1/documents/{id} | jq
```

### Update Document (Full Replace)
```bash
curl -X PUT http://localhost:8080/api/v1/documents/{id} \
  -H "Content-Type: application/json" \
  -d '{
    "properties": {
      "title": "Updated Document",
      "content": "Updated content",
      "author": "Developer",
      "version": 2
    }
  }' | jq
```

### Patch Document (Partial Update)
```bash
curl -X PATCH http://localhost:8080/api/v1/documents/{id} \
  -H "Content-Type: application/json" \
  -d '{
    "properties": {
      "tags": ["test", "api"],
      "status": "modified"
    }
  }' | jq
```

### Delete Document
```bash
curl -X DELETE http://localhost:8080/api/v1/documents/{id} | jq
```

### List Documents
```bash
# List all documents
curl http://localhost:8080/api/v1/documents | jq

# List with pagination
curl "http://localhost:8080/api/v1/documents?limit=5&offset=0" | jq
```

## Collection Operations

### Create Collection
```bash
curl -X POST http://localhost:8080/api/v1/collections \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Collection",
    "description": "A collection for testing"
  }' | jq
```

### List Collections
```bash
curl http://localhost:8080/api/v1/collections | jq
```

## Error Testing

### Test 404 Error
```bash
curl http://localhost:8080/api/v1/documents/nonexistent | jq
```

### Test Invalid Data
```bash
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d '{
    "doc_type": "",
    "properties": "invalid"
  }' | jq
```

## Delta Operations (Placeholder)

### Get Document Deltas
```bash
curl http://localhost:8080/api/v1/documents/test123/deltas | jq
```

### Get Collection Deltas
```bash
curl http://localhost:8080/api/v1/collections/test123/deltas | jq
```

## Testing Different Document Types

### Create Binary Document
```bash
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d '{
    "doc_type": "binary",
    "parent_id": null,
    "properties": {
      "filename": "test.jpg",
      "mimetype": "image/jpeg",
      "size": 1024
    }
  }' | jq
```

### Create Graph Document
```bash
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d '{
    "doc_type": "graph",
    "parent_id": null,
    "properties": {
      "name": "Test Graph",
      "type": "directed",
      "node_count": 0,
      "edge_count": 0
    }
  }' | jq
```

## Storage Configuration Testing

```bash
# Test different storage types (these should fail for unsupported types)
cargo run -- --storage-type memory    # Should work
cargo run -- --storage-type disk      # Should fail (not implemented)
cargo run -- --storage-type invalid   # Should fail with validation error
```

## Quick Test Sequence

For a quick smoke test, run these commands in order:

```bash
# 1. Health check
curl http://localhost:8080/api/v1/health

# 2. Create document
DOC_ID=$(curl -s -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d '{"doc_type": "text", "properties": {"title": "Quick Test"}}' | \
  jq -r '.data.id')

# 3. Get document
curl http://localhost:8080/api/v1/documents/$DOC_ID | jq

# 4. Update document
curl -X PATCH http://localhost:8080/api/v1/documents/$DOC_ID \
  -H "Content-Type: application/json" \
  -d '{"properties": {"status": "tested"}}' | jq

# 5. Delete document
curl -X DELETE http://localhost:8080/api/v1/documents/$DOC_ID | jq
``` 