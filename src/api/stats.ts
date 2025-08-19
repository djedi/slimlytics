import { Database } from 'bun:sqlite';
import { existsSync } from 'fs';
import { resolve } from 'path';

// Resolve database path relative to project root
const dbPath = resolve(process.cwd(), 'data/analytics.db');
console.log('[Stats API] Database path:', dbPath);
console.log('[Stats API] Database exists:', existsSync(dbPath));

const db = new Database(dbPath, { create: true });
console.log('[Stats API] Database connected successfully');

export interface RecentVisitor {
    ip_hash: string;
    page_url: string;
    timestamp: string;
    country: string;
    country_code: string;
    city: string;
    region: string;
}

export interface TrafficSource {
    source: string;
    icon: string;
    count: number;
    percentage: number;
}

export interface DashboardStats {
    visitors: number;
    pageViews: number;
    avgSessionDuration: number;
    bounceRate: number;
    topPages: Array<{ url: string; views: number }>;
    topReferrers: Array<{ referrer: string; count: number }>;
    topCountries: Array<{ country: string; countryCode: string; count: number }>;
    topCities: Array<{ city: string; country: string; count: number }>;
    topLocales: Array<{ locale: string; language: string; country: string; count: number; percentage: number }>;
    trafficSources: TrafficSource[];
    searchQueries: Array<{ query: string; count: number }>;
    realtimeVisitors: number;
    visitorsTrend: number; // percentage change
    pageViewsTrend: number; // percentage change
    recentVisitors?: RecentVisitor[];
}

export interface TimeSeriesData {
    labels: string[];
    visitors: number[];
    pageViews: number[];
}

// Extract search query from referrer URL
function extractSearchQuery(referrer: string): string | null {
    if (!referrer) return null;
    
    try {
        const url = new URL(referrer);
        const params = url.searchParams;
        
        // Common search query parameters used by different search engines
        const queryParams = ['q', 'query', 'search', 'searchTerm', 'text', 'p', 'wd', 'keyword'];
        
        for (const param of queryParams) {
            const query = params.get(param);
            if (query) {
                return decodeURIComponent(query).trim();
            }
        }
        
        // Special handling for specific search engines
        if (url.hostname.includes('google')) {
            return params.get('q') || params.get('query') || null;
        } else if (url.hostname.includes('bing')) {
            return params.get('q') || null;
        } else if (url.hostname.includes('yahoo')) {
            return params.get('p') || params.get('q') || null;
        } else if (url.hostname.includes('duckduckgo')) {
            return params.get('q') || null;
        } else if (url.hostname.includes('baidu')) {
            return params.get('wd') || params.get('word') || null;
        } else if (url.hostname.includes('yandex')) {
            return params.get('text') || params.get('query') || null;
        }
    } catch (e) {
        // Invalid URL
        return null;
    }
    
    return null;
}

// Get search queries from referrers
export function getSearchQueries(siteId: string, startDate: string, endDate: string, limit: number = 10): Array<{ query: string; count: number }> {
    // Get all referrers from search engines
    const stmt = db.prepare(`
        SELECT referrer, COUNT(*) as count
        FROM events
        WHERE site_id = ?
        AND timestamp BETWEEN ? AND ?
        AND referrer IS NOT NULL
        AND referrer != ''
        AND (
            referrer LIKE '%google.%' OR
            referrer LIKE '%bing.%' OR
            referrer LIKE '%yahoo.%' OR
            referrer LIKE '%duckduckgo.%' OR
            referrer LIKE '%baidu.%' OR
            referrer LIKE '%yandex.%' OR
            referrer LIKE '%ask.com%' OR
            referrer LIKE '%aol.%'
        )
        GROUP BY referrer
        ORDER BY count DESC
    `);
    
    const referrers = stmt.all(siteId, startDate, endDate) as Array<{ referrer: string; count: number }>;
    
    // Extract and aggregate search queries
    const queryMap = new Map<string, number>();
    
    for (const ref of referrers) {
        const query = extractSearchQuery(ref.referrer);
        if (query) {
            queryMap.set(query, (queryMap.get(query) || 0) + ref.count);
        }
    }
    
    // Convert to array and sort by count
    const queries = Array.from(queryMap.entries())
        .map(([query, count]) => ({ query, count }))
        .sort((a, b) => b.count - a.count)
        .slice(0, limit);
    
    return queries;
}

