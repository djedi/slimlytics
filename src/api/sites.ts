import { Database } from 'bun:sqlite';
import { nanoid } from 'nanoid';

// Initialize database
const db = new Database('data/analytics.db', { create: true });

// Create sites table if it doesn't exist
db.run(`
    CREATE TABLE IF NOT EXISTS sites (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        url TEXT NOT NULL,
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
        updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
    )
`);

// Create indexes
db.run(`CREATE INDEX IF NOT EXISTS idx_sites_created_at ON sites(created_at)`);

export interface Site {
    id: string;
    name: string;
    url: string;
    created_at: string;
    updated_at: string;
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

// Create a new site
export function createSite(name: string, url: string): Site {
    const id = nanoid(10); // Generate a short unique ID
    const stmt = db.prepare(`
        INSERT INTO sites (id, name, url) 
        VALUES (?, ?, ?)
    `);
    
    stmt.run(id, name, url);
    
    return getSiteById(id)!;
}

// Update a site
export function updateSite(id: string, name: string, url: string): Site | null {
    const stmt = db.prepare(`
        UPDATE sites 
        SET name = ?, url = ?, updated_at = CURRENT_TIMESTAMP 
        WHERE id = ?
    `);
    
    const result = stmt.run(name, url, id);
    
    if (result.changes > 0) {
        return getSiteById(id);
    }
    
    return null;
}

// Delete a site
export function deleteSite(id: string): boolean {
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
                headers: { 'Content-Type': 'application/json' }
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
            
            const site = createSite(name, url);
            return new Response(JSON.stringify(site), {
                status: 201,
                headers: { 'Content-Type': 'application/json' }
            });
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
            
            const site = updateSite(id, name, url);
            
            if (!site) {
                return new Response(JSON.stringify({ error: 'Site not found' }), {
                    status: 404,
                    headers: { 'Content-Type': 'application/json' }
                });
            }
            
            return new Response(JSON.stringify(site), {
                headers: { 'Content-Type': 'application/json' }
            });
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
                headers: { 'Content-Type': 'application/json' }
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