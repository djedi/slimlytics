// Slimlytics Site Settings Page JavaScript

function siteSettings() {
	return {
		site: {
			id: "",
			name: "",
			domain: "",
			created_at: "",
		},
		originalSite: {},
		stats: {},
		loading: true,
		updateLoading: false,
		deleteLoading: false,
		updateSuccess: false,
		updateError: null,
		showDeleteModal: false,
		showClearDataModal: false,
		deleteConfirmation: "",
		clearDataRange: "all",

		async init() {
			// Get current site from SiteManager
			const currentSite = await window.SiteManager.getSelectedSite();
			console.log("Initializing site settings for site:", currentSite);

			if (!currentSite) {
				// No site selected, try to auto-select
				const sites = await window.SiteManager.getAllSites();
				if (sites.length === 0) {
					window.location.href = "/add-site";
					return;
				}
				// Auto-select first site
				window.SiteManager.setSelectedSite(sites[0]);
				await this.loadSite(sites[0].id);
			} else {
				await this.loadSite(currentSite.id);
			}

			await this.loadStats(this.site.id);
		},

		async loadSite(siteId) {
			try {
				const response = await fetch(
					window.SLIMLYTICS_CONFIG.apiEndpoint("/api/sites"),
				);
				if (response.ok) {
					const sites = await response.json();
					const foundSite = sites.find((s) => s.id === siteId);

					if (!foundSite) {
						window.location.href = "/";
						return; // Don't update site if not found
					}

					this.site = foundSite;
					this.originalSite = { ...this.site };
				} else {
					// Mock data for development
					this.site = {
						id: siteId,
						name: "example.com",
						domain: "example.com",
						created_at: "2024-01-15T10:00:00Z",
					};
					this.originalSite = { ...this.site };
				}
			} catch (err) {
				// Mock data for development
				this.site = {
					id: siteId,
					name: "example.com",
					domain: "example.com",
					created_at: "2024-01-15T10:00:00Z",
				};
				this.originalSite = { ...this.site };
			}

			// Only set loading to false if we have a valid site
			if (this.site?.id) {
				this.loading = false;
			}
		},

		async loadStats(siteId) {
			// Load basic statistics for the site
			try {
				// Calculate date range for all-time stats
				const now = new Date();
				const thirtyDaysAgo = new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000);
				
				const response = await fetch(
					window.SLIMLYTICS_CONFIG.apiEndpoint(
						`/api/stats/${siteId}?start=${thirtyDaysAgo.toISOString()}&end=${now.toISOString()}`
					)
				);
				
				if (response.ok) {
					const data = await response.json();
					
					// Format the stats for display
					this.stats = {
						totalPageViews: data.pageViews?.toLocaleString() || "0",
						uniqueVisitors: data.visitors?.toLocaleString() || "0",
						avgTimeOnSite: this.formatDuration(data.avgSessionDuration || 0),
						bounceRate: `${Math.round(data.bounceRate || 0)}%`,
					};
				} else {
					// Fallback to mock data if API fails
					this.stats = {
						totalPageViews: "0",
						uniqueVisitors: "0",
						avgTimeOnSite: "0s",
						bounceRate: "0%",
					};
				}
			} catch (err) {
				console.error("Failed to load stats:", err);
				// Fallback to empty stats on error
				this.stats = {
					totalPageViews: "0",
					uniqueVisitors: "0",
					avgTimeOnSite: "0s",
					bounceRate: "0%",
				};
			}
		},
		
		formatDuration(seconds) {
			if (!seconds || seconds === 0) return "0s";
			const minutes = Math.floor(seconds / 60);
			const secs = Math.round(seconds % 60);
			if (minutes > 0) {
				return `${minutes}m ${secs}s`;
			}
			return `${secs}s`;
		},

		async updateSite() {
			this.updateLoading = true;
			this.updateSuccess = false;
			this.updateError = null;

			try {
				const response = await fetch(
					window.SLIMLYTICS_CONFIG.apiEndpoint(`/api/sites/${this.site.id}`),
					{
						method: "PUT",
						headers: {
							"Content-Type": "application/json",
						},
						body: JSON.stringify({
							name: this.site.name,
							url: this.site.domain, // API expects 'url' but we send domain
						}),
					},
				);

				if (response.ok) {
					this.originalSite = { ...this.site };
					// Update the site in SiteManager so the header reflects changes
					window.SiteManager.setSelectedSite({
						id: this.site.id,
						name: this.site.name,
						domain: this.site.domain,
					});
					this.updateSuccess = true;
					setTimeout(() => {
						this.updateSuccess = false;
					}, 3000);
				} else {
					throw new Error("Failed to update site");
				}
			} catch (err) {
				this.updateError = err.message || "Failed to update site settings";
			} finally {
				this.updateLoading = false;
			}
		},

		resetForm() {
			this.site = { ...this.originalSite };
			this.updateSuccess = false;
			this.updateError = null;
		},

		async deleteSite() {
			if (this.deleteConfirmation !== this.site.name) return;

			this.deleteLoading = true;

			try {
				const response = await fetch(
					window.SLIMLYTICS_CONFIG.apiEndpoint(`/api/sites/${this.site.id}`),
					{
						method: "DELETE",
					},
				);

				if (response.ok) {
					// Redirect to dashboard or add-site if no sites left
					window.location.href = "/";
				} else {
					throw new Error("Failed to delete site");
				}
			} catch (err) {
				alert(`Failed to delete site: ${err.message}`);
			} finally {
				this.deleteLoading = false;
			}
		},

		async clearData() {
			try {
				const response = await fetch(
					window.SLIMLYTICS_CONFIG.apiEndpoint(`/api/stats/${this.site.id}/data`),
					{
						method: "DELETE",
						headers: {
							"Content-Type": "application/json",
						},
						body: JSON.stringify({ range: this.clearDataRange }),
					},
				);

				if (response.ok) {
					const result = await response.json();
					alert(`Successfully cleared ${result.deleted} events from ${this.clearDataRange === 'all' ? 'all time' : this.clearDataRange}`);
					// Reload stats to reflect the change
					await this.loadStats(this.site.id);
				} else {
					const error = await response.json();
					alert(`Failed to clear data: ${error.error || "Unknown error"}`);
				}
			} catch (err) {
				alert(`Failed to clear data: ${err.message}`);
			} finally {
				this.showClearDataModal = false;
				this.clearDataRange = "all"; // Reset to default
			}
		},

		async exportData(format) {
			// In production, this would trigger a download
			alert(`Exporting data as ${format.toUpperCase()}`);
		},

		formatDate(dateString) {
			if (!dateString) return "N/A";
			const date = new Date(dateString);
			return date.toLocaleDateString("en-US", {
				year: "numeric",
				month: "short",
				day: "numeric",
			});
		},
	};
}
