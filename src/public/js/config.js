// Slimlytics Configuration
// Simplified configuration - API and dashboard served from same origin

(function() {
    // Create global configuration object
    window.SLIMLYTICS_CONFIG = {
        // API is always on the same origin as the dashboard
        API_URL: window.location.origin,
        
        // Helper method to build API endpoints
        apiEndpoint: function(path) {
            // Ensure path starts with /
            if (!path.startsWith('/')) {
                path = '/' + path;
            }
            return this.API_URL + path;
        }
    };

    // Log the configuration in development
    if (window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1') {
        console.log('Slimlytics Config:', window.SLIMLYTICS_CONFIG);
    }
})();