import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import path from 'path'

export default defineConfig({
  server: {
    watch: {
      // Watch the Postman collection file
      ignored: ['!**/poc.postman_collection.json']
    }
  },
  plugins: [
    svelte(),
    {
      name: 'test-api',
      configureServer(server) {
        const postmanCollectionPath = path.join(process.cwd(), 'poc.postman_collection.json')
        
        // Watch the Postman collection file and trigger full reload
        server.watcher.add(postmanCollectionPath)
        server.watcher.on('change', (file) => {
          if (file === postmanCollectionPath) {
            // Send full reload command
            server.ws.send({
              type: 'full-reload',
              path: '*'
            })
          }
        })
        // Serve the Postman collection file from the root directory
        server.middlewares.use((req, res, next) => {
          if (req.url === '/poc.postman_collection.json') {
            const fs = require('fs')
            const path = require('path')
            const filePath = path.join(process.cwd(), 'poc.postman_collection.json')
            
            res.setHeader('Content-Type', 'application/json')
            fs.createReadStream(filePath).pipe(res)
            return
          }
          next()
        })
        // Get test collection without running tests
        server.middlewares.use('/api/tests', async (req, res) => {
          if (req.method !== 'GET') {
            res.statusCode = 405
            res.end()
            return
          }

          // Enable CORS
          res.setHeader('Access-Control-Allow-Origin', '*')
          res.setHeader('Content-Type', 'application/json')

          try {
            const fs = await import('fs')
            const path = await import('path')
            const collectionPath = path.join(process.cwd(), 'poc.postman_collection.json')
            const collectionData = JSON.parse(fs.readFileSync(collectionPath, 'utf8'))
            
            const tests = extractTestsFromCollection(collectionData)
            
            res.statusCode = 200
            res.end(JSON.stringify({ success: true, tests }))
          } catch (error) {
            res.statusCode = 500
            res.end(JSON.stringify({ success: false, error: error.message }))
          }
        })

        server.middlewares.use('/api/run-tests', async (req, res) => {
          if (req.method !== 'POST') {
            res.statusCode = 405
            res.end()
            return
          }

          // Enable CORS
          res.setHeader('Access-Control-Allow-Origin', '*')
          res.setHeader('Content-Type', 'application/json')

          try {
            // Parse request body to get filtered test names
            let body = ''
            req.on('data', chunk => { body += chunk.toString() })
            await new Promise(resolve => req.on('end', resolve))
            
            const requestData = JSON.parse(body || '{}')
            const filteredTestNames = requestData.filteredTests || []
            
            const { exec } = await import('child_process')
            const { promisify } = await import('util')
            const fs = await import('fs')
            const path = await import('path')
            const execAsync = promisify(exec)
            
            // Load and filter the collection if filteredTests is provided
            let collectionPath = path.join(process.cwd(), 'poc.postman_collection.json')
            
            if (filteredTestNames.length > 0) {
              // Load original collection
              const originalCollection = JSON.parse(fs.readFileSync(collectionPath, 'utf8'))
              
              // Filter the collection to only include selected tests
              const filteredCollection = filterCollection(originalCollection, filteredTestNames)
              
              // Create temporary filtered collection file
              const tempCollectionPath = path.join(process.cwd(), 'temp_filtered_collection.json')
              fs.writeFileSync(tempCollectionPath, JSON.stringify(filteredCollection, null, 2))
              collectionPath = tempCollectionPath
            }
            
            // Run newman command
            const baseUrl = process.env.BASE_URL || 'http://localhost:8080'
            let stdout = ''
            try {
              const result = await execAsync(
                `NODE_TLS_REJECT_UNAUTHORIZED=0 newman run "${collectionPath}" --env-var base_url="${baseUrl}" -r cli --color off`,
                { cwd: process.cwd() }
              )
              stdout = result.stdout
            } catch (error) {
              // Newman returns exit code 1 when tests fail, but we still want the output
              stdout = error.stdout || ''
            }
            
            // Clean up temporary file if created
            if (filteredTestNames.length > 0) {
              try {
                fs.unlinkSync(collectionPath)
              } catch (e) {
                // Ignore cleanup errors
              }
            }
            
            // Parse the output to extract test results
            const results = parseNewmanOutput(stdout)
            
            res.statusCode = 200
            res.end(JSON.stringify({ success: true, results }))
          } catch (error) {
            res.statusCode = 500
            res.end(JSON.stringify({ success: false, error: error.message }))
          }
        })
      }
    }
  ],
  server: {
    port: 5173
  }
})

