// Slimlytics Dashboard Main JavaScript

function dashboard() {
    return {
        // Site management
        sites: [],
        selectedSite: null,
        selectedSiteId: null,
        siteDropdownOpen: false,
        
        // Date range management
        selectedDateRange: 'today',
        dateDropdownOpen: false,
        dateRanges: {
            'today': 'Today',
            'yesterday': 'Yesterday',
            '2days': '2 Days Ago',
            'thisweek': 'This Week',
            'lastweek': 'Last Week',
            '7days': 'Last 7 Days',
            '30days': 'Last 30 Days',
            'thismonth': 'This Month',
            'lastmonth': 'Last Month'
        },
        
        async init() {
            // Load sites first
            await this.loadSites();
            
            // Check if we have sites, if not redirect to add-site page
            if (this.sites.length === 0) {
                window.location.href = '/add-site';
                return;
            }
            
            // Initialize with the first site
            if (!this.selectedSite && this.sites.length > 0) {
                this.selectedSite = this.sites[0].name;
                this.selectedSiteId = this.sites[0].id;
            }
            
            this.initChart();
        },
        
        async loadSites() {
            try {
                const response = await fetch('/api/sites');
                if (response.ok) {
                    const sitesData = await response.json();
                    this.sites = sitesData || [];
                } else {
                    // If API doesn't exist yet, use mock data for now
                    this.sites = [
                        {id: '1', name: 'example.com', url: 'https://example.com'},
                        {id: '2', name: 'blog.example.com', url: 'https://blog.example.com'},
                        {id: '3', name: 'shop.example.com', url: 'https://shop.example.com'}
                    ];
                }
            } catch (err) {
                console.log('Sites API not available, using mock data');
                // Use mock data if API isn't ready
                this.sites = [
                    {id: '1', name: 'example.com', url: 'https://example.com'},
                    {id: '2', name: 'blog.example.com', url: 'https://blog.example.com'},
                    {id: '3', name: 'shop.example.com', url: 'https://shop.example.com'}
                ];
            }
        },
        
        selectSite(site) {
            this.selectedSite = site.name;
            this.selectedSiteId = site.id;
            this.siteDropdownOpen = false;
            // In real app, would fetch new data here
        },
        
        goToSiteSettings() {
            if (this.selectedSiteId) {
                window.location.href = `/site-settings?id=${this.selectedSiteId}`;
            }
        },
        
        selectDateRange(range) {
            this.selectedDateRange = range;
            this.dateDropdownOpen = false;
            // In real app, would fetch new data here
        },
        
        chartSvg: '',
        hoveredPoint: null,
        
        initChart() {
            // Mock visitor data (today)
            const todayData = [
                12, 15, 18, 25, 32, 45, 52, 68, 89, 112, 125, 134,
                145, 132, 128, 115, 98, 87, 76, 65, 54, 43, 32, 21
            ];
            
            // Mock visitor data (7 days ago)
            const compareData = [
                8, 11, 14, 19, 24, 35, 42, 58, 76, 98, 105, 112,
                118, 108, 102, 95, 82, 73, 64, 55, 46, 37, 28, 18
            ];
            
            const width = 600;
            const height = 300;
            const padding = { top: 20, right: 30, bottom: 50, left: 50 };
            const chartWidth = width - padding.left - padding.right;
            const chartHeight = height - padding.top - padding.bottom;
            
            // Find max value for scaling
            const maxValue = Math.max(...todayData, ...compareData);
            const yScale = chartHeight / maxValue;
            const xStep = chartWidth / (todayData.length - 1);
            
            // Create path for today's data
            const todayPath = todayData.map((value, i) => {
                const x = padding.left + (i * xStep);
                const y = height - padding.bottom - (value * yScale);
                return `${i === 0 ? 'M' : 'L'} ${x} ${y}`;
            }).join(' ');
            
            // Create area fill for today's data
            const todayArea = todayPath + ` L ${padding.left + chartWidth} ${height - padding.bottom} L ${padding.left} ${height - padding.bottom} Z`;
            
            // Create path for comparison data
            const comparePath = compareData.map((value, i) => {
                const x = padding.left + (i * xStep);
                const y = height - padding.bottom - (value * yScale);
                return `${i === 0 ? 'M' : 'L'} ${x} ${y}`;
            }).join(' ');
            
            // Create Y-axis labels
            const yAxisLabels = [];
            const ySteps = 5;
            for (let i = 0; i <= ySteps; i++) {
                const value = Math.round((maxValue / ySteps) * i);
                const y = height - padding.bottom - (value * yScale);
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
                const x = padding.left + (i * xStep);
                xAxisLabels.push(`
                    <text x="${x}" y="${height - padding.bottom + 20}" text-anchor="middle" 
                          font-size="12" fill="#7f8c8d">${i.toString().padStart(2, '0')}:00</text>
                `);
            }
            
            // Create interactive points for today's data
            const todayPoints = todayData.map((value, i) => {
                const x = padding.left + (i * xStep);
                const y = height - padding.bottom - (value * yScale);
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
            }).join('');
            
            this.chartSvg = `
                <svg width="100%" height="${height}" viewBox="0 0 ${width} ${height}" 
                     style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto;">
                    
                    <!-- Grid lines and labels -->
                    ${yAxisLabels.join('')}
                    
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
                    ${xAxisLabels.join('')}
                    
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
        }
    }
}