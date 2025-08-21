// Slimlytics Dashboard Main JavaScript

function dashboard() {
	return {
		// Site management
		selectedSiteId: null,

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
		topLocales: [],
		trafficSources: [],
		searchQueries: [],
		recentVisitors: [],
		realtimeVisitors: 0,
		trendLabels: [],
		trendVisitors: [],
		trendPageViews: [],
		compareVisitors: [],
		comparePageViews: [],

		// WebSocket connection
		ws: null,
		wsConnected: false,
		wsReconnectTimeout: null,
		wsHeartbeatInterval: null,

		async init() {
			console.log('[Dashboard] Starting initialization...');
			console.log('[Dashboard] Config:', window.SLIMLYTICS_CONFIG);
			console.log('[Dashboard] SiteManager available:', !!window.SiteManager);
			
			// Get current site from SiteManager
			const currentSite = await window.SiteManager.getSelectedSite();
			console.log('[Dashboard] Current site from SiteManager:', currentSite);

			if (!currentSite) {
				// No site selected, check if we need to redirect
				const sites = await window.SiteManager.getAllSites();
				console.log('[Dashboard] All sites from SiteManager:', sites);
				
				// Check if we got an empty array due to API error
				// If so, don't redirect to avoid loops
				if (sites.length === 0) {
					// Check if we're already on the add-site page
					if (window.location.pathname === '/add-site' || window.location.pathname === '/add-site.html') {
						// Already on add-site page, don't redirect
						console.warn('No sites available and already on add-site page');
						return;
					}
					
					// Only redirect if we're sure the API is working
					// Try a simple health check first
					try {
						const apiUrl = window.SLIMLYTICS_CONFIG.apiEndpoint('/api/sites');
						console.log('[Dashboard] Health check URL:', apiUrl);
						const healthCheck = await fetch(apiUrl);
						console.log('[Dashboard] Health check response status:', healthCheck.status);
						console.log('[Dashboard] Health check content-type:', healthCheck.headers.get('content-type'));
						if (!healthCheck.ok || !healthCheck.headers.get('content-type')?.includes('application/json')) {
							console.error('[Dashboard] API appears to be down, not redirecting to avoid loops');
							this.stats = { error: 'Unable to connect to API. Please try again later.' };
							return;
						}
					} catch (err) {
						console.error('[Dashboard] API connection failed:', err);
						console.error('[Dashboard] Error details:', err.message, err.stack);
						this.stats = { error: 'Unable to connect to API. Please try again later.' };
						return;
					}
					
					// API is working but no sites exist, safe to redirect
					window.location.href = "/add-site";
					return;
				}
				// Auto-select first site
				window.SiteManager.setSelectedSite(sites[0]);
				this.selectedSiteId = sites[0].id;
			} else {
				this.selectedSiteId = currentSite.id;
			}

			// Load stats for the selected site
			await this.loadStats();

			// Connect to WebSocket for real-time updates
			this.connectWebSocket();

			this.initChart();

			// Clean up on page unload
			window.addEventListener("beforeunload", () => {
				this.disconnectWebSocket();
			});
		},

		selectDateRange(range) {
			this.selectedDateRange = range;
			this.dateDropdownOpen = false;
			// Reload stats with new date range
			this.loadStats();
		},

		async loadStats() {
			console.log('[Dashboard] Loading stats for site:', this.selectedSiteId);
			if (!this.selectedSiteId) {
				console.warn('[Dashboard] No site selected, cannot load stats');
				return;
			}

			try {
				// Calculate date range
				const dateRange = this.getDateRange();
				console.log('[Dashboard] Date range:', dateRange);

				// Fetch dashboard stats
				const statsUrl = window.SLIMLYTICS_CONFIG.apiEndpoint(
					`/api/stats/${this.selectedSiteId}?start=${dateRange.start}&end=${dateRange.end}`,
				);
				console.log('[Dashboard] Fetching stats from:', statsUrl);
				// Use cached fetch if available
				const statsResponse = window.APICache 
					? await window.APICache.fetch(statsUrl)
					: await fetch(statsUrl);
				console.log('[Dashboard] Stats response status:', statsResponse.status);

				if (statsResponse.ok) {
					const data = await statsResponse.json();
					console.log('[Dashboard] Stats data received:', data);

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
					this.topLocales = data.topLocales || [];
					this.trafficSources = data.trafficSources || [];
					this.searchQueries = data.searchQueries || [];
					this.recentVisitors = data.recentVisitors || [];
					this.realtimeVisitors = data.realtimeVisitors || 0;
				} else {
					console.error('[Dashboard] Stats response not OK, using mock data. Status:', statsResponse.status);
					try {
						const errorText = await statsResponse.text();
						console.error('[Dashboard] Error response body:', errorText);
					} catch (e) {
						console.error('[Dashboard] Could not read error response body');
					}
					// Use mock data as fallback
					this.loadMockStats();
				}

				// Fetch time series data for chart
				const timeseriesUrl = window.SLIMLYTICS_CONFIG.apiEndpoint(
					`/api/stats/${this.selectedSiteId}/timeseries?start=${dateRange.start}&end=${dateRange.end}`,
				);
				// Use cached fetch if available
				const timeseriesResponse = window.APICache
					? await window.APICache.fetch(timeseriesUrl)
					: await fetch(timeseriesUrl);

				if (timeseriesResponse.ok) {
					const data = await timeseriesResponse.json();
					this.trendLabels = data.labels;
					this.trendVisitors = data.visitors;
					this.trendPageViews = data.pageViews;

					// Calculate previous period for comparison
					const currentStart = new Date(dateRange.start);
					const currentEnd = new Date(dateRange.end);
					const duration = currentEnd.getTime() - currentStart.getTime();

					const compareStart = new Date(currentStart.getTime() - duration);
					const compareEnd = currentStart;

					// Fetch comparison data (previous period)
					const compareUrl = window.SLIMLYTICS_CONFIG.apiEndpoint(
						`/api/stats/${this.selectedSiteId}/timeseries?start=${compareStart.toISOString()}&end=${compareEnd.toISOString()}`,
					);
					// Use cached fetch if available
					const compareResponse = window.APICache
						? await window.APICache.fetch(compareUrl)
						: await fetch(compareUrl);

					if (compareResponse.ok) {
						const compareData = await compareResponse.json();
						this.compareVisitors = compareData.visitors;
						this.comparePageViews = compareData.pageViews;
					}

					this.updateChart();
				}
			} catch (error) {
				console.error('[Dashboard] Error loading stats:', error);
				console.error('[Dashboard] Error details:', error.message, error.stack);
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

			this.searchQueries = [
				{ query: "analytics dashboard", count: 89 },
				{ query: "web analytics tool", count: 67 },
				{ query: "google analytics alternative", count: 45 },
				{ query: "website statistics", count: 34 },
				{ query: "visitor tracking", count: 28 },
				{ query: "real time analytics", count: 23 },
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
				case "2days":
					start = new Date(today.getTime() - 2 * 24 * 60 * 60 * 1000);
					end = new Date(today.getTime() - 24 * 60 * 60 * 1000);
					break;
				case "thisweek": {
					// Start from Monday of current week
					const dayOfWeek = now.getDay();
					const daysFromMonday = dayOfWeek === 0 ? 6 : dayOfWeek - 1;
					start = new Date(
						today.getTime() - daysFromMonday * 24 * 60 * 60 * 1000,
					);
					end = now;
					break;
				}
				case "lastweek": {
					// Full previous week (Monday to Sunday)
					const currentDayOfWeek = now.getDay();
					const daysToLastMonday =
						currentDayOfWeek === 0 ? 13 : currentDayOfWeek + 6;
					start = new Date(
						today.getTime() - daysToLastMonday * 24 * 60 * 60 * 1000,
					);
					end = new Date(start.getTime() + 7 * 24 * 60 * 60 * 1000);
					break;
				}
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
				case "2days":
					return 3;
				case "thisweek": {
					const dayOfWeek = new Date().getDay();
					const daysFromMonday = dayOfWeek === 0 ? 6 : dayOfWeek - 1;
					return daysFromMonday + 1;
				}
				case "lastweek":
					return 7;
				case "7days":
					return 7;
				case "30days":
					return 30;
				case "thismonth":
					return new Date().getDate();
				case "lastmonth": {
					const now = new Date();
					const lastMonth = new Date(now.getFullYear(), now.getMonth(), 0);
					return lastMonth.getDate();
				}
				default:
					return 7;
			}
		},

		connectWebSocket() {
			console.log('[Dashboard] Connecting WebSocket for site:', this.selectedSiteId);
			if (!this.selectedSiteId) {
				console.warn('[Dashboard] No site selected, skipping WebSocket connection');
				return;
			}

			// Get WebSocket URL from API endpoint
			const apiUrl = window.SLIMLYTICS_CONFIG.apiEndpoint("");
			const wsUrl = apiUrl.replace(/^http/, "ws").replace(/\/$/, "");
			console.log('[Dashboard] WebSocket URL:', wsUrl);

			try {
				this.ws = new WebSocket(wsUrl);

				this.ws.onopen = () => {
					console.log('[Dashboard] WebSocket connected successfully');
					this.wsConnected = true;

					// Ensure WebSocket is in OPEN state before sending
					if (this.ws.readyState === WebSocket.OPEN) {
						// Subscribe to updates for the selected site
						this.ws.send(
							JSON.stringify({
								type: "subscribe",
								siteId: this.selectedSiteId,
							}),
						);
					} else {
						// If not open yet, wait a bit and retry
						setTimeout(() => {
							if (this.ws && this.ws.readyState === WebSocket.OPEN) {
								this.ws.send(
									JSON.stringify({
										type: "subscribe",
										siteId: this.selectedSiteId,
									}),
								);
							}
						}, 100);
					}

					// Start heartbeat to keep connection alive
					this.startHeartbeat();
				};

				this.ws.onmessage = (event) => {
					try {
						const data = JSON.parse(event.data);

						if (data.type === "stats-update") {
							this.handleStatsUpdate(data.stats);
						} else if (data.type === "subscribed") {
							console.log("Successfully subscribed to real-time updates");
						}
					} catch (error) {
						console.error("Error parsing WebSocket message:", error);
					}
				};

				this.ws.onerror = (error) => {
					console.error('[Dashboard] WebSocket error:', error);
					console.error('[Dashboard] WebSocket readyState:', this.ws.readyState);
				};

				this.ws.onclose = (event) => {
					console.log('[Dashboard] WebSocket disconnected. Code:', event.code, 'Reason:', event.reason);
					this.wsConnected = false;
					this.stopHeartbeat();
					// Attempt to reconnect after 5 seconds
					this.wsReconnectTimeout = setTimeout(() => {
						this.connectWebSocket();
					}, 5000);
				};
			} catch (error) {
				console.error('[Dashboard] Failed to connect WebSocket:', error);
				console.error('[Dashboard] WebSocket error details:', error.message, error.stack);
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
					this.ws.send(JSON.stringify({ type: "ping" }));
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
					element.style.transition = "color 0.3s";
					element.style.color = "#3498db";
					setTimeout(() => {
						element.style.color = "";
					}, 300);
				}
			};

			// Format and update stats
			if (newStats.visitors !== undefined) {
				const formatted = newStats.visitors.toLocaleString();
				if (this.stats.visitors !== formatted) {
					this.stats.visitors = formatted;
					updateStat("visitors", formatted);
				}
			}

			if (newStats.pageViews !== undefined) {
				const formatted = newStats.pageViews.toLocaleString();
				if (this.stats.pageViews !== formatted) {
					this.stats.pageViews = formatted;
					updateStat("pageViews", formatted);
				}
			}

			if (newStats.avgSessionDuration !== undefined) {
				const formatted = this.formatDuration(newStats.avgSessionDuration);
				if (this.stats.avgSessionDuration !== formatted) {
					this.stats.avgSessionDuration = formatted;
					updateStat("avgSessionDuration", formatted);
				}
			}

			if (newStats.bounceRate !== undefined) {
				const formatted = `${Math.round(newStats.bounceRate)}%`;
				if (this.stats.bounceRate !== formatted) {
					this.stats.bounceRate = formatted;
					updateStat("bounceRate", formatted);
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

			if (newStats.topLocales) {
				this.topLocales = newStats.topLocales;
			}

			if (newStats.trafficSources) {
				this.trafficSources = newStats.trafficSources;
			}

			if (newStats.searchQueries) {
				this.searchQueries = newStats.searchQueries;
			}

			if (newStats.recentVisitors) {
				this.recentVisitors = newStats.recentVisitors;
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
			if (!countryCode || countryCode.length !== 2) return "";

			const codePoints = countryCode
				.toUpperCase()
				.split("")
				.map((char) => 127397 + char.charCodeAt(0));

			return String.fromCodePoint(...codePoints);
		},

		getPathFromUrl(url) {
			// Extract path from full URL
			if (!url) return '/';
			try {
				const urlObj = new URL(url);
				return urlObj.pathname || '/';
			} catch (e) {
				// If URL parsing fails, assume it's already a path
				return url;
			}
		},

		getLocaleFlagEmoji(locale) {
			// Extract country code from locale (e.g., "en-US" -> "US")
			const parts = locale.split("-");
			if (parts.length > 1) {
				return this.getFlagEmoji(parts[1]);
			}
			// Default flags for common language codes without country
			const languageDefaults = {
				en: "GB",
				es: "ES",
				fr: "FR",
				de: "DE",
				it: "IT",
				pt: "PT",
				ru: "RU",
				ja: "JP",
				ko: "KR",
				zh: "CN",
				ar: "SA",
				hi: "IN",
			};
			return this.getFlagEmoji(languageDefaults[locale] || "");
		},

		formatTimeAgo(timestamp) {
			const now = new Date();
			const visitTime = new Date(timestamp);
			const diffInSeconds = Math.floor((now - visitTime) / 1000);

			if (diffInSeconds < 60) {
				return "just now";
			}
			if (diffInSeconds < 3600) {
				const minutes = Math.floor(diffInSeconds / 60);
				return `${minutes} min${minutes > 1 ? "s" : ""} ago`;
			}
			if (diffInSeconds < 86400) {
				const hours = Math.floor(diffInSeconds / 3600);
				return `${hours} hour${hours > 1 ? "s" : ""} ago`;
			}
			const days = Math.floor(diffInSeconds / 86400);
			return `${days} day${days > 1 ? "s" : ""} ago`;
		},

		formatLocation(visitor) {
			const parts = [];
			if (visitor.city) parts.push(visitor.city);
			if (visitor.country) parts.push(visitor.country);
			return parts.length > 0 ? parts.join(", ") : "Unknown";
		},

		maskIpAddress(ipHash) {
			// Since we're storing hashed IPs, just show a masked format
			// Take first 8 chars of hash and format like an IP
			if (!ipHash) return "***.***.***";
			const shortHash = ipHash.substring(0, 8);
			return `${shortHash.substring(0, 3)}.${shortHash.substring(3, 6)}.***`;
		},

		chartSvg: "",
		hoveredPoint: null,

		initChart() {
			// Initialize with empty data or mock data
			this.updateChart();
		},

		updateChart() {
			// Use visitors data (not pageviews) for the chart
			const chartData =
				this.trendVisitors.length > 0
					? this.trendVisitors
					: [12, 15, 18, 25, 32, 45, 52, 68, 89, 112, 125, 134, 145, 132, 128];

			// Use comparison visitors data if available
			const compareData =
				this.compareVisitors.length > 0
					? this.compareVisitors
					: chartData.map((v) => Math.floor(v * 0.8)); // Default to 80% of current if no comparison data

			// Use the actual labels from API
			const labels =
				this.trendLabels.length > 0
					? this.trendLabels
					: ["Jan 1", "Jan 2", "Jan 3", "Jan 4", "Jan 5"];

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

			// Create path for comparison data (only if we have data)
			const comparePath =
				compareData.length > 0
					? compareData
							.slice(0, chartData.length) // Ensure same length as current data
							.map((value, i) => {
								const x = padding.left + i * xStep;
								const y = height - padding.bottom - value * yScale;
								return `${i === 0 ? "M" : "L"} ${x} ${y}`;
							})
							.join(" ")
					: "";

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

			// Create X-axis labels - show every Nth label to avoid crowding
			const xAxisLabels = [];
			const labelInterval = Math.ceil(labels.length / 7); // Show max 7 labels

			labels.forEach((label, i) => {
				if (i % labelInterval === 0 || i === labels.length - 1) {
					const x = padding.left + i * xStep;
					xAxisLabels.push(`
						<text x="${x}" y="${height - padding.bottom + 20}" text-anchor="middle"
							  font-size="11" fill="#7f8c8d">${label}</text>
					`);
				}
			});

			// Create interactive points for chart data
			const todayPoints = chartData
				.map((value, i) => {
					const x = padding.left + i * xStep;
					const y = height - padding.bottom - value * yScale;
					const label = labels[i] || "";
					const compareValue = compareData[i] || 0;
					const diff = value - compareValue;
					const diffPercent =
						compareValue > 0 ? Math.round((diff / compareValue) * 100) : 0;
					const diffSign = diff >= 0 ? "+" : "";

					return `
                    <circle cx="${x}" cy="${y}" r="3" fill="#3498db" opacity="0"
                            style="cursor: pointer; transition: all 0.2s;"
                            onmouseover="this.style.opacity='1'; this.setAttribute('r', '5'); this.nextElementSibling.style.display='block'"
                            onmouseout="this.style.opacity='0'; this.setAttribute('r', '3'); this.nextElementSibling.style.display='none'" />
                    <g style="display: none; pointer-events: none;">
                        <rect x="${x - 50}" y="${y - 55}" width="100" height="45" rx="4"
                              fill="rgba(0,0,0,0.85)" />
                        <text x="${x}" y="${y - 38}" text-anchor="middle"
                              font-size="11" fill="white" font-weight="bold">${label}</text>
                        <text x="${x}" y="${y - 24}" text-anchor="middle"
                              font-size="12" fill="white">${value.toLocaleString()} visitors</text>
                        <text x="${x}" y="${y - 10}" text-anchor="middle"
                              font-size="10" fill="${diff >= 0 ? "#27ae60" : "#e74c3c"}">${diffSign}${diffPercent}% vs prev</text>
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
                    <g transform="translate(${width / 2 - 100}, ${height - 15})">
                        <line x1="0" y1="0" x2="20" y2="0" stroke="#3498db" stroke-width="2" />
                        <text x="25" y="4" font-size="11" fill="#2c3e50">Current Period</text>

                        <line x1="110" y1="0" x2="130" y2="0" stroke="#95a5a6"
                              stroke-width="1" stroke-dasharray="5,5" />
                        <text x="135" y="4" font-size="11" fill="#2c3e50">Previous Period</text>
                    </g>
                </svg>
            `;
		},
	};
}