// Parse Newman CLI output
function parseNewmanOutput(output) {
  const lines = output.split('\n')
  const results = []
  let currentGroup = ''
  let currentTest = null
  let testId = 0
  

  
  for (const line of lines) {
    // Stop parsing when we hit the summary table or failure details section
    if (line.includes('┌─────────────────────────┬─────────────────┬─────────────────┐') || 
        line.includes('│                         │        executed │          failed │') ||
        line.includes('#  failure') || 
        line.includes('failure detail')) {
      break
    }
    
    // Skip empty lines, separators, and summary table content
    if (!line.trim() || 
        line.includes('─────') || 
        line.includes('newman') ||
        (line.includes('│') && (line.includes('executed') || line.includes('failed') || line.includes('iterations')))) {
      continue
    }
    
    // Test group
    if (line.includes('❏')) {
      currentGroup = line.replace(/❏\s+/, '').trim()
      continue
    }
    
    // Test item
    if (line.includes('↳')) {
      // Save previous test if exists
      if (currentTest && currentTest.id) {
        results.push(currentTest)
      }
      
      const testName = line.replace(/↳\s+/, '').trim()
      currentTest = {
        id: `test-${testId++}`,
        name: testName,
        group: currentGroup,
        status: 'not_implemented',
        assertions: [],
        request: null,
        response: null
      }
      continue
    }
    
    // HTTP request line
    if (currentTest && (line.includes('GET') || line.includes('POST') || line.includes('PUT') || line.includes('DELETE'))) {
      const match = line.match(/\s*(GET|POST|PUT|DELETE)\s+([^\s]+)\s+\[([^\]]+)\]/)
      if (match) {
        currentTest.request = {
          method: match[1],
          path: match[2]
        }
        currentTest.response = {
          status: parseInt(match[3]) || (match[3].includes('OK') ? 200 : 0),
          statusText: match[3]
        }
      }
      continue
    }
    
    // Passed assertion
    if (currentTest && line.includes('✓')) {
      const assertion = line.replace(/\s*✓\s*/, '').trim()
      if (assertion) {
        currentTest.assertions.push({ passed: true, message: assertion })
        if (currentTest.status === 'not_implemented') {
          currentTest.status = 'passed'
        }
      }
      continue
    }
    
    // Failed assertion (numbered)
    if (currentTest && /^\s*\d+\./.test(line)) {
      const assertion = line.replace(/^\s*\d+\.\s*/, '').trim()
      if (assertion) {
        currentTest.assertions.push({ passed: false, message: assertion })
        currentTest.status = 'failed'
      }
      continue
    }
  }
  
  // Add last test if exists
  if (currentTest && currentTest.id) {
    results.push(currentTest)
  }
  
  return results
}

// Extract tests from Postman collection to show by default
function extractTestsFromCollection(collection) {
  const tests = []
  let testId = 0
  
  function processItem(item, groupName = '') {
    if (item.item) {
      // This is a group
      const currentGroup = item.name || groupName
      item.item.forEach(subItem => processItem(subItem, currentGroup))
    } else {
      // This is a test
      const test = {
        id: `test-${testId++}`,
        name: item.name || 'Unnamed Test',
        group: groupName,
        status: 'not_implemented',
        assertions: [],
        request: item.request ? {
          method: item.request.method,
          path: item.request.url?.raw || item.request.url
        } : null,
        response: null
      }
      
      // Extract expected assertions from test scripts
      if (item.event) {
        const testEvent = item.event.find(e => e.listen === 'test')
        if (testEvent && testEvent.script && testEvent.script.exec) {
          const testLines = testEvent.script.exec
          for (const line of testLines) {
            const match = line.match(/pm\.test\("([^"]+)"/)
            if (match) {
              test.assertions.push({
                passed: null,
                message: match[1]
              })
            }
          }
        }
      }
      
      tests.push(test)
    }
  }
  
  if (collection.item) {
    collection.item.forEach(item => processItem(item))
  }
  
  return tests
}

// Filter Postman collection to only include specified tests
function filterCollection(collection, testNamesToInclude) {
  function filterItems(items) {
    const filtered = []
    
    for (const item of items) {
      if (item.item) {
        // This is a group - recursively filter and keep if it has any matching tests
        const filteredSubItems = filterItems(item.item)
        if (filteredSubItems.length > 0) {
          filtered.push({ ...item, item: filteredSubItems })
        }
      } else {
        // This is a test - keep if it's in the list
        if (testNamesToInclude.includes(item.name)) {
          filtered.push(item)
        }
      }
    }
    
    return filtered
  }
  
  const filteredCollection = {
    ...collection,
    item: filterItems(collection.item || [])
  }
  
  return filteredCollection
}