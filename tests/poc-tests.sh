#!/bin/bash

# Newman Test Runner for Massive Graph API
# End-to-end tests that validate current functionality level

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
# BASE_URL="https://api.local.econic.ai/mg"
BASE_URL="http://localhost:8080"
COLLECTION="poc.postman_collection.json"
REPORT_DIR="reports"

# Function to print colored output
print_info() {
    echo -e "${BLUE}ℹ ${NC}$1"
}

print_success() {
    echo -e "${GREEN}✓ ${NC}$1"
}

print_error() {
    echo -e "${RED}✗ ${NC}$1"
}

# Check if newman is installed
check_newman() {
    if ! command -v newman &> /dev/null; then
        print_error "Newman is not installed!"
        print_info "Install with: npm install -g newman"
        exit 1
    fi
}

# Show usage
usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -u, --url URL        Base URL for API (default: $BASE_URL)"
    echo "  -r, --reporters      Reporters to use (default: cli)"
    echo "                       Options: cli, html, json, junit"
    echo "  -f, --folder NAME    Run specific test folder only"
    echo "  -h, --help           Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                   # Run with default settings"
    echo "  $0 -r cli,html       # Run with CLI and HTML reports"
    echo "  $0 -u http://localhost:3000  # Use different base URL"
    echo "  $0 -f \"Flow 1: Create-Get-Delete-Get\"  # Run specific flow"
}

# Parse command line arguments
REPORTERS="cli"
FOLDER=""

while [[ $# -gt 0 ]]; do
    case $1 in
        -u|--url)
            BASE_URL="$2"
            shift 2
            ;;
        -r|--reporters)
            REPORTERS="$2"
            shift 2
            ;;
        -f|--folder)
            FOLDER="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Main execution
main() {
    print_info "Starting Newman test runner for Massive Graph API"
    
    # Check dependencies
    check_newman
    
    # Create reports directory if needed
    if [[ $REPORTERS == *"html"* ]] || [[ $REPORTERS == *"json"* ]] || [[ $REPORTERS == *"junit"* ]]; then
        mkdir -p "$REPORT_DIR"
        print_info "Reports will be saved to: $REPORT_DIR"
    fi
    
    # Build newman command
    NEWMAN_CMD="newman run $COLLECTION"
    NEWMAN_CMD="$NEWMAN_CMD --env-var base_url=$BASE_URL"
    NEWMAN_CMD="$NEWMAN_CMD -r $REPORTERS"
    
    # Add folder filter if specified
    if [ -n "$FOLDER" ]; then
        NEWMAN_CMD="$NEWMAN_CMD --folder \"$FOLDER\""
        print_info "Running only folder: $FOLDER"
    fi
    
    # Add report export paths
    if [[ $REPORTERS == *"html"* ]]; then
        # Check if HTML reporter is installed
        if ! npm list -g newman-reporter-html &> /dev/null; then
            print_error "HTML reporter not installed!"
            print_info "Install with: npm install -g newman-reporter-html"
            exit 1
        fi
        NEWMAN_CMD="$NEWMAN_CMD --reporter-html-export $REPORT_DIR/report.html"
    fi
    
    if [[ $REPORTERS == *"json"* ]]; then
        NEWMAN_CMD="$NEWMAN_CMD --reporter-json-export $REPORT_DIR/report.json"
    fi
    
    if [[ $REPORTERS == *"junit"* ]]; then
        NEWMAN_CMD="$NEWMAN_CMD --reporter-junit-export $REPORT_DIR/report.xml"
    fi
    
    # Show configuration
    echo ""
    print_info "Configuration:"
    echo "  Base URL: $BASE_URL"
    echo "  Reporters: $REPORTERS"
    echo "  Collection: $COLLECTION"
    echo ""
    
    # Check if server is running
    print_info "Checking if server is running at $BASE_URL..."
    if curl --cacert /Users/jordan/code/econic/ssl/local.econic.ai.crt -s -f "$BASE_URL/health" > /dev/null 2>&1; then
        print_success "Server is running"
    else
        print_error "Server is not responding at $BASE_URL"
        print_info "Start the server with: cargo run -- --http-addr 127.0.0.1:8080"
        exit 1
    fi
    
    echo ""
    print_info "Running tests..."
    echo "─────────────────────────────────────────────"
    
    # Run newman with SSL verification disabled for self-signed certificates
    NODE_TLS_REJECT_UNAUTHORIZED=0 eval $NEWMAN_CMD
    
    # Check exit code
    if [ $? -eq 0 ]; then
        echo "─────────────────────────────────────────────"
        print_success "All tests passed! API functionality is complete!"
        
        # Show report locations
        if [[ $REPORTERS == *"html"* ]]; then
            print_info "HTML report: $REPORT_DIR/report.html"
        fi
        if [[ $REPORTERS == *"json"* ]]; then
            print_info "JSON report: $REPORT_DIR/report.json"
        fi
        if [[ $REPORTERS == *"junit"* ]]; then
            print_info "JUnit report: $REPORT_DIR/report.xml"
        fi
    else
        echo "─────────────────────────────────────────────"
        print_error "Some tests failed - indicates incomplete functionality"
        echo -e "${YELLOW}ℹ  Check test results to see which features need implementation${NC}"
        exit 1
    fi
}

# Run main function
main 