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
		trackingCode: "",
		copyButtonText: "📋 Copy",

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
			this.generateTrackingCode();
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
			// In production, this would fetch from the API
			this.stats = {
				totalPageViews: "12,345",
				uniqueVisitors: "3,456",
				avgTimeOnSite: "2m 34s",
				bounceRate: "32%",
			};
		},

		generateTrackingCode() {
			if (!this.site) return;

			const apiBase =
				window.location.hostname === "localhost"
					? "http://localhost:3000"
					: window.location.origin;

			// Split the script tags to avoid parsing issues
			const scriptOpen = "<scr" + "ipt>";
			const scriptClose = "</scr" + "ipt>";
			const noscriptOpen = "<noscr" + "ipt>";
			const noscriptClose = "</noscr" + "ipt>";

			this.trackingCode = `<!-- Slimlytics Tracking Code -->
${scriptOpen} async data-id="${this.site.id}" src="${apiBase}/sa.js"${scriptClose}
${noscriptOpen}<p><img alt="Slimlytics" width="1" height="1" src="${apiBase}/${this.site.id}ns.gif" /></p>${noscriptClose}
<!-- End Slimlytics Tracking Code -->`;
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
				this.copyButtonText = "✅ Copied!";
				setTimeout(() => {
					this.copyButtonText = "📋 Copy";
				}, 2000);
			} catch (err) {
				this.copyButtonText = "❌ Failed";
				setTimeout(() => {
					this.copyButtonText = "📋 Copy";
				}, 2000);
			}
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
