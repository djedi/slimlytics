// Slimlytics Dashboard Main JavaScript

function dashboard() {
	return {
		// Site management
		sites: [],
		selectedSite: null,
		selectedSiteId: null,
		siteDropdownOpen: false,

		// Date range management
		selectedDateRange: "today",
		dateDropdownOpen: false,
		dateRanges: {
			today: "Today",
			yesterday: "Yesterday",
			"2days": "2 Days Ago",
			thisweek: "This Week",
			lastweek: "Last Week",
			"7days": "Last 7 Days",
			"30days": "Last 30 Days",
			thismonth: "This Month",
			lastmonth: "Last Month",
		},
		
		// Stats data
		stats: {
			visitors: "0",
			pageViews: "0",
			avgSessionDuration: "0s",
			bounceRate: "0%",
		},
		topPages: [],
		topReferrers: [],
		realtimeVisitors: 0,
		trendLabels: [],
		trendData: [],

		async init() {
			// Load sites first
			await this.loadSites();

			// Check if we have sites, if not redirect to add-site page
			if (this.sites.length === 0) {
				window.location.href = "/add-site";
				return;
			}

			// Initialize with the first site
			if (!this.selectedSite && this.sites.length > 0) {
				this.selectedSite = this.sites[0].name;
				this.selectedSiteId = this.sites[0].id;
			}

			// Load stats for the selected site
			await this.loadStats();
			
			// Start polling for realtime updates
			this.startRealtimeUpdates();

			this.initChart();
		},

		async loadSites() {
			try {
				const response = await fetch(window.SLIMLYTICS_CONFIG.apiEndpoint('/api/sites'));
				if (response.ok) {
					const sitesData = await response.json();
					this.sites = sitesData || [];
				} else {
					// If API doesn't exist yet, use mock data for now
					this.sites = [
						{ id: "1", name: "example.com", domain: "example.com" },
						{ id: "2", name: "blog.example.com", domain: "blog.example.com" },
						{ id: "3", name: "shop.example.com", domain: "shop.example.com" },
					];
				}
			} catch (err) {
				console.log("Sites API not available, using mock data");
				// Use mock data if API isn't ready
				this.sites = [
					{ id: "1", name: "example.com", domain: "example.com" },
					{ id: "2", name: "blog.example.com", domain: "blog.example.com" },
					{ id: "3", name: "shop.example.com", domain: "shop.example.com" },
				];
			}
		},

		selectSite(site) {
			this.selectedSite = site.name;
			this.selectedSiteId = site.id;
			this.siteDropdownOpen = false;
			// Reload stats for the new site
			this.loadStats();
		},

		goToSiteSettings() {
			if (this.selectedSiteId) {
				window.location.href = `/site-settings/?id=${this.selectedSiteId}`;
			}
		},

		selectDateRange(range) {
			this.selectedDateRange = range;
			this.dateDropdownOpen = false;
			// Reload stats with new date range
			this.loadStats();
		},
		
		async loadStats() {
			if (!this.selectedSiteId) return;
			
			try {
				// Calculate date range
				const dateRange = this.getDateRange();
				
				// Fetch dashboard stats
				const statsResponse = await fetch(
					window.SLIMLYTICS_CONFIG.apiEndpoint(
						`/api/stats/${this.selectedSiteId}?start=${dateRange.start}&end=${dateRange.end}`
					)
				);
				
				if (statsResponse.ok) {
					const data = await statsResponse.json();
					
					// Format stats for display
					this.stats = {
						visitors: data.visitors.toLocaleString(),
						pageViews: data.pageViews.toLocaleString(),
						avgSessionDuration: this.formatDuration(data.avgSessionDuration),
						bounceRate: `${Math.round(data.bounceRate)}%`,
					};
					
					this.topPages = data.topPages || [];
					this.topReferrers = data.topReferrers || [];
					this.realtimeVisitors = data.realtimeVisitors || 0;
				} else {
					// Use mock data as fallback
					this.loadMockStats();
				}
				
				// Fetch time series data for chart
				const days = this.getDaysForRange();
				const timeseriesResponse = await fetch(
					window.SLIMLYTICS_CONFIG.apiEndpoint(
						`/api/stats/${this.selectedSiteId}/timeseries?days=${days}`
					)
				);
				
				if (timeseriesResponse.ok) {
					const data = await timeseriesResponse.json();
					this.trendLabels = data.labels;
					this.trendData = data.pageViews;
					this.updateChart();
				}
				
			} catch (error) {
				console.error('Error loading stats:', error);
				this.loadMockStats();
			}
		},
		
		loadMockStats() {
			// Fallback mock stats
			this.stats = {
				visitors: "1,234",
				pageViews: "5,678",
				avgSessionDuration: "2m 34s",
				bounceRate: "45%",
			};
			
			this.topPages = [
				{ url: "/blog/hello-world", views: 523 },
				{ url: "/", views: 412 },
				{ url: "/about", views: 234 },
				{ url: "/contact", views: 156 },
				{ url: "/blog/second-post", views: 89 },
			];
			
			this.topReferrers = [
				{ referrer: "google.com", count: 412 },
				{ referrer: "twitter.com", count: 234 },
				{ referrer: "direct", count: 189 },
				{ referrer: "facebook.com", count: 123 },
				{ referrer: "github.com", count: 78 },
			];
			
			this.realtimeVisitors = Math.floor(Math.random() * 20) + 1;
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
		
		getDateRange() {
			const now = new Date();
			const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
			let start, end;
			
			switch (this.selectedDateRange) {
				case "today":
					start = today;
					end = now;
					break;
				case "yesterday":
					start = new Date(today.getTime() - 24 * 60 * 60 * 1000);
					end = today;
					break;
				case "7days":
					start = new Date(today.getTime() - 7 * 24 * 60 * 60 * 1000);
					end = now;
					break;
				case "30days":
					start = new Date(today.getTime() - 30 * 24 * 60 * 60 * 1000);
					end = now;
					break;
				case "thismonth":
					start = new Date(now.getFullYear(), now.getMonth(), 1);
					end = now;
					break;
				case "lastmonth":
					start = new Date(now.getFullYear(), now.getMonth() - 1, 1);
					end = new Date(now.getFullYear(), now.getMonth(), 0);
					break;
				default:
					start = today;
					end = now;
			}
			
			return {
				start: start.toISOString(),
				end: end.toISOString()
			};
		},
		
		getDaysForRange() {
			switch (this.selectedDateRange) {
				case "today": return 1;
				case "yesterday": return 2;
				case "7days": return 7;
				case "30days": return 30;
				case "thismonth": return new Date().getDate();
				case "lastmonth": return 30;
				default: return 7;
			}
		},
		
		startRealtimeUpdates() {
			// Update realtime visitors every 30 seconds
			setInterval(async () => {
				if (!this.selectedSiteId) return;
				
				try {
					const response = await fetch(
						window.SLIMLYTICS_CONFIG.apiEndpoint(
							`/api/stats/${this.selectedSiteId}/realtime`
						)
					);
					
					if (response.ok) {
						const data = await response.json();
						this.realtimeVisitors = data.visitors || 0;
					}
				} catch (error) {
					console.error('Error fetching realtime visitors:', error);
				}
			}, 30000);
		},

		chartSvg: "",
		hoveredPoint: null,

		initChart() {
			// Initialize with empty data or mock data
			this.updateChart();
		},
		
		updateChart() {
			// Use trend data if available, otherwise use mock data
			const chartData = this.trendData.length > 0 ? this.trendData : [
				12, 15, 18, 25, 32, 45, 52, 68, 89, 112, 125, 134, 145, 132, 128, 115,
				98, 87, 76, 65, 54, 43, 32, 21,
			];

			// For comparison, use previous period or mock data
			const compareData = [
				8, 11, 14, 19, 24, 35, 42, 58, 76, 98, 105, 112, 118, 108, 102, 95, 82,
				73, 64, 55, 46, 37, 28, 18,
			];

			const width = 600;
			const height = 300;
			const padding = { top: 20, right: 30, bottom: 50, left: 50 };
			const chartWidth = width - padding.left - padding.right;
			const chartHeight = height - padding.top - padding.bottom;

			// Find max value for scaling
			const maxValue = Math.max(...chartData, ...compareData);
			const yScale = chartHeight / maxValue;
			const xStep = chartWidth / (chartData.length - 1);

			// Create path for chart data
			const todayPath = chartData
				.map((value, i) => {
					const x = padding.left + i * xStep;
					const y = height - padding.bottom - value * yScale;
					return `${i === 0 ? "M" : "L"} ${x} ${y}`;
				})
				.join(" ");

			// Create area fill for today's data
			const todayArea =
				todayPath +
				` L ${padding.left + chartWidth} ${height - padding.bottom} L ${padding.left} ${height - padding.bottom} Z`;

			// Create path for comparison data
			const comparePath = compareData
				.map((value, i) => {
					const x = padding.left + i * xStep;
					const y = height - padding.bottom - value * yScale;
					return `${i === 0 ? "M" : "L"} ${x} ${y}`;
				})
				.join(" ");

			// Create Y-axis labels
			const yAxisLabels = [];
			const ySteps = 5;
			for (let i = 0; i <= ySteps; i++) {
				const value = Math.round((maxValue / ySteps) * i);
				const y = height - padding.bottom - value * yScale;
				yAxisLabels.push(`
                    <line x1="${padding.left - 5}" y1="${y}" x2="${width - padding.right}" y2="${y}"
                          stroke="#e0e0e0" stroke-dasharray="2,2" />
                    <text x="${padding.left - 10}" y="${y + 5}" text-anchor="end"
                          font-size="12" fill="#7f8c8d">${value}</text>
                `);
			}

			// Create X-axis labels (show every 3 hours)
			const xAxisLabels = [];
			for (let i = 0; i < 24; i += 3) {
				const x = padding.left + i * xStep;
				xAxisLabels.push(`
                    <text x="${x}" y="${height - padding.bottom + 20}" text-anchor="middle"
                          font-size="12" fill="#7f8c8d">${i.toString().padStart(2, "0")}:00</text>
                `);
			}

			// Create interactive points for chart data
			const todayPoints = chartData
				.map((value, i) => {
					const x = padding.left + i * xStep;
					const y = height - padding.bottom - value * yScale;
					return `
                    <circle cx="${x}" cy="${y}" r="4" fill="#3498db" opacity="0"
                            style="cursor: pointer; transition: opacity 0.2s;"
                            onmouseover="this.style.opacity='1'; this.nextElementSibling.style.display='block'"
                            onmouseout="this.style.opacity='0'; this.nextElementSibling.style.display='none'" />
                    <g style="display: none;">
                        <rect x="${x - 30}" y="${y - 35}" width="60" height="25" rx="3"
                              fill="rgba(0,0,0,0.8)" />
                        <text x="${x}" y="${y - 18}" text-anchor="middle"
                              font-size="12" fill="white">${value} visitors</text>
                    </g>
                `;
				})
				.join("");

			this.chartSvg = `
                <svg width="100%" height="${height}" viewBox="0 0 ${width} ${height}"
                     style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto;">

                    <!-- Grid lines and labels -->
                    ${yAxisLabels.join("")}

                    <!-- Today's area fill -->
                    <path d="${todayArea}" fill="rgba(52, 152, 219, 0.1)" />

                    <!-- Comparison line -->
                    <path d="${comparePath}" fill="none" stroke="#95a5a6" stroke-width="1"
                          stroke-dasharray="5,5" opacity="0.7" />

                    <!-- Today's line -->
                    <path d="${todayPath}" fill="none" stroke="#3498db" stroke-width="2" />

                    <!-- X and Y axis lines -->
                    <line x1="${padding.left}" y1="${height - padding.bottom}"
                          x2="${width - padding.right}" y2="${height - padding.bottom}"
                          stroke="#2c3e50" stroke-width="1" />
                    <line x1="${padding.left}" y1="${padding.top}"
                          x2="${padding.left}" y2="${height - padding.bottom}"
                          stroke="#2c3e50" stroke-width="1" />

                    <!-- X-axis labels -->
                    ${xAxisLabels.join("")}

                    <!-- Interactive points -->
                    ${todayPoints}

                    <!-- Legend -->
                    <g transform="translate(${width / 2 - 80}, ${height - 15})">
                        <line x1="0" y1="0" x2="20" y2="0" stroke="#3498db" stroke-width="2" />
                        <text x="25" y="4" font-size="12" fill="#2c3e50">Today</text>

                        <line x1="80" y1="0" x2="100" y2="0" stroke="#95a5a6"
                              stroke-width="1" stroke-dasharray="5,5" />
                        <text x="105" y="4" font-size="12" fill="#2c3e50">7 Days Ago</text>
                    </g>
                </svg>
            `;
		},
	};
}
