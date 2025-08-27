<script>
  import { createEventDispatcher } from 'svelte'
  
  export let test = {}
  
  const dispatch = createEventDispatcher()
  
  function getStatusIcon(status) {
    switch (status) {
      case 'passed': return '●'
      case 'failed': return '●'
      case 'not_implemented': return '○'
      case 'running': return '⊙'
      default: return '?'
    }
  }
  
  function getStatusClass(status) {
    switch (status) {
      case 'passed': return 'status-passed'
      case 'failed': return 'status-failed'
      case 'not_implemented': return 'status-not-implemented'
      default: return 'status-unknown'
    }
  }
  
  function toggle() {
    dispatch('toggle')
  }
</script>

<div class="test-item {getStatusClass(test.status)}">
  <div class="test-header" on:click={toggle}>
    <div class="test-info">
      <span class="status-icon">{getStatusIcon(test.status)}</span>
      <span class="test-name">{test.name}</span>
      {#if test.request}
        <span class="test-request">{test.request.method} {test.request.path}</span>
      {/if}
    </div>
    
    {#if test.error || test.assertions?.length > 0}
      <button class="toggle-button" class:expanded={test.expanded}>
        {test.expanded ? '−' : '+'}
      </button>
    {/if}
  </div>
  
  {#if test.expanded && (test.error || test.assertions?.length > 0)}
    <div class="test-details">
      {#if test.assertions && test.assertions.length > 0}
        <div class="assertions">
          {#each test.assertions as assertion}
            <div class="assertion {assertion.passed ? 'passed' : 'failed'}">
              <span class="assertion-icon">{assertion.passed ? '✓' : '✗'}</span>
              <span class="assertion-message">{assertion.message}</span>
            </div>
          {/each}
        </div>
      {/if}
      
      {#if test.error}
        <div class="error-message">
          <strong>Failed assertions:</strong> {test.error}
        </div>
      {/if}
      
      {#if test.response}
        <div class="response-info">
          <span class="response-status">
            {test.response.status} {test.response.statusText}
          </span>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .test-item {
    border-bottom: 1px solid #e5e7eb;
    transition: background-color 0.2s;
  }
  
  .test-item:hover {
    background-color: #f9fafb;
  }
  
  .test-header {
    padding: 8px 16px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    cursor: pointer;
  }
  
  .test-info {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
  }
  
  .status-icon {
    font-weight: bold;
    font-size: 18px;
    width: 24px;
    text-align: center;
  }
  
  .status-passed .status-icon {
    color: #22c55e;
  }
  
  .status-failed .status-icon {
    color: #ef4444;
  }
  
  .status-not-implemented .status-icon {
    color: #94a3b8;
  }
  
  .test-name {
    font-weight: 500;
    color: #1f2937;
    flex: 1;
    font-size: 14px;
  }
  
  .test-request {
    font-size: 13px;
    color: #6b7280;
    font-family: monospace;
    background: #f3f4f6;
    padding: 4px 8px;
    border-radius: 4px;
  }
  
  .toggle-button {
    width: 24px;
    height: 24px;
    border: 1px solid #e5e7eb;
    background: white;
    border-radius: 4px;
    cursor: pointer;
    font-size: 16px;
    line-height: 1;
    transition: all 0.2s;
  }
  
  .toggle-button:hover {
    background: #f3f4f6;
  }
  
  .toggle-button.expanded {
    background: #f3f4f6;
  }
  
  .test-details {
    padding: 0 20px 20px 56px;
    animation: slideDown 0.2s ease-out;
  }
  
  .error-message {
    background: #fef2f2;
    border: 1px solid #fecaca;
    color: #991b1b;
    padding: 12px;
    border-radius: 6px;
    margin-bottom: 12px;
    font-size: 14px;
    line-height: 1.5;
  }
  
  .test-message {
    background: #f9fafb;
    border: 1px solid #e5e7eb;
    padding: 12px;
    border-radius: 6px;
    margin-bottom: 12px;
    font-size: 14px;
    line-height: 1.5;
    color: #374151;
  }
  
  .response-info {
    display: flex;
    gap: 20px;
    font-size: 13px;
    color: #6b7280;
  }
  
  .response-status {
    font-weight: 500;
  }
  
  .assertions {
    margin-bottom: 12px;
  }
  
  .assertion {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 12px;
    margin-bottom: 4px;
    border-radius: 4px;
    font-size: 14px;
  }
  
  .assertion.passed {
    background: #f0fdf4;
    color: #166534;
  }
  
  .assertion.failed {
    background: #fef2f2;
    color: #991b1b;
  }
  
  .assertion-icon {
    font-weight: bold;
    flex-shrink: 0;
  }
  
  .assertion-message {
    line-height: 1.4;
  }
  
  @keyframes slideDown {
    from {
      opacity: 0;
      transform: translateY(-10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
