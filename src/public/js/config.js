// Slimlytics Configuration
// This file provides dynamic configuration based on the environment

(function() {
    // Determine the API URL based on the current environment
    function getApiUrl() {
        const hostname = window.location.hostname;
        const protocol = window.location.protocol;
        
        // Check if we're in development (localhost or local IP)
        if (hostname === 'localhost' || hostname === '127.0.0.1' || hostname.startsWith('192.168.')) {
            // Development environment - API runs on port 3000
            return `${protocol}//${hostname}:3000`;
        } else {
            // Production environment - API runs on same domain/port as the frontend
            // This assumes the API is served from the same domain (e.g., behind a reverse proxy)
            return `${protocol}//${window.location.host}`;
        }
    }

    // Create global configuration object
    window.SLIMLYTICS_CONFIG = {
        API_URL: getApiUrl(),
        
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