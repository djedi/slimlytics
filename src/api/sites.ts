import { Database } from 'bun:sqlite';
import { nanoid } from 'nanoid';

// Initialize database - reuse existing schema from init.js
const db = new Database('data/analytics.db', { create: true });

export interface Site {
    id: string;
    name: string;
    domain: string;
    created_at: string;
}

// Helper function to extract domain from URL
function extractDomain(url: string): string {
    // Remove protocol if present
    let domain = url.replace(/^https?:\/\//, '');
    // Remove www. if present
    domain = domain.replace(/^www\./, '');
    // Remove path, query params, and hash
    domain = domain.split('/')[0];
    domain = domain.split('?')[0];
    domain = domain.split('#')[0];
    // Remove port if present
    domain = domain.split(':')[0];
    
    return domain.toLowerCase();
}

// Validate domain format
function isValidDomain(domain: string): boolean {
    // Basic domain validation regex
    const domainRegex = /^([a-z0-9]+(-[a-z0-9]+)*\.)+[a-z]{2,}$|^localhost$|^([a-z0-9]+(-[a-z0-9]+)*)$/i;
    return domainRegex.test(domain);
}

// Get all sites
export function getAllSites(): Site[] {
    const stmt = db.prepare('SELECT * FROM sites ORDER BY created_at DESC');
    return stmt.all() as Site[];
}

// Get site by ID
export function getSiteById(id: string): Site | null {
    const stmt = db.prepare('SELECT * FROM sites WHERE id = ?');
    return stmt.get(id) as Site | null;
}

// Get site by domain
export function getSiteByDomain(domain: string): Site | null {
    const stmt = db.prepare('SELECT * FROM sites WHERE domain = ?');
    return stmt.get(domain) as Site | null;
}

// Create a new site
export function createSite(name: string, domain: string): Site {
    // Extract and validate domain
    const cleanDomain = extractDomain(domain);
    
    if (!isValidDomain(cleanDomain)) {
        throw new Error('Invalid domain format');
    }
    
    // Check if domain already exists
    const existing = getSiteByDomain(cleanDomain);
    if (existing) {
        throw new Error('A site with this domain already exists');
    }
    
    const id = nanoid(10); // Generate a short unique ID
    const stmt = db.prepare(`
        INSERT INTO sites (id, name, domain) 
        VALUES (?, ?, ?)
    `);
    
    stmt.run(id, name, cleanDomain);
    
    return getSiteById(id)!;
}

// Update a site
export function updateSite(id: string, name: string, domain: string): Site | null {
    // Extract and validate domain
    const cleanDomain = extractDomain(domain);
    
    if (!isValidDomain(cleanDomain)) {
        throw new Error('Invalid domain format');
    }
    
    // Check if domain already exists for a different site
    const existing = getSiteByDomain(cleanDomain);
    if (existing && existing.id !== id) {
        throw new Error('A site with this domain already exists');
    }
    
    const stmt = db.prepare(`
        UPDATE sites 
        SET name = ?, domain = ?
        WHERE id = ?
    `);
    
    const result = stmt.run(name, cleanDomain, id);
    
    if (result.changes > 0) {
        return getSiteById(id);
    }
    
    return null;
}

// Delete a site
export function deleteSite(id: string): boolean {
    // Also delete related events and stats
    db.run('DELETE FROM events WHERE site_id = ?', [id]);
    db.run('DELETE FROM daily_stats WHERE site_id = ?', [id]);
    
    const stmt = db.prepare('DELETE FROM sites WHERE id = ?');
    const result = stmt.run(id);
    return result.changes > 0;
}

// API route handlers
export const sitesRoutes = {
    // GET /api/sites
    async getAllSites(req: Request) {
        try {
            const sites = getAllSites();
            return new Response(JSON.stringify(sites), {
                headers: { 
                    'Content-Type': 'application/json',
                    'Access-Control-Allow-Origin': '*'
                }
            });
        } catch (error) {
            console.error('Error fetching sites:', error);
            return new Response(JSON.stringify({ error: 'Failed to fetch sites' }), {
                status: 500,
                headers: { 'Content-Type': 'application/json' }
            });
        }
    },
    
    // POST /api/sites
    async createSite(req: Request) {
        try {
            const { name, url } = await req.json();
            
            if (!name || !url) {
                return new Response(JSON.stringify({ error: 'Name and URL are required' }), {
                    status: 400,
                    headers: { 'Content-Type': 'application/json' }
                });
            }
            
            try {
                const site = createSite(name, url);
                return new Response(JSON.stringify(site), {
                    status: 201,
                    headers: { 
                        'Content-Type': 'application/json',
                        'Access-Control-Allow-Origin': '*'
                    }
                });
            } catch (err: any) {
                return new Response(JSON.stringify({ error: err.message }), {
                    status: 400,
                    headers: { 'Content-Type': 'application/json' }
                });
            }
        } catch (error) {
            console.error('Error creating site:', error);
            return new Response(JSON.stringify({ error: 'Failed to create site' }), {
                status: 500,
                headers: { 'Content-Type': 'application/json' }
            });
        }
    },
    
    // PUT /api/sites/:id
    async updateSite(req: Request, id: string) {
        try {
            const { name, url } = await req.json();
            
            if (!name || !url) {
                return new Response(JSON.stringify({ error: 'Name and URL are required' }), {
                    status: 400,
                    headers: { 'Content-Type': 'application/json' }
                });
            }
            
            try {
                const site = updateSite(id, name, url);
                
                if (!site) {
                    return new Response(JSON.stringify({ error: 'Site not found' }), {
                        status: 404,
                        headers: { 'Content-Type': 'application/json' }
                    });
                }
                
                return new Response(JSON.stringify(site), {
                    headers: { 
                        'Content-Type': 'application/json',
                        'Access-Control-Allow-Origin': '*'
                    }
                });
            } catch (err: any) {
                return new Response(JSON.stringify({ error: err.message }), {
                    status: 400,
                    headers: { 'Content-Type': 'application/json' }
                });
            }
        } catch (error) {
            console.error('Error updating site:', error);
            return new Response(JSON.stringify({ error: 'Failed to update site' }), {
                status: 500,
                headers: { 'Content-Type': 'application/json' }
            });
        }
    },
    
    // DELETE /api/sites/:id
    async deleteSite(req: Request, id: string) {
        try {
            const success = deleteSite(id);
            
            if (!success) {
                return new Response(JSON.stringify({ error: 'Site not found' }), {
                    status: 404,
                    headers: { 'Content-Type': 'application/json' }
                });
            }
            
            return new Response(JSON.stringify({ success: true }), {
                headers: { 
                    'Content-Type': 'application/json',
                    'Access-Control-Allow-Origin': '*'
                }
            });
        } catch (error) {
            console.error('Error deleting site:', error);
            return new Response(JSON.stringify({ error: 'Failed to delete site' }), {
                status: 500,
                headers: { 'Content-Type': 'application/json' }
            });
        }
    }
};