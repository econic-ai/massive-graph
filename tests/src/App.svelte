<script>
  import { onMount } from 'svelte'
  import TestItem from './TestItem.svelte'
  
  let tests = []
  let filter = ''
  let showPassed = true
  let showFailed = true
  let showNotImplemented = true
  let isRunning = false
  let serverStatus = 'unknown'
  let lastRun = null
  
  // Test statistics
  let totalTests = 0
  let passedTests = 0
  let failedTests = 0
  let notImplementedTests = 0
  
  // Base URL for the API being tested
  const BASE_URL = 'http://localhost:8080'
  
  // Check server status
  async function checkServerStatus() {
    try {
      const response = await fetch(`${BASE_URL}/health`)
      serverStatus = response.ok ? 'online' : 'offline'
    } catch (error) {
      serverStatus = 'offline'
    }
  }
  
  // Load test collection from API
  async function loadTests() {
    try {
      const response = await fetch('/api/tests')
      const data = await response.json()
      
      if (data.success) {
        tests = data.tests
        updateStatistics()
      } else {
        throw new Error(data.error || 'Failed to load tests')
      }
    } catch (error) {
      console.error('Failed to load test collection:', error)
      // Use placeholder tests if collection not found
      tests = [
        { id: 'placeholder', name: 'Test collection not found', group: 'Error', status: 'not_implemented', assertions: [] }
      ]
      updateStatistics()
    }
  }
  
  // Parse Postman collection
  function parseCollection(collection) {
    const parsedTests = []
    let testId = 0
    
    function extractTests(items, groupName = '') {
      for (const item of items) {
        if (item.item) {
          // This is a folder/group
          extractTests(item.item, item.name)
        } else if (item.request) {
          // This is a test
          const test = {
            id: `test-${testId++}`,
            name: item.name,
            group: groupName,
            request: {
              method: item.request.method,
              path: item.request.url?.raw || item.request.url || ''
            },
            status: 'not_implemented',
            expanded: false,
            assertions: []
          }
          parsedTests.push(test)
        }
      }
    }
    
    if (collection.item) {
      extractTests(collection.item)
    }
    
    return parsedTests
  }
  
  // Run tests
  async function runTests() {
    if (isRunning) return
    
    isRunning = true
    lastRun = new Date().toLocaleTimeString()
    
    // Reset all tests to running state
    tests = tests.map(test => ({
      ...test,
      status: 'running',
      error: null,
      assertions: []
    }))
    
    try {
      const response = await fetch('/api/run-tests', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          filteredTests: flatFilteredTests.map(test => test.name)
        })
      })
      
      const result = await response.json()
      
      if (result.success && result.results) {
        // Update test results
        const resultMap = new Map(result.results.map(r => [r.name, r]))
        
        tests = tests.map(test => {
          const result = resultMap.get(test.name)
          if (result) {
            return {
              ...test,
              status: result.status || 'not_implemented',
              assertions: result.assertions || [],
              response: result.response,
              error: result.assertions?.filter(a => !a.passed).map(a => a.message).join('; ')
            }
          }
          return { ...test, status: 'not_implemented' }
        })
      }
      
      updateStatistics()
    } catch (error) {
      console.error('Failed to run tests:', error)
      // Reset tests to previous state
      tests = tests.map(test => ({
        ...test,
        status: test.status === 'running' ? 'not_implemented' : test.status
      }))
    } finally {
      isRunning = false
    }
  }
  
  // Update test statistics
  function updateStatistics() {
    totalTests = tests.length
    passedTests = tests.filter(t => t.status === 'passed').length
    failedTests = tests.filter(t => t.status === 'failed').length
    notImplementedTests = tests.filter(t => t.status === 'not_implemented').length
  }
  
  // Group tests by their group field
  function getGroupedTests() {
    const groups = {}
    
    tests.forEach(test => {
      const groupName = test.group || 'Ungrouped'
      if (!groups[groupName]) {
        groups[groupName] = []
      }
      groups[groupName].push(test)
    })
    
    return groups
  }
  
  // Filter tests with enhanced group logic
  function getFilteredTests(tests, filter, showPassed, showFailed, showNotImplemented) {
    const groupedTests = getGroupedTests()
    const filteredGroups = {}
    
    Object.keys(groupedTests).forEach(groupName => {
      const groupTests = groupedTests[groupName]
      
      // Check if filter matches group name
      const groupMatches = !filter || groupName.toLowerCase().includes(filter.toLowerCase())
      
      // Filter individual tests
      const filteredTestsInGroup = groupTests.filter(test => {
        // If group matches, include all tests in group (regardless of test name)
        if (groupMatches) {
          // Still apply status filtering
          if (test.status === 'passed') return showPassed
          if (test.status === 'failed') return showFailed
          if (test.status === 'not_implemented') return showNotImplemented
          if (test.status === 'running') return true
          return true
        }
        
        // Otherwise, filter by individual test name AND status
        const testMatches = !filter || test.name.toLowerCase().includes(filter.toLowerCase())
        if (!testMatches) return false
        
        // Apply status filtering
        if (test.status === 'passed') return showPassed
        if (test.status === 'failed') return showFailed
        if (test.status === 'not_implemented') return showNotImplemented
        if (test.status === 'running') return true
        return true
      })
      
      // Always include the group (even if empty) when there's no filter or group matches
      if (!filter || groupMatches || filteredTestsInGroup.length > 0) {
        filteredGroups[groupName] = filteredTestsInGroup
      }
    })
    
    return filteredGroups
  }
  
  // Get flat list of filtered tests (for running tests)
  function getFlatFilteredTests(filteredGroups) {
    const flatTests = []
    Object.values(filteredGroups).forEach(groupTests => {
      flatTests.push(...groupTests)
    })
    return flatTests
  }
  
  // Handle group double-click
  function onGroupDoubleClick(groupName) {
    filter = groupName
  }
  
  // Clear filter
  function clearFilter() {
    filter = ''
  }
  
  // Collapse/expand all
  function collapseAll() {
    tests = tests.map(test => ({ ...test, expanded: false }))
  }
  
  function expandAll() {
    tests = tests.map(test => ({ ...test, expanded: true }))
  }
  
  // Toggle test expansion
  function toggleTest(testId) {
    tests = tests.map(test => 
      test.id === testId ? { ...test, expanded: !test.expanded } : test
    )
  }
  
  onMount(() => {
    checkServerStatus()
    loadTests()
    
    // Check server status periodically
    const interval = setInterval(checkServerStatus, 60000)
    
    // Add keyboard shortcut for running tests
    const handleKeydown = (event) => {
      if (event.key.toLowerCase() === 'o' && !event.ctrlKey && !event.metaKey && !event.altKey) {
        // Only trigger if not in an input field
        if (event.target.tagName !== 'INPUT' && event.target.tagName !== 'TEXTAREA') {
          runTests()
        }
      }
    }
    
    document.addEventListener('keydown', handleKeydown)
    
    return () => {
      clearInterval(interval)
      document.removeEventListener('keydown', handleKeydown)
    }
  })
  
  $: filteredGroups = getFilteredTests(tests, filter, showPassed, showFailed, showNotImplemented)
  $: flatFilteredTests = getFlatFilteredTests(filteredGroups)
