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
		topCountries: [],
		topCities: [],
		realtimeVisitors: 0,
		trendLabels: [],
		trendData: [],
		
		// WebSocket connection
		ws: null,
		wsConnected: false,
		wsReconnectTimeout: null,
		wsHeartbeatInterval: null,

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

			// Connect to WebSocket for real-time updates
			this.connectWebSocket();

			this.initChart();
			
			// Clean up on page unload
			window.addEventListener('beforeunload', () => {
				this.disconnectWebSocket();
			});
		},

		async loadSites() {
			try {
				const response = await fetch(
					window.SLIMLYTICS_CONFIG.apiEndpoint("/api/sites"),
				);
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
			// Reconnect WebSocket for the new site
			this.reconnectWebSocket();
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
						`/api/stats/${this.selectedSiteId}?start=${dateRange.start}&end=${dateRange.end}`,
					),
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
					this.topCountries = data.topCountries || [];
					this.topCities = data.topCities || [];
					this.realtimeVisitors = data.realtimeVisitors || 0;
				} else {
					// Use mock data as fallback
					this.loadMockStats();
				}

				// Fetch time series data for chart
				const days = this.getDaysForRange();
				const timeseriesResponse = await fetch(
					window.SLIMLYTICS_CONFIG.apiEndpoint(
						`/api/stats/${this.selectedSiteId}/timeseries?days=${days}`,
					),
				);

				if (timeseriesResponse.ok) {
					const data = await timeseriesResponse.json();
					this.trendLabels = data.labels;
					this.trendData = data.pageViews;
					this.updateChart();
				}
			} catch (error) {
				console.error("Error loading stats:", error);
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

			this.topCountries = [
				{ country: "United States", countryCode: "US", count: 523 },
				{ country: "United Kingdom", countryCode: "GB", count: 234 },
				{ country: "Canada", countryCode: "CA", count: 189 },
				{ country: "Germany", countryCode: "DE", count: 156 },
				{ country: "France", countryCode: "FR", count: 134 },
			];

			this.topCities = [
				{ city: "New York", country: "United States", count: 234 },
				{ city: "London", country: "United Kingdom", count: 189 },
				{ city: "San Francisco", country: "United States", count: 145 },
				{ city: "Toronto", country: "Canada", count: 112 },
				{ city: "Berlin", country: "Germany", count: 98 },
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
			let start;
			let end;

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
				end: end.toISOString(),
			};
		},

		getDaysForRange() {
			switch (this.selectedDateRange) {
				case "today":
					return 1;
				case "yesterday":
					return 2;
				case "7days":
					return 7;
				case "30days":
					return 30;
				case "thismonth":
					return new Date().getDate();
				case "lastmonth":
					return 30;
				default:
					return 7;
			}
		},

		connectWebSocket() {
			if (!this.selectedSiteId) return;
			
			// Get WebSocket URL from API endpoint
			const apiUrl = window.SLIMLYTICS_CONFIG.apiEndpoint("");
			const wsUrl = apiUrl.replace(/^http/, 'ws').replace(/\/$/, '');
			
			try {
				this.ws = new WebSocket(wsUrl);
				
				this.ws.onopen = () => {
					console.log('WebSocket connected');
					this.wsConnected = true;
					
					// Ensure WebSocket is in OPEN state before sending
					if (this.ws.readyState === WebSocket.OPEN) {
						// Subscribe to updates for the selected site
						this.ws.send(JSON.stringify({
							type: 'subscribe',
							siteId: this.selectedSiteId
						}));
					} else {
						// If not open yet, wait a bit and retry
						setTimeout(() => {
							if (this.ws && this.ws.readyState === WebSocket.OPEN) {
								this.ws.send(JSON.stringify({
									type: 'subscribe',
									siteId: this.selectedSiteId
								}));
							}
						}, 100);
					}
					
					// Start heartbeat to keep connection alive
					this.startHeartbeat();
				};
				
				this.ws.onmessage = (event) => {
					try {
						const data = JSON.parse(event.data);
						
						if (data.type === 'stats-update') {
							this.handleStatsUpdate(data.stats);
						} else if (data.type === 'subscribed') {
							console.log('Successfully subscribed to real-time updates');
						}
					} catch (error) {
						console.error('Error parsing WebSocket message:', error);
					}
				};
				
				this.ws.onerror = (error) => {
					console.error('WebSocket error:', error);
				};
				
				this.ws.onclose = () => {
					console.log('WebSocket disconnected');
					this.wsConnected = false;
					this.stopHeartbeat();
					// Attempt to reconnect after 5 seconds
					this.wsReconnectTimeout = setTimeout(() => {
						this.connectWebSocket();
					}, 5000);
				};
			} catch (error) {
				console.error('Failed to connect WebSocket:', error);
			}
		},
		
		disconnectWebSocket() {
			if (this.wsReconnectTimeout) {
				clearTimeout(this.wsReconnectTimeout);
				this.wsReconnectTimeout = null;
			}
			
			this.stopHeartbeat();
			
			if (this.ws) {
				this.ws.close();
				this.ws = null;
			}
		},
		
		reconnectWebSocket() {
			this.disconnectWebSocket();
			this.connectWebSocket();
		},
		
		startHeartbeat() {
			this.wsHeartbeatInterval = setInterval(() => {
				if (this.ws && this.ws.readyState === WebSocket.OPEN) {
					this.ws.send(JSON.stringify({ type: 'ping' }));
				}
			}, 30000); // Send ping every 30 seconds
		},
		
		stopHeartbeat() {
			if (this.wsHeartbeatInterval) {
				clearInterval(this.wsHeartbeatInterval);
				this.wsHeartbeatInterval = null;
			}
		},
		
		handleStatsUpdate(newStats) {
			// Update stats with animation effect
			const updateStat = (key, value) => {
				const element = document.querySelector(`[x-text="stats.${key}"]`);
				if (element) {
					element.style.transition = 'color 0.3s';
					element.style.color = '#3498db';
					setTimeout(() => {
						element.style.color = '';
					}, 300);
				}
			};
			
			// Format and update stats
			if (newStats.visitors !== undefined) {
				const formatted = newStats.visitors.toLocaleString();
				if (this.stats.visitors !== formatted) {
					this.stats.visitors = formatted;
					updateStat('visitors', formatted);
				}
			}
			
			if (newStats.pageViews !== undefined) {
				const formatted = newStats.pageViews.toLocaleString();
				if (this.stats.pageViews !== formatted) {
					this.stats.pageViews = formatted;
					updateStat('pageViews', formatted);
				}
			}
			
			if (newStats.avgSessionDuration !== undefined) {
				const formatted = this.formatDuration(newStats.avgSessionDuration);
				if (this.stats.avgSessionDuration !== formatted) {
					this.stats.avgSessionDuration = formatted;
					updateStat('avgSessionDuration', formatted);
				}
			}
			
			if (newStats.bounceRate !== undefined) {
				const formatted = `${Math.round(newStats.bounceRate)}%`;
				if (this.stats.bounceRate !== formatted) {
					this.stats.bounceRate = formatted;
					updateStat('bounceRate', formatted);
				}
			}
			
			// Update other data
			if (newStats.topPages) {
				this.topPages = newStats.topPages;
			}
			
			if (newStats.topReferrers) {
				this.topReferrers = newStats.topReferrers;
			}
			
			if (newStats.topCountries) {
				this.topCountries = newStats.topCountries;
			}
			
			if (newStats.topCities) {
				this.topCities = newStats.topCities;
			}
			
			if (newStats.realtimeVisitors !== undefined) {
				this.realtimeVisitors = newStats.realtimeVisitors;
			}
			
			// Optionally refresh the chart with new data
			if (newStats.timeSeriesData) {
				this.trendLabels = newStats.timeSeriesData.labels;
				this.trendData = newStats.timeSeriesData.pageViews;
				this.updateChart();
			}
		},
		
		getFlagEmoji(countryCode) {
			// Convert country code to flag emoji
			if (!countryCode || countryCode.length !== 2) return '';
			
			const codePoints = countryCode
				.toUpperCase()
				.split('')
				.map(char => 127397 + char.charCodeAt(0));
			
			return String.fromCodePoint(...codePoints);
		},

		chartSvg: "",
		hoveredPoint: null,

		initChart() {
			// Initialize with empty data or mock data
			this.updateChart();
		},

		updateChart() {
			// Use trend data if available, otherwise use mock data
			const chartData =
				this.trendData.length > 0
					? this.trendData
					: [
							12, 15, 18, 25, 32, 45, 52, 68, 89, 112, 125, 134, 145, 132, 128,
							115, 98, 87, 76, 65, 54, 43, 32, 21,
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

			// Ensure we have at least 2 data points to draw a line
			if (chartData.length < 2) {
				this.chartSvg = `
					<svg width="100%" height="300" viewBox="0 0 600 300">
						<text x="300" y="150" text-anchor="middle" font-size="14" fill="#7f8c8d">
							No data available yet
						</text>
					</svg>
				`;
				return;
			}

			// Find max value for scaling (ensure min value of 1 to prevent division by 0)
			const maxValue = Math.max(...chartData, ...compareData, 1);
			const yScale = chartHeight / maxValue;
			const xStep = chartWidth / Math.max(chartData.length - 1, 1); // Prevent division by 0

			// Create path for chart data
			const todayPath = chartData
				.map((value, i) => {
					const x = padding.left + i * xStep;
					const y = height - padding.bottom - value * yScale;
					return `${i === 0 ? "M" : "L"} ${x} ${y}`;
				})
				.join(" ");

			// Create area fill for today's data
			const todayArea = `${todayPath} L ${padding.left + chartWidth} ${height - padding.bottom} L ${padding.left} ${height - padding.bottom} Z`;

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
