// Slimlytics Configuration
// Simplified configuration - API and dashboard served from same origin

(function() {
    // Create global configuration object
    window.SLIMLYTICS_CONFIG = {
        // API is always on the same origin as the dashboard
        API_URL: window.location.origin,
        
        // Helper method to build API endpoints
        apiEndpoint: function(path) {
            console.log('[Config] Building API endpoint for path:', path);
            console.log('[Config] Current API_URL:', this.API_URL);
            // Ensure path starts with /
            if (!path.startsWith('/')) {
                path = '/' + path;
            }
            const fullUrl = this.API_URL + path;
            console.log('[Config] Full API URL:', fullUrl);
            return fullUrl;
        }
    };

    // Always log the configuration for debugging
    console.log('[Config] Slimlytics Config initialized:', {
        API_URL: window.SLIMLYTICS_CONFIG.API_URL,
        hostname: window.location.hostname,
        origin: window.location.origin,
        pathname: window.location.pathname
    });
})();