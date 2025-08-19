// Slimlytics Add Site Page JavaScript

function addSite() {
    return {
        siteName: '',
        siteUrl: '',
        siteId: null,
        loading: false,
        error: null,
        siteAdded: false,
        trackingCode: '',
        copyButtonText: '📋 Copy',
        hasExistingSites: false,
        
        async init() {
            // Check if we have existing sites
            try {
                const response = await fetch(window.SLIMLYTICS_CONFIG.apiEndpoint('/api/sites'));
                
                // Check if response is OK and is JSON
                if (!response.ok) {
                    console.error('API request failed with status:', response.status);
                    // Don't redirect if API is down to avoid loops
                    this.error = 'Unable to connect to the API. Please check if the service is running.';
                    return;
                }
                
                const contentType = response.headers.get('content-type');
                if (!contentType || !contentType.includes('application/json')) {
                    console.error('API returned non-JSON response:', contentType);
                    this.error = 'API returned an unexpected response. Please try again later.';
                    return;
                }
                
                const sites = await response.json();
                if (sites && sites.length > 0) {
                    // Sites exist, but user wants to add another
                    this.hasExistingSites = true;
                }
            } catch (err) {
                console.error('Error checking sites:', err);
                this.error = 'Unable to load sites. Please try again later.';
            }
        },
        
        async addSite() {
            this.loading = true;
            this.error = null;
            
            try {
                // Normalize URL
                let url = this.siteUrl.trim();
                if (!url.startsWith('http://') && !url.startsWith('https://')) {
                    url = 'https://' + url;
                }
                
                const response = await fetch(window.SLIMLYTICS_CONFIG.apiEndpoint('/api/sites'), {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({
                        name: this.siteName.trim(),
                        url: url
                    })
                });
                
                if (!response.ok) {
                    throw new Error('Failed to add site');
                }
                
                const data = await response.json();
                this.siteId = data.id;
                
                // Set the newly added site as the selected site
                window.SiteManager.setSelectedSite({
                    id: data.id,
                    name: this.siteName.trim(),
                    domain: data.domain
                });
                
                this.generateTrackingCode(data.id);
                this.siteAdded = true;
                
                // Redirect to dashboard after a short delay to show success message
                setTimeout(() => {
                    window.location.href = '/';
                }, 2000);
                
            } catch (err) {
                this.error = err.message || 'Failed to add site. Please try again.';
            } finally {
                this.loading = false;
            }
        },
        
        generateTrackingCode(siteId) {
            // Generate the tracking code for the site
            const baseUrl = window.location.origin;
            // Split the script tags to avoid parsing issues
            const scriptOpen = '<scr' + 'ipt>';
            const scriptClose = '</scr' + 'ipt>';
            
            this.trackingCode = `<!-- Slimlytics Tracking Code -->
${scriptOpen}
(function() {
    var script = document.createElement('script');
    script.src = '${baseUrl}/t.js';
    script.async = true;
    script.dataset.site = '${siteId}';
    document.head.appendChild(script);
})();
${scriptClose}
<!-- End Slimlytics Tracking Code -->`;
        },
        
        async copyTrackingCode() {
            try {
                await navigator.clipboard.writeText(this.trackingCode);
                this.copyButtonText = '✅ Copied!';
                setTimeout(() => {
                    this.copyButtonText = '📋 Copy';
                }, 2000);
            } catch (err) {
                this.copyButtonText = '❌ Failed';
                setTimeout(() => {
                    this.copyButtonText = '📋 Copy';
                }, 2000);
            }
        }
    }
}