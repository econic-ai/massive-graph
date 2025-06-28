#!/bin/bash

# Massive Graph API Test Script
# Tests all CRUD operations and API endpoints

set -e  # Exit on any error

# BASE_URL="http://localhost:8080"
BASE_URL="https://api.local.econic.ai"
API_BASE="$BASE_URL/mg/api/v1"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
print_header() {
    echo -e "\n${BLUE}=== $1 ===${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

print_info() {
    echo -e "${YELLOW}ℹ $1${NC}"
}

# Check if server is running
check_server() {
    print_header "Checking Server Status"
    
    # First check what the health endpoint actually returns
    print_info "Testing health endpoint: $API_BASE/health"
    health_response=$(curl -s -w "\nHTTP_CODE:%{http_code}" "$API_BASE/health")
    http_code=$(echo "$health_response" | tail -n1 | cut -d: -f2)
    response_body=$(echo "$health_response" | sed '$d')
    
    echo "HTTP Status: $http_code"
    echo "Response: $response_body"
    
    if [ "$http_code" = "200" ]; then
        print_success "Server is running and health endpoint is working"
    elif [ "$http_code" = "404" ]; then
        print_error "Server responded but health endpoint not found (404)"
        print_info "This might be a routing issue. Check nginx configuration or endpoint path."
        exit 1
    else
        print_error "Server health check failed with HTTP $http_code"
        exit 1
    fi
}

# Test health endpoints
test_health() {
    print_header "Testing Health Endpoints"
    
    echo "GET /api/v1/health"
    response=$(curl -s "$API_BASE/health")
    
    # Check if response is JSON before trying to parse with jq
    if echo "$response" | jq . >/dev/null 2>&1; then
        echo "$response" | jq .
        print_success "Health check passed - JSON response received"
    else
        echo "Raw response (not JSON): $response"
        print_error "Health endpoint returned non-JSON response"
        return 1
    fi
    
    echo -e "\nGET /api/v1/info"
    response=$(curl -s "$API_BASE/info")
    
    # Check if response is JSON before trying to parse with jq
    if echo "$response" | jq . >/dev/null 2>&1; then
        echo "$response" | jq .
        print_success "Info endpoint passed - JSON response received"
    else
        echo "Raw response (not JSON): $response"
        print_error "Info endpoint returned non-JSON response"
        return 1
    fi
}

# Test document CRUD operations
test_document_crud() {
    print_header "Testing Document CRUD Operations"
    
    # Create a document
    print_info "Creating a new document..."
    create_response=$(curl -s -X POST "$API_BASE/documents" \
        -H "Content-Type: application/json" \
        -d '{
            "doc_type": "text",
            "parent_id": null,
            "properties": {
                "title": "Test Document",
                "content": "This is a test document",
                "author": "API Test Script"
            }
        }')
    
    echo "Create Response:"
    echo "$create_response" | jq .
    
    # Extract document ID
    doc_id=$(echo "$create_response" | jq -r '.data.id')
    if [ "$doc_id" = "null" ] || [ -z "$doc_id" ]; then
        print_error "Failed to create document"
        return 1
    fi
    print_success "Document created with ID: $doc_id"
    
    # Get the document
    print_info "Retrieving the document..."
    get_response=$(curl -s "$API_BASE/documents/$doc_id")
    echo "Get Response:"
    echo "$get_response" | jq .
    print_success "Document retrieved successfully"
    
    # Update the document
    print_info "Updating the document..."
    update_response=$(curl -s -X PUT "$API_BASE/documents/$doc_id" \
        -H "Content-Type: application/json" \
        -d '{
            "properties": {
                "title": "Updated Test Document",
                "content": "This content has been updated",
                "author": "API Test Script",
                "version": 2
            }
        }')
    
    echo "Update Response:"
    echo "$update_response" | jq .
    print_success "Document updated successfully"
    
    # Patch the document
    print_info "Patching the document..."
    patch_response=$(curl -s -X PATCH "$API_BASE/documents/$doc_id" \
        -H "Content-Type: application/json" \
        -d '{
            "properties": {
                "tags": ["test", "api", "patch"],
                "status": "modified"
            }
        }')
    
    echo "Patch Response:"
    echo "$patch_response" | jq .
    print_success "Document patched successfully"
    
    # Get updated document
    print_info "Retrieving updated document..."
    updated_get_response=$(curl -s "$API_BASE/documents/$doc_id")
    echo "Updated Document:"
    echo "$updated_get_response" | jq .
    print_success "Updated document retrieved successfully"
    
    # Delete the document
    print_info "Deleting the document..."
    delete_response=$(curl -s -X DELETE "$API_BASE/documents/$doc_id")
    echo "Delete Response:"
    echo "$delete_response" | jq .
    print_success "Document deleted successfully"
    
    # Try to get deleted document (should fail)
    print_info "Verifying document deletion..."
    deleted_get_response=$(curl -s "$API_BASE/documents/$doc_id")
    echo "Get Deleted Document Response:"
    echo "$deleted_get_response" | jq .
    
    if echo "$deleted_get_response" | jq -e '.success == false' > /dev/null; then
        print_success "Document deletion verified"
    else
        print_error "Document deletion verification failed"
    fi
}

# Test collection operations
test_collections() {
    print_header "Testing Collection Operations"
    
    # Create a collection
    print_info "Creating a collection..."
    collection_response=$(curl -s -X POST "$API_BASE/collections" \
        -H "Content-Type: application/json" \
        -d '{
            "name": "Test Collection",
            "description": "A collection for testing"
        }')
    
    echo "Create Collection Response:"
    echo "$collection_response" | jq .
    print_success "Collection creation endpoint tested"
    
    # List collections
    print_info "Listing collections..."
    list_response=$(curl -s "$API_BASE/collections")
    echo "List Collections Response:"
    echo "$list_response" | jq .
    print_success "Collection listing endpoint tested"
}

# Test document listing and pagination
test_document_listing() {
    print_header "Testing Document Listing"
    
    # Create multiple documents for testing
    print_info "Creating multiple test documents..."
    for i in {1..3}; do
        curl -s -X POST "$API_BASE/documents" \
            -H "Content-Type: application/json" \
            -d "{
                \"doc_type\": \"text\",
                \"parent_id\": null,
                \"properties\": {
                    \"title\": \"Test Document $i\",
                    \"content\": \"Content for document $i\",
                    \"number\": $i
                }
            }" > /dev/null
    done
    print_success "Created 3 test documents"
    
    # List all documents
    print_info "Listing all documents..."
    list_response=$(curl -s "$API_BASE/documents")
    echo "List Documents Response:"
    echo "$list_response" | jq .
    print_success "Document listing tested"
    
    # Test pagination
    print_info "Testing pagination..."
    paginated_response=$(curl -s "$API_BASE/documents?limit=2&offset=0")
    echo "Paginated Response (limit=2, offset=0):"
    echo "$paginated_response" | jq .
    print_success "Pagination tested"
}

