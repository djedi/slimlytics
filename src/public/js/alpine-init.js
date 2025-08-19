// Alpine.js initialization wrapper with error handling and debugging
(function() {
    console.log('[Alpine Init] Starting Alpine.js initialization wrapper');
    
    // Check if Alpine is loaded
    function checkAlpine() {
        if (typeof Alpine !== 'undefined') {
            console.log('[Alpine Init] Alpine.js is loaded successfully');
            return true;
        }
        console.warn('[Alpine Init] Alpine.js not found in global scope');
        return false;
    }
    
    // Wait for Alpine to be available
    let attempts = 0;
    const maxAttempts = 50; // 5 seconds max wait
    
    function waitForAlpine() {
        attempts++;
        console.log(`[Alpine Init] Checking for Alpine.js (attempt ${attempts}/${maxAttempts})`);
        
        if (checkAlpine()) {
            console.log('[Alpine Init] Alpine.js found, initializing components');
            initializeAlpineComponents();
            return;
        }
        
        if (attempts >= maxAttempts) {
            console.error('[Alpine Init] Failed to load Alpine.js after maximum attempts');
            console.error('[Alpine Init] Dashboard will run in degraded mode');
            initializeFallback();
            return;
        }
        
        // Try again in 100ms
        setTimeout(waitForAlpine, 100);
    }
    
    // Initialize Alpine components
    function initializeAlpineComponents() {
        console.log('[Alpine Init] Registering Alpine components');
        
        // Make sure dashboard function is available globally
        if (typeof dashboard === 'function') {
            console.log('[Alpine Init] Dashboard function found');
            window.dashboard = dashboard;
        } else {
            console.error('[Alpine Init] Dashboard function not found!');
        }
        
        // Log Alpine version
        if (window.Alpine && window.Alpine.version) {
            console.log('[Alpine Init] Alpine version:', window.Alpine.version);
        }
    }
    
    // Fallback initialization without Alpine
    function initializeFallback() {
        console.log('[Alpine Init] Initializing fallback mode without Alpine.js');
        
        // Create a basic dashboard object that can work without Alpine
        window.dashboardFallback = {
            init: function() {
                console.log('[Alpine Init] Running dashboard in fallback mode');
                // Load mock data or show error message
                const container = document.querySelector('[x-data="dashboard()"]');
                if (container) {
                    container.innerHTML = `
                        <div style="padding: 20px; background: #fee; border: 1px solid #fcc; border-radius: 4px; color: #c00;">
                            <h3>Dashboard Loading Error</h3>
                            <p>Failed to load Alpine.js framework. The dashboard requires Alpine.js to function properly.</p>
                            <p>Please check:</p>
                            <ul>
                                <li>Network connectivity</li>
                                <li>Content Security Policy settings</li>
                                <li>JavaScript console for errors</li>
                            </ul>
                            <p>Debug info: Check browser console for detailed error messages.</p>
                        </div>
                    `;
                }
            }
        };
        
        // Try to initialize fallback on DOM ready
        if (document.readyState === 'loading') {
            document.addEventListener('DOMContentLoaded', window.dashboardFallback.init);
        } else {
            window.dashboardFallback.init();
        }
    }
    
    // Start checking for Alpine
    console.log('[Alpine Init] Starting Alpine.js detection');
    waitForAlpine();
    
    // Also listen for Alpine init event
    document.addEventListener('alpine:init', function() {
        console.log('[Alpine Init] Alpine:init event fired');
    });
    
    // Log any Alpine errors
    window.addEventListener('error', function(event) {
        if (event.message && event.message.includes('Alpine')) {
            console.error('[Alpine Init] Alpine error caught:', event.message);
            console.error('[Alpine Init] Error details:', event);
        }
    });
})();