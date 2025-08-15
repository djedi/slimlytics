// Slimlytics Site Settings Page JavaScript

function siteSettings() {
    return {
        site: null,
        originalSite: null,
        stats: {},
        loading: true,
        updateLoading: false,
        deleteLoading: false,
        updateSuccess: false,
        updateError: null,
        showDeleteModal: false,
        showClearDataModal: false,
        deleteConfirmation: '',
        clearDataRange: 'all',
        trackingCode: '',
        copyButtonText: '📋 Copy',
        
        async init() {
            // Get site ID from URL params
            const urlParams = new URLSearchParams(window.location.search);
            const siteId = urlParams.get('id');
            
            if (!siteId) {
                window.location.href = '/';
                return;
            }
            
            await this.loadSite(siteId);
            await this.loadStats(siteId);
            this.generateTrackingCode();
        },
        
        async loadSite(siteId) {
            try {
                const response = await fetch('/api/sites');
                if (response.ok) {
                    const sites = await response.json();
                    this.site = sites.find(s => s.id === siteId);
                    this.originalSite = {...this.site};
                    
                    if (!this.site) {
                        window.location.href = '/';
                    }
                } else {
                    // Mock data for development
                    this.site = {
                        id: siteId,
                        name: 'example.com',
                        url: 'https://example.com',
                        created_at: '2024-01-15T10:00:00Z'
                    };
                    this.originalSite = {...this.site};
                }
            } catch (err) {
                // Mock data for development
                this.site = {
                    id: siteId,
                    name: 'example.com',
                    url: 'https://example.com',
                    created_at: '2024-01-15T10:00:00Z'
                };
                this.originalSite = {...this.site};
            } finally {
                this.loading = false;
            }
        },
        
        async loadStats(siteId) {
            // Load basic statistics for the site
            // In production, this would fetch from the API
            this.stats = {
                totalPageViews: '12,345',
                uniqueVisitors: '3,456',
                avgTimeOnSite: '2m 34s',
                bounceRate: '32%'
            };
        },
        
        generateTrackingCode() {
            if (!this.site) return;
            
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
    script.dataset.site = '${this.site.id}';
    document.head.appendChild(script);
})();
${scriptClose}
<!-- End Slimlytics Tracking Code -->`;
        },
        
        async updateSite() {
            this.updateLoading = true;
            this.updateSuccess = false;
            this.updateError = null;
            
            try {
                const response = await fetch(`/api/sites/${this.site.id}`, {
                    method: 'PUT',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({
                        name: this.site.name,
                        url: this.site.url
                    })
                });
                
                if (response.ok) {
                    this.originalSite = {...this.site};
                    this.updateSuccess = true;
                    setTimeout(() => {
                        this.updateSuccess = false;
                    }, 3000);
                } else {
                    throw new Error('Failed to update site');
                }
            } catch (err) {
                this.updateError = err.message || 'Failed to update site settings';
            } finally {
                this.updateLoading = false;
            }
        },
        
        resetForm() {
            this.site = {...this.originalSite};
            this.updateSuccess = false;
            this.updateError = null;
        },
        
        async deleteSite() {
            if (this.deleteConfirmation !== this.site.name) return;
            
            this.deleteLoading = true;
            
            try {
                const response = await fetch(`/api/sites/${this.site.id}`, {
                    method: 'DELETE'
                });
                
                if (response.ok) {
                    // Redirect to dashboard or add-site if no sites left
                    window.location.href = '/';
                } else {
                    throw new Error('Failed to delete site');
                }
            } catch (err) {
                alert('Failed to delete site: ' + err.message);
            } finally {
                this.deleteLoading = false;
            }
        },
        
        async clearData() {
            // In production, this would call an API to clear data
            alert(`Clearing ${this.clearDataRange} data for ${this.site.name}`);
            this.showClearDataModal = false;
        },
        
        async exportData(format) {
            // In production, this would trigger a download
            alert(`Exporting data as ${format.toUpperCase()}`);
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
        },
        
        formatDate(dateString) {
            if (!dateString) return 'N/A';
            const date = new Date(dateString);
            return date.toLocaleDateString('en-US', { 
                year: 'numeric', 
                month: 'short', 
                day: 'numeric' 
            });
        }
    }
}