// Categorize traffic sources
function categorizeTrafficSource(referrer: string | null): { category: string; icon: string } {
    if (!referrer || referrer === '' || referrer === 'direct') {
        return { category: 'Direct', icon: '🔗' };
    }
    
    const url = referrer.toLowerCase();
    
    // Email - check first to catch mail providers before they match search engines
    if (url.includes('mail.') || url.includes('outlook.') || url.includes('gmail.') || 
        url.includes('/mail') || url.includes('webmail') || url.includes('email')) {
        return { category: 'Email', icon: '📧' };
    }
    
    // Search engines
    const searchEngines = ['google', 'bing', 'yahoo', 'duckduckgo', 'baidu', 'yandex', 'ask.com', 'aol'];
    if (searchEngines.some(engine => url.includes(engine))) {
        return { category: 'Search Engines', icon: '🔍' };
    }
    
    // Social media
    const socialMedia = ['facebook', 'twitter', 'linkedin', 'instagram', 'youtube', 'reddit', 
                        'pinterest', 'tumblr', 'snapchat', 'tiktok', 'whatsapp', 'telegram',
                        'discord', 'slack', 'medium', 'quora'];
    if (socialMedia.some(social => url.includes(social))) {
        return { category: 'Social Media', icon: '📱' };
    }
    
    // All others are referral sites
    return { category: 'Referral Sites', icon: '🌐' };
}

// Get traffic sources for a site
export function getTrafficSources(siteId: string, startDate: string, endDate: string): TrafficSource[] {
    // Get all referrers with counts
    const stmt = db.prepare(`
        SELECT 
            COALESCE(referrer, 'direct') as referrer,
            COUNT(*) as count
        FROM events
        WHERE site_id = ?
        AND timestamp BETWEEN ? AND ?
        GROUP BY referrer
    `);
    
    const referrers = stmt.all(siteId, startDate, endDate) as Array<{ referrer: string; count: number }>;
    
    // Categorize and aggregate
    const sourceMap = new Map<string, { icon: string; count: number }>();
    let totalCount = 0;
    
    for (const ref of referrers) {
        const { category, icon } = categorizeTrafficSource(ref.referrer);
        const existing = sourceMap.get(category) || { icon, count: 0 };
        existing.count += ref.count;
        sourceMap.set(category, existing);
        totalCount += ref.count;
    }
    
    // Convert to array and calculate percentages
    const sources: TrafficSource[] = [];
    for (const [source, data] of sourceMap.entries()) {
        sources.push({
            source,
            icon: data.icon,
            count: data.count,
            percentage: totalCount > 0 ? Math.round((data.count / totalCount) * 100) : 0
        });
    }
    
    // Sort by count descending
    sources.sort((a, b) => b.count - a.count);
    
    return sources;
}

// Get recent visitors for a site
export function getRecentVisitors(siteId: string, limit: number = 20, startDate?: string, endDate?: string): RecentVisitor[] {
    let query = `
        SELECT 
            ip_hash,
            page_url,
            timestamp,
            country,
            country_code,
            city,
            region
        FROM events
        WHERE site_id = ?
    `;
    
    const params: any[] = [siteId];
    
    if (startDate && endDate) {
        query += ` AND timestamp BETWEEN ? AND ?`;
        params.push(startDate, endDate);
    }
    
    query += ` ORDER BY timestamp DESC LIMIT ?`;
    params.push(limit);
    
    const stmt = db.prepare(query);
    
    return stmt.all(...params) as RecentVisitor[];
}

