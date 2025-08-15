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
                const response = await fetch('/api/sites');
                const sites = await response.json();
                if (sites && sites.length > 0) {
                    // Sites exist, but user wants to add another
                    this.hasExistingSites = true;
                }
            } catch (err) {
                console.error('Error checking sites:', err);
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
                
                const response = await fetch('/api/sites', {
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
                this.generateTrackingCode(data.id);
                this.siteAdded = true;
                
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