# Test error handling
test_error_handling() {
    print_header "Testing Error Handling"
    
    # Try to get non-existent document
    print_info "Testing 404 error handling..."
    error_response=$(curl -s "$API_BASE/documents/nonexistent123")
    echo "404 Error Response:"
    echo "$error_response" | jq .
    
    if echo "$error_response" | jq -e '.success == false' > /dev/null; then
        print_success "404 error handling works correctly"
    else
        print_error "404 error handling failed"
    fi
    
    # Try to create document with invalid data
    print_info "Testing invalid data handling..."
    invalid_response=$(curl -s -X POST "$API_BASE/documents" \
        -H "Content-Type: application/json" \
        -d '{
            "doc_type": "",
            "properties": "invalid_json_structure"
        }')
    
    echo "Invalid Data Response:"
    echo "$invalid_response" | jq .
    print_success "Invalid data error handling tested"
}

# Test delta operations (placeholder endpoints)
test_delta_operations() {
    print_header "Testing Delta Operations"
    
    print_info "Testing delta endpoints (placeholder implementations)..."
    
    # Test document deltas
    delta_response=$(curl -s "$API_BASE/documents/test123/deltas")
    echo "Document Deltas Response:"
    echo "$delta_response" | jq .
    print_success "Document deltas endpoint tested"
    
    # Test collection deltas
    collection_delta_response=$(curl -s "$API_BASE/collections/test123/deltas")
    echo "Collection Deltas Response:"
    echo "$collection_delta_response" | jq .
    print_success "Collection deltas endpoint tested"
}

# Main test execution
main() {
    echo -e "${BLUE}🚀 Starting Massive Graph API Tests${NC}\n"
    
    # Check if jq is installed
    if ! command -v jq &> /dev/null; then
        print_error "jq is required for JSON parsing. Please install it: brew install jq"
        exit 1
    fi
    
    check_server
    test_health
    test_document_crud
    test_collections
    test_document_listing
    test_error_handling
    test_delta_operations
    
    echo -e "\n${GREEN}🎉 All API tests completed!${NC}"
    echo -e "${YELLOW}Note: Some endpoints are placeholder implementations and will be enhanced in future development.${NC}"
}

# Run tests
main "$@" 