// Get dashboard stats for a site
export function getDashboardStats(siteId: string, startDate?: string, endDate?: string): DashboardStats {
    const now = new Date();
    const end = endDate || now.toISOString();
    const start = startDate || new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000).toISOString(); // 30 days ago
    
    // Get current period stats
    const currentStats = getPeriodStats(siteId, start, end);
    
    // Get previous period for trend calculation
    const prevStart = new Date(new Date(start).getTime() - (new Date(end).getTime() - new Date(start).getTime())).toISOString();
    const prevEnd = start;
    const prevStats = getPeriodStats(siteId, prevStart, prevEnd);
    
    // Calculate trends
    const visitorsTrend = prevStats.visitors > 0 
        ? ((currentStats.visitors - prevStats.visitors) / prevStats.visitors) * 100 
        : 0;
    const pageViewsTrend = prevStats.pageViews > 0 
        ? ((currentStats.pageViews - prevStats.pageViews) / prevStats.pageViews) * 100 
        : 0;
    
    // Get realtime visitors (last 5 minutes)
    const realtimeStmt = db.prepare(`
        SELECT COUNT(DISTINCT ip_hash) as count 
        FROM events 
        WHERE site_id = ? 
        AND timestamp > datetime('now', '-5 minutes')
    `);
    const realtime = realtimeStmt.get(siteId) as { count: number };
    
    // Get top pages
    const topPagesStmt = db.prepare(`
        SELECT page_url as url, COUNT(*) as views
        FROM events
        WHERE site_id = ?
        AND timestamp BETWEEN ? AND ?
        GROUP BY page_url
        ORDER BY views DESC
        LIMIT 10
    `);
    const topPages = topPagesStmt.all(siteId, start, end) as Array<{ url: string; views: number }>;
    
    // Get top referrers
    const topReferrersStmt = db.prepare(`
        SELECT referrer, COUNT(*) as count
        FROM events
        WHERE site_id = ?
        AND timestamp BETWEEN ? AND ?
        AND referrer IS NOT NULL
        AND referrer != ''
        GROUP BY referrer
        ORDER BY count DESC
        LIMIT 10
    `);
    const topReferrers = topReferrersStmt.all(siteId, start, end) as Array<{ referrer: string; count: number }>;
    
    // Get top countries
    const topCountriesStmt = db.prepare(`
        SELECT country, country_code as countryCode, COUNT(*) as count
        FROM events
        WHERE site_id = ?
        AND timestamp BETWEEN ? AND ?
        AND country IS NOT NULL
        GROUP BY country, country_code
        ORDER BY count DESC
        LIMIT 10
    `);
    const topCountries = topCountriesStmt.all(siteId, start, end) as Array<{ country: string; countryCode: string; count: number }>;
    
    // Get top cities
    const topCitiesStmt = db.prepare(`
        SELECT city, country, COUNT(*) as count
        FROM events
        WHERE site_id = ?
        AND timestamp BETWEEN ? AND ?
        AND city IS NOT NULL
        GROUP BY city, country
        ORDER BY count DESC
        LIMIT 10
    `);
    const topCities = topCitiesStmt.all(siteId, start, end) as Array<{ city: string; country: string; count: number }>;
    
    // Get top locales (languages)
    const topLocalesStmt = db.prepare(`
        SELECT 
            language as locale,
            COUNT(*) as count
        FROM events
        WHERE site_id = ?
        AND timestamp BETWEEN ? AND ?
        AND language IS NOT NULL
        GROUP BY language
        ORDER BY count DESC
        LIMIT 10
    `);
    const localesResult = topLocalesStmt.all(siteId, start, end) as Array<{ locale: string; count: number }>;
    
    // Calculate total for percentage
    const totalWithLocale = localesResult.reduce((sum, l) => sum + l.count, 0);
    
    // Parse locale codes and add display information
    const topLocales = localesResult.map(item => {
        const [langCode, countryCode] = item.locale.split('-');
        const languageNames: { [key: string]: string } = {
            'en': 'English',
            'es': 'Spanish', 
            'fr': 'French',
            'de': 'German',
            'it': 'Italian',
            'pt': 'Portuguese',
            'ru': 'Russian',
            'ja': 'Japanese',
            'ko': 'Korean',
            'zh': 'Chinese',
            'ar': 'Arabic',
            'hi': 'Hindi',
            'nl': 'Dutch',
            'sv': 'Swedish',
            'pl': 'Polish',
            'tr': 'Turkish',
            'vi': 'Vietnamese',
            'th': 'Thai',
            'id': 'Indonesian',
            'ms': 'Malay'
        };
        
        const countryNames: { [key: string]: string } = {
            'US': 'United States',
            'GB': 'United Kingdom',
            'CA': 'Canada',
            'AU': 'Australia',
            'DE': 'Germany',
            'FR': 'France',
            'ES': 'Spain',
            'IT': 'Italy',
            'BR': 'Brazil',
            'PT': 'Portugal',
            'RU': 'Russia',
            'JP': 'Japan',
            'KR': 'South Korea',
            'CN': 'China',
            'TW': 'Taiwan',
            'HK': 'Hong Kong',
            'IN': 'India',
            'MX': 'Mexico',
            'AR': 'Argentina',
            'NL': 'Netherlands',
            'SE': 'Sweden',
            'PL': 'Poland',
            'TR': 'Turkey',
            'VN': 'Vietnam',
            'TH': 'Thailand',
            'ID': 'Indonesia',
            'MY': 'Malaysia',
            'SG': 'Singapore'
        };
        
        return {
            locale: item.locale,
            language: languageNames[langCode] || langCode.toUpperCase(),
            country: countryNames[countryCode] || countryCode || '',
            count: item.count,
            percentage: totalWithLocale > 0 ? Math.round((item.count / totalWithLocale) * 100) : 0
        };
    });
    
    // Get recent visitors
    const recentVisitors = getRecentVisitors(siteId, 10, start, end);
    
    // Get traffic sources
    const trafficSources = getTrafficSources(siteId, start, end);
    
    // Get search queries
    const searchQueries = getSearchQueries(siteId, start, end, 6);
    
    return {
        visitors: currentStats.visitors,
        pageViews: currentStats.pageViews,
        avgSessionDuration: currentStats.avgSessionDuration,
        bounceRate: currentStats.bounceRate,
        topPages,
        topReferrers,
        topCountries,
        topCities,
        topLocales,
        trafficSources,
        searchQueries,
        realtimeVisitors: realtime?.count || 0,
        visitorsTrend,
        pageViewsTrend,
        recentVisitors
    };
}

