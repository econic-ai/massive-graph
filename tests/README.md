# Massive Graph API Tests

This directory contains comprehensive API tests for Massive Graph, with both an interactive web dashboard and a command-line script for CI/CD pipelines.

## 🚀 Quick Start

### Web Dashboard (Interactive Testing)

```bash
# Install dependencies
npm install

# Start the test dashboard
npm run dev

# Open http://localhost:5173 in your browser
```

**Features:**
- Visual test execution with real-time results
- Filter by test name or group
- Run specific test subsets
- Keyboard shortcut 'O' to run tests
- Status indicators: ● Passed (green), ● Failed (red), ○ Not Implemented (gray)

### Command-Line Testing (CI/CD)

```bash
# Run all tests
./poc-tests.sh

# The script will:
# 1. Check if the server is running
# 2. Execute all tests via Newman
# 3. Display results with pass/fail summary
```

## 📋 Test Structure

Tests are organized into groups (flows):

1. **Health Checks** - Basic server health and info endpoints
2. **Flow 1: Create-Get-Delete-Get** - Document lifecycle tests
3. **Flow 2: Create-ApplyDelta-GetLatest** - Delta operations tests
4. **Additional Tests** - Error handling and edge cases

## 🛠️ Configuration

- **Base URL**: Default is `http://localhost:8080`
- **Collection**: Tests are defined in `poc.postman_collection.json`
- **Variables**: `document_id` is automatically managed between tests

## 📊 Dashboard Usage

1. **Filter Tests**: Type in the filter box to search by test name or group
2. **Run Tests**: Click "Run Tests" or press 'O'
3. **Toggle Status**: Use checkboxes to show/hide passed, failed, or not implemented tests
4. **Group Filter**: Double-click a group header to filter to that group
5. **Expand Results**: Click test items to see detailed assertions and errors

## 🔧 CI/CD Integration

The `poc-tests.sh` script is designed for CI/CD pipelines:

```yaml
# GitHub Actions
- name: Run API Tests
  run: cd apps/massive-graph/tests && ./poc-tests.sh

# Jenkins
stage('API Tests') {
    steps {
        sh 'cd apps/massive-graph/tests && ./poc-tests.sh'
    }
}
```

Exit codes:
- 0: All tests passed
- 1: Some tests failed
- 2: Server not reachable

## 📦 Dependencies

- **Newman**: Command-line Postman collection runner
- **Node.js**: Required for both dashboard and Newman
- **Svelte/Vite**: Powers the web dashboard

## 🔍 Troubleshooting

- **SSL Issues**: The script sets `NODE_TLS_REJECT_UNAUTHORIZED=0` for local development
- **Port Conflicts**: Ensure port 8080 (API) and 5173 (dashboard) are available
- **Missing Dependencies**: Run `npm install` to install all required packages