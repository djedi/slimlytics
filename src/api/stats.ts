import { Database } from 'bun:sqlite';

const db = new Database('data/analytics.db', { create: true });

export interface DashboardStats {
    visitors: number;
    pageViews: number;
    avgSessionDuration: number;
    bounceRate: number;
    topPages: Array<{ url: string; views: number }>;
    topReferrers: Array<{ referrer: string; count: number }>;
    realtimeVisitors: number;
    visitorsTrend: number; // percentage change
    pageViewsTrend: number; // percentage change
}

export interface TimeSeriesData {
    labels: string[];
    visitors: number[];
    pageViews: number[];
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
    
    return {
        visitors: currentStats.visitors,
        pageViews: currentStats.pageViews,
        avgSessionDuration: currentStats.avgSessionDuration,
        bounceRate: currentStats.bounceRate,
        topPages,
        topReferrers,
        realtimeVisitors: realtime?.count || 0,
        visitorsTrend,
        pageViewsTrend
    };
}

// Get time series data for charts
export function getTimeSeriesData(siteId: string, days: number = 30): TimeSeriesData {
    const stmt = db.prepare(`
        SELECT 
            DATE(timestamp) as date,
            COUNT(DISTINCT ip_hash) as visitors,
            COUNT(*) as pageViews
        FROM events
        WHERE site_id = ?
        AND timestamp > datetime('now', '-' || ? || ' days')
        GROUP BY DATE(timestamp)
        ORDER BY date ASC
    `);
    
    const results = stmt.all(siteId, days) as Array<{
        date: string;
        visitors: number;
        pageViews: number;
    }>;
    
    // Fill in missing dates with zeros
    const labels: string[] = [];
    const visitors: number[] = [];
    const pageViews: number[] = [];
    
    const startDate = new Date();
    startDate.setDate(startDate.getDate() - days);
    
    for (let i = 0; i < days; i++) {
        const currentDate = new Date(startDate);
        currentDate.setDate(startDate.getDate() + i);
        const dateStr = currentDate.toISOString().split('T')[0];
        
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
    // GET /api/stats/:siteId
    async getStats(req: Request, siteId: string) {
        try {
            const url = new URL(req.url);
            const startDate = url.searchParams.get('start');
            const endDate = url.searchParams.get('end');
            
            const stats = getDashboardStats(siteId, startDate || undefined, endDate || undefined);
            
            return new Response(JSON.stringify(stats), {
                headers: { 
                    'Content-Type': 'application/json',
                    'Access-Control-Allow-Origin': '*'
                }
            });
        } catch (error) {
            console.error('Error fetching stats:', error);
            return new Response(JSON.stringify({ error: 'Failed to fetch stats' }), {
                status: 500,
                headers: { 'Content-Type': 'application/json' }
            });
        }
    },
    
    // GET /api/stats/:siteId/timeseries
    async getTimeSeries(req: Request, siteId: string) {
        try {
            const url = new URL(req.url);
            const days = parseInt(url.searchParams.get('days') || '30');
            
            const data = getTimeSeriesData(siteId, days);
            
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
    }
};