// Get time series data for charts
export function getTimeSeriesData(siteId: string, startDate?: string, endDate?: string): TimeSeriesData {
    // If dates not provided, use last 30 days as default
    const now = new Date();
    const end = endDate || now.toISOString();
    const start = startDate || new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000).toISOString();
    
    const stmt = db.prepare(`
        SELECT 
            DATE(timestamp) as date,
            COUNT(DISTINCT ip_hash) as visitors,
            COUNT(*) as pageViews
        FROM events
        WHERE site_id = ?
        AND timestamp BETWEEN ? AND ?
        GROUP BY DATE(timestamp)
        ORDER BY date ASC
    `);
    
    const results = stmt.all(siteId, start, end) as Array<{
        date: string;
        visitors: number;
        pageViews: number;
    }>;
    
    // Fill in missing dates with zeros
    const labels: string[] = [];
    const visitors: number[] = [];
    const pageViews: number[] = [];
    
    const startDateObj = new Date(start);
    const endDateObj = new Date(end);
    
    // Calculate number of days
    const days = Math.ceil((endDateObj.getTime() - startDateObj.getTime()) / (24 * 60 * 60 * 1000)) + 1;
    
    for (let i = 0; i < days; i++) {
        const currentDate = new Date(startDateObj);
        currentDate.setDate(startDateObj.getDate() + i);
        const dateStr = currentDate.toISOString().split('T')[0];
        
        // Stop if we've passed the end date
        if (currentDate > endDateObj) break;
        
        const dayData = results.find(r => r.date === dateStr);
        
        labels.push(currentDate.toLocaleDateString('en-US', { month: 'short', day: 'numeric' }));
        visitors.push(dayData?.visitors || 0);
        pageViews.push(dayData?.pageViews || 0);
    }
    
    return { labels, visitors, pageViews };
}

// Helper function to get stats for a period
function getPeriodStats(siteId: string, start: string, end: string) {
    const stmt = db.prepare(`
        SELECT 
            COUNT(DISTINCT ip_hash) as visitors,
            COUNT(*) as pageViews,
            0 as avgSessionDuration,
            COALESCE(
                (
                    SELECT COUNT(*) * 100.0 / NULLIF(COUNT(DISTINCT ip_hash), 0)
                    FROM (
                        SELECT ip_hash
                        FROM events
                        WHERE site_id = ?
                        AND timestamp BETWEEN ? AND ?
                        GROUP BY ip_hash
                        HAVING COUNT(*) = 1
                    )
                ), 0
            ) as bounceRate
        FROM events e1
        WHERE site_id = ?
        AND timestamp BETWEEN ? AND ?
    `);
    
    const result = stmt.get(siteId, start, end, siteId, start, end) as {
        visitors: number;
        pageViews: number;
        avgSessionDuration: number | null;
        bounceRate: number | null;
    };
    
    return {
        visitors: result?.visitors || 0,
        pageViews: result?.pageViews || 0,
        avgSessionDuration: result?.avgSessionDuration || 0,
        bounceRate: result?.bounceRate || 0
    };
}

