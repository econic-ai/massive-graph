#!/bin/bash

# Configuration
COLLECTION_ID="629886-7db406a5-ce98-4631-a5a1-db8d0557ad28"
FILE="poc.postman_collection.json"
LAST_SYNC_FILE="/tmp/.postman_sync_timestamp"
MIN_SYNC_INTERVAL=10  # seconds

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to check if enough time has passed since last sync
check_rate_limit() {
    if [ -f "$LAST_SYNC_FILE" ]; then
        last_sync=$(cat "$LAST_SYNC_FILE")
        current_time=$(date +%s)
        time_diff=$((current_time - last_sync))
        
        if [ $time_diff -lt $MIN_SYNC_INTERVAL ]; then
            remaining=$((MIN_SYNC_INTERVAL - time_diff))
            echo -e "${YELLOW}⏳ Rate limit: Waiting ${remaining}s before next sync...${NC}"
            return 1
        fi
    fi
    return 0
}

# Function to validate JSON
validate_json() {
    if ! jq empty "$FILE" 2>/dev/null; then
        echo -e "${RED}❌ Invalid JSON in $FILE${NC}"
        
        # Try to show where the error is
        jq_error=$(jq empty "$FILE" 2>&1 | head -n 2)
        echo -e "${RED}Error details: $jq_error${NC}"
        return 1
    fi
    
    echo -e "${GREEN}✓ JSON validation passed${NC}"
    return 0
}

# Function to sync to Postman
sync_to_postman() {
    echo -e "${YELLOW}📤 Syncing to Postman...${NC}"
    
    # Create temporary file with wrapped collection
    temp_file=$(mktemp)
    echo '{"collection":' > "$temp_file"
    cat "$FILE" >> "$temp_file"
    echo '}' >> "$temp_file"
    
    # Make the API call
    response=$(curl -s -w "\n%{http_code}" -X PUT \
        "https://api.postman.com/collections/$COLLECTION_ID" \
        -H "X-Api-Key: $POSTMAN_API_KEY" \
        -H "Content-Type: application/json" \
        --data-binary "@$temp_file" 2>/dev/null)
    
    # Extract status code
    http_code=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | sed '$d')
    
    # Clean up temp file
    rm "$temp_file"
    
    # Check response
    if [ "$http_code" = "200" ]; then
        echo -e "${GREEN}✅ Successfully synced to Postman!${NC}"
        date +%s > "$LAST_SYNC_FILE"
        
        # Parse and show collection name if possible
        collection_name=$(jq -r '.collection.info.name // "Unknown"' "$FILE" 2>/dev/null)
        echo -e "${GREEN}   Collection: $collection_name${NC}"
        return 0
    else
        echo -e "${RED}❌ Sync failed with status code: $http_code${NC}"
        
        # Try to parse error message
        error_msg=$(echo "$body" | jq -r '.error.message // .message // "Unknown error"' 2>/dev/null)
        echo -e "${RED}   Error: $error_msg${NC}"
        return 1
    fi
}

# Main execution
main() {
    echo "========================================="
    echo "$(date '+%Y-%m-%d %H:%M:%S') - File change detected"
    
    # Check rate limit
    if ! check_rate_limit; then
        exit 0
    fi
    
    # Validate JSON
    if ! validate_json; then
        echo -e "${YELLOW}⚠️  Skipping sync due to invalid JSON${NC}"
        exit 1
    fi
    
    # Sync to Postman
    if sync_to_postman; then
        echo -e "${GREEN}🎉 Sync complete!${NC}"
    else
        echo -e "${RED}💥 Sync failed!${NC}"
        exit 1
    fi
}

# Check dependencies
if ! command -v jq &> /dev/null; then
    echo -e "${RED}Error: jq is not installed. Please install it first:${NC}"
    echo "  Ubuntu/Debian: sudo apt-get install jq"
    echo "  MacOS: brew install jq"
    echo "  RHEL/CentOS: sudo yum install jq"
    exit 1
fi

if ! command -v curl &> /dev/null; then
    echo -e "${RED}Error: curl is not installed${NC}"
    exit 1
fi

# Run main function
main