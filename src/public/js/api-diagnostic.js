// API Diagnostic Tool - helps diagnose API connectivity issues
(function() {
    console.log('[API Diagnostic] Starting API diagnostics...');
    
    // Test configuration
    console.log('[API Diagnostic] Current configuration:', {
        origin: window.location.origin,
        hostname: window.location.hostname,
        protocol: window.location.protocol,
        pathname: window.location.pathname,
        config: window.SLIMLYTICS_CONFIG
    });
    
    // Test API endpoints
    async function testEndpoint(path, description) {
        const url = window.SLIMLYTICS_CONFIG ? 
            window.SLIMLYTICS_CONFIG.apiEndpoint(path) : 
            window.location.origin + path;
            
        console.log(`[API Diagnostic] Testing ${description}: ${url}`);
        
        try {
            const startTime = performance.now();
            const response = await fetch(url, {
                method: 'GET',
                headers: {
                    'Accept': 'application/json'
                },
                credentials: 'same-origin'
            });
            const endTime = performance.now();
            
            const responseInfo = {
                url: url,
                status: response.status,
                statusText: response.statusText,
                ok: response.ok,
                contentType: response.headers.get('content-type'),
                responseTime: Math.round(endTime - startTime) + 'ms',
                headers: {}
            };
            
            // Capture relevant headers
            ['content-type', 'server', 'x-powered-by', 'access-control-allow-origin'].forEach(header => {
                const value = response.headers.get(header);
                if (value) {
                    responseInfo.headers[header] = value;
                }
            });
            
            // Try to read response body
            let body = null;
            try {
                const text = await response.text();
                if (responseInfo.contentType && responseInfo.contentType.includes('application/json')) {
                    try {
                        body = JSON.parse(text);
                    } catch (e) {
                        body = text.substring(0, 500); // First 500 chars if not valid JSON
                    }
                } else {
                    body = text.substring(0, 500); // First 500 chars
                }
            } catch (e) {
                console.error(`[API Diagnostic] Could not read response body for ${path}:`, e);
            }
            
            console.log(`[API Diagnostic] ✅ ${description} response:`, responseInfo);
            if (body) {
                console.log(`[API Diagnostic] Response body preview:`, body);
            }
            
            return { success: response.ok, ...responseInfo, body };
            
        } catch (error) {
            console.error(`[API Diagnostic] ❌ ${description} failed:`, {
                error: error.message,
                stack: error.stack,
                url: url
            });
            return { success: false, error: error.message, url };
        }
    }
    
    // Run all tests
    async function runDiagnostics() {
        console.log('[API Diagnostic] Running comprehensive API tests...');
        
        const tests = [
            { path: '/api/sites', description: 'Sites API' },
            { path: '/api/stats/test', description: 'Stats API (test)' },
            { path: '/t.js', description: 'Tracking script' },
            { path: '/', description: 'Root endpoint' }
        ];
        
        const results = [];
        for (const test of tests) {
            const result = await testEndpoint(test.path, test.description);
            results.push({ ...test, ...result });
        }
        
        // Summary
        console.log('[API Diagnostic] === DIAGNOSTIC SUMMARY ===');
        console.table(results.map(r => ({
            Endpoint: r.path,
            Status: r.success ? '✅ OK' : '❌ FAILED',
            HTTPStatus: r.status || 'N/A',
            ResponseTime: r.responseTime || 'N/A',
            Error: r.error || ''
        })));
        
        // Check for common issues
        console.log('[API Diagnostic] === COMMON ISSUES CHECK ===');
        
        // Check if API is on different origin
        const apiOrigin = window.SLIMLYTICS_CONFIG?.API_URL;
        if (apiOrigin && apiOrigin !== window.location.origin) {
            console.warn('[API Diagnostic] ⚠️ API is on different origin. CORS issues possible.');
            console.warn(`[API Diagnostic] Dashboard: ${window.location.origin}, API: ${apiOrigin}`);
        }
        
        // Check for HTTPS issues
        if (window.location.protocol === 'https:' && apiOrigin && apiOrigin.startsWith('http:')) {
            console.error('[API Diagnostic] ❌ Mixed content: HTTPS page trying to access HTTP API');
        }
        
        // Check for 500 errors
        const serverErrors = results.filter(r => r.status >= 500);
        if (serverErrors.length > 0) {
            console.error('[API Diagnostic] ❌ Server errors detected:', serverErrors);
            console.error('[API Diagnostic] The API server appears to be having issues. Check server logs.');
        }
        
        // Check for 404 errors
        const notFoundErrors = results.filter(r => r.status === 404);
        if (notFoundErrors.length > 0) {
            console.warn('[API Diagnostic] ⚠️ Some endpoints not found:', notFoundErrors);
            console.warn('[API Diagnostic] The API server may not be properly configured.');
        }
        
        // Check for network errors
        const networkErrors = results.filter(r => !r.status && r.error);
        if (networkErrors.length > 0) {
            console.error('[API Diagnostic] ❌ Network errors detected:', networkErrors);
            console.error('[API Diagnostic] Cannot reach the API server. Check network/firewall/proxy settings.');
        }
        
        return results;
    }
    
    // Auto-run diagnostics after page load
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', runDiagnostics);
    } else {
        // DOM already loaded
        setTimeout(runDiagnostics, 100);
    }
    
    // Expose for manual testing
    window.SlimlyticsDiagnostic = {
        testEndpoint,
        runDiagnostics
    };
})();