// API route handlers
export const statsRoutes = {
    // Helper method for WebSocket broadcasts
    async getStatsData(siteId: string, startDate: string, endDate: string) {
        return getDashboardStats(siteId, startDate, endDate);
    },
    
    // GET /api/stats/:siteId
    async getStats(req: Request, siteId: string) {
        try {
            console.log('[Stats API] Getting stats for site:', siteId);
            const url = new URL(req.url);
            const startDate = url.searchParams.get('start');
            const endDate = url.searchParams.get('end');
            console.log('[Stats API] Date range:', { start: startDate, end: endDate });
            
            const stats = getDashboardStats(siteId, startDate || undefined, endDate || undefined);
            console.log('[Stats API] Stats retrieved successfully');
            
            return new Response(JSON.stringify(stats), {
                headers: { 
                    'Content-Type': 'application/json',
                    'Access-Control-Allow-Origin': '*'
                }
            });
        } catch (error) {
            console.error('[Stats API] Error fetching stats:', error);
            console.error('[Stats API] Error stack:', error.stack);
            console.error('[Stats API] Error details:', {
                siteId,
                url: req.url,
                message: error.message
            });
            
            // Return more detailed error in development
            const errorResponse = {
                error: 'Failed to fetch stats',
                message: error.message,
                siteId: siteId
            };
            
            return new Response(JSON.stringify(errorResponse), {
                status: 500,
                headers: { 'Content-Type': 'application/json' }
            });
        }
    },
    
    // GET /api/stats/:siteId/timeseries
    async getTimeSeries(req: Request, siteId: string) {
        try {
            const url = new URL(req.url);
            const startDate = url.searchParams.get('start');
            const endDate = url.searchParams.get('end');
            
            const data = getTimeSeriesData(siteId, startDate || undefined, endDate || undefined);
            
            return new Response(JSON.stringify(data), {
                headers: { 
                    'Content-Type': 'application/json',
                    'Access-Control-Allow-Origin': '*'
                }
            });
        } catch (error) {
            console.error('Error fetching time series:', error);
            return new Response(JSON.stringify({ error: 'Failed to fetch time series' }), {
                status: 500,
                headers: { 'Content-Type': 'application/json' }
            });
        }
    },
    
    // GET /api/stats/:siteId/realtime
    async getRealtime(req: Request, siteId: string) {
        try {
            const stmt = db.prepare(`
                SELECT COUNT(DISTINCT ip_hash) as visitors 
                FROM events 
                WHERE site_id = ? 
                AND timestamp > datetime('now', '-5 minutes')
            `);
            const result = stmt.get(siteId) as { visitors: number };
            
            return new Response(JSON.stringify({ visitors: result?.visitors || 0 }), {
                headers: { 
                    'Content-Type': 'application/json',
                    'Access-Control-Allow-Origin': '*'
                }
            });
        } catch (error) {
            console.error('Error fetching realtime visitors:', error);
            return new Response(JSON.stringify({ error: 'Failed to fetch realtime visitors' }), {
                status: 500,
                headers: { 'Content-Type': 'application/json' }
            });
        }
    },
    
    // GET /api/stats/:siteId/recent-visitors
    async getRecentVisitorsEndpoint(req: Request, siteId: string) {
        try {
            const url = new URL(req.url);
            const limit = parseInt(url.searchParams.get('limit') || '20');
            
            const visitors = getRecentVisitors(siteId, limit);
            
            return new Response(JSON.stringify(visitors), {
                headers: { 
                    'Content-Type': 'application/json',
                    'Access-Control-Allow-Origin': '*'
                }
            });
        } catch (error) {
            console.error('Error fetching recent visitors:', error);
            return new Response(JSON.stringify({ error: 'Failed to fetch recent visitors' }), {
                status: 500,
                headers: { 'Content-Type': 'application/json' }
            });
        }
    }
};