</script>

<div class="dashboard">
  <header>
    <h1>Massive Graph API Test Dashboard</h1>
    
    <div class="status-bar">
      <div class="status-item">
        <span class="label">Server:</span>
        <span class="status-indicator {serverStatus}">
          {serverStatus === 'online' ? '●' : '○'} {serverStatus}
        </span>
      </div>
      
      <div class="status-item">
        <span class="label">API URL:</span>
        <span>{BASE_URL}</span>
      </div>
      
      <div class="status-item">
        <span class="label">Last Run:</span>
        <span>{lastRun || 'Never'}</span>
      </div>
      
      <div class="status-item">
        <span class="label">Tests:</span>
        <span class="stats">
          <span class="passed">{passedTests}</span> / 
          <span class="total">{totalTests}</span>
          {#if failedTests > 0}
            <span class="failed">({failedTests} failed)</span>
          {/if}
        </span>
      </div>
    </div>
  </header>
  
  <div class="controls">
        <div class="control-row">
      <div class="filter-container">
        <input 
          type="text"
          placeholder="Filter tests..."
          bind:value={filter}
          class="filter-input"
        />
        {#if filter}
          <button 
            on:click={clearFilter}
            class="clear-button"
            title="Clear filter"
          >
            ✕
          </button>
        {/if}
      </div>
      
      <button 
        on:click={runTests} 
        disabled={isRunning || serverStatus !== 'online'}
        class="run-button"
        title="Click or press 'O' to run tests"
      >
        {isRunning ? 'Running...' : 'Run Tests (O)'}
      </button>
    </div>
    
    <div class="control-row">
      <div class="toggles">
        <label>
          <input type="checkbox" bind:checked={showPassed} />
          <span class="passed">✓ Passed ({passedTests})</span>
        </label>
        
        <label>
          <input type="checkbox" bind:checked={showFailed} />
          <span class="failed">✗ Failed ({failedTests})</span>
        </label>
        
        <label>
          <input type="checkbox" bind:checked={showNotImplemented} />
          <span class="not-implemented">○ Not Implemented ({notImplementedTests})</span>
        </label>
      </div>
      
      <div class="actions">
        <button on:click={expandAll} class="action-button">Expand All</button>
        <button on:click={collapseAll} class="action-button">Collapse All</button>
      </div>
    </div>
  </div>
  
  <div class="test-list">
    {#each Object.entries(filteredGroups) as [groupName, groupTests] (groupName)}
      <div class="test-group">
        <div 
          class="group-header"
          on:dblclick={() => onGroupDoubleClick(groupName)}
          title="Double-click to filter by this group"
        >
          <h3 class="group-title">{groupName}</h3>
          <span class="group-stats">
            {groupTests.length} test{groupTests.length !== 1 ? 's' : ''}
          </span>
        </div>
        
        <div class="group-tests">
          {#each groupTests as test (test.id)}
            <TestItem 
              {test} 
              on:toggle={() => toggleTest(test.id)}
            />
          {/each}
          
          {#if groupTests.length === 0}
            <div class="no-tests-in-group">
              <p>No tests in this group match the current filter.</p>
            </div>
          {/if}
        </div>
      </div>
    {/each}
    
    {#if Object.keys(filteredGroups).length === 0}
      <div class="no-tests">
        {#if tests.length === 0}
          <p>No tests loaded yet.</p>
        {:else}
          <p>No tests match the current filter.</p>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .dashboard {
    max-width: 1200px;
    margin: 0 auto;
    padding: 20px;
  }
  
  header {
    background: white;
    padding: 20px;
    border-radius: 8px;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    margin-bottom: 20px;
  }
  
  h1 {
    margin: 0 0 20px 0;
    color: #333;
    font-size: 24px;
  }
  
  .status-bar {
    display: flex;
    gap: 30px;
    font-size: 14px;
    flex-wrap: wrap;
  }
  
  .status-item {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  
  .label {
    color: #666;
    font-weight: 500;
  }
  
  .status-indicator {
    font-weight: 600;
  }
  
  .status-indicator.online {
    color: #22c55e;
  }
  
  .status-indicator.offline {
    color: #ef4444;
  }
  
  .status-indicator.unknown {
    color: #94a3b8;
  }
  
  .stats {
    font-weight: 600;
  }
  
  .passed {
    color: #22c55e;
  }
  
  .failed {
    color: #ef4444;
  }
  
  .not-implemented {
    color: #94a3b8;
  }
  
  .total {
    color: #333;
  }
  
  .controls {
    background: white;
    padding: 20px;
    border-radius: 8px;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    margin-bottom: 20px;
  }
  
  .control-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 20px;
    margin-bottom: 15px;
  }
  
  .control-row:last-child {
    margin-bottom: 0;
  }
  
  .filter-container {
    position: relative;
    flex: 1;
    display: flex;
    align-items: center;
  }
  
  .filter-input {
    flex: 1;
    padding: 10px 15px;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    font-size: 14px;
    outline: none;
    transition: border-color 0.2s;
    padding-right: 40px;
  }
  
  .filter-input:focus {
    border-color: #3b82f6;
  }
  
  .clear-button {
    position: absolute;
    right: 8px;
    background: none;
    border: none;
    color: #6b7280;
    cursor: pointer;
    padding: 4px;
    border-radius: 3px;
    font-size: 14px;
    line-height: 1;
  }
  
  .clear-button:hover {
    background-color: #f3f4f6;
    color: #374151;
  }
  
  .run-button {
    padding: 10px 30px;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
  }
  
  .run-button:hover:not(:disabled) {
    background: #2563eb;
  }
  
  .run-button:disabled {
    background: #94a3b8;
    cursor: not-allowed;
  }
  
  .toggles {
    display: flex;
    gap: 20px;
  }
  
  .toggles label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    font-size: 14px;
  }
  
  .toggles input[type="checkbox"] {
    cursor: pointer;
  }
  
  .actions {
    display: flex;
    gap: 10px;
  }
  
  .action-button {
    padding: 6px 16px;
    background: #f3f4f6;
    color: #374151;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    font-size: 14px;
    cursor: pointer;
    transition: all 0.2s;
  }
  
  .action-button:hover {
    background: #e5e7eb;
  }
  
  .test-list {
    background: white;
    border-radius: 8px;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    overflow: hidden;
  }
  
  .test-group {
    border-bottom: 1px solid #e5e7eb;
  }
  
  .test-group:last-child {
    border-bottom: none;
  }
  
  .group-header {
    background: #f8fafc;
    padding: 12px 20px;
    border-bottom: 1px solid #e5e7eb;
    cursor: pointer;
    user-select: none;
    transition: background-color 0.2s;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  
  .group-header:hover {
    background: #f1f5f9;
  }
  
  .group-title {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: #1f2937;
  }
  
  .group-stats {
    font-size: 12px;
    color: #6b7280;
    background: #e5e7eb;
    padding: 2px 8px;
    border-radius: 12px;
  }
  
  .group-tests {
    /* Tests will be rendered here */
  }
  
  .no-tests-in-group {
    padding: 20px;
    text-align: center;
    color: #6b7280;
    font-style: italic;
  }
  
  .no-tests {
    padding: 60px 20px;
    text-align: center;
    color: #6b7280;
  }
  
  @media (max-width: 768px) {
    .control-row {
      flex-direction: column;
      align-items: stretch;
    }
    
    .toggles {
      flex-direction: column;
      gap: 10px;
    }
    
    .status-bar {
      flex-direction: column;
      gap: 10px;
    }
  }
</style>