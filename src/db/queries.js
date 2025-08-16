export function trackEvent(db, eventData) {
  const stmt = db.prepare(`
    INSERT INTO events (
      site_id, page_url, referrer, user_agent, ip_hash, 
      screen_resolution, language, timestamp,
      country, country_code, region, city, 
      latitude, longitude, timezone, asn, asn_org
    )
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
  `);
  
  stmt.run(
    eventData.site_id,
    eventData.page_url,
    eventData.referrer,
    eventData.user_agent,
    eventData.ip_hash,
    eventData.screen_resolution,
    eventData.language,
    eventData.timestamp,
    eventData.country || null,
    eventData.country_code || null,
    eventData.region || null,
    eventData.city || null,
    eventData.latitude || null,
    eventData.longitude || null,
    eventData.timezone || null,
    eventData.asn || null,
    eventData.asn_org || null
  );
}

export function getStats(db, siteId, startDate, endDate) {
  const pageViews = db.prepare(`
    SELECT COUNT(*) as count 
    FROM events 
    WHERE site_id = ? 
    AND timestamp BETWEEN ? AND ?
  `).get(siteId, startDate || '1970-01-01', endDate || '2100-01-01');

  const uniqueVisitors = db.prepare(`
    SELECT COUNT(DISTINCT ip_hash) as count 
    FROM events 
    WHERE site_id = ? 
    AND timestamp BETWEEN ? AND ?
  `).get(siteId, startDate || '1970-01-01', endDate || '2100-01-01');

  const topPages = db.prepare(`
    SELECT page_url, COUNT(*) as count 
    FROM events 
    WHERE site_id = ? 
    AND timestamp BETWEEN ? AND ?
    GROUP BY page_url 
    ORDER BY count DESC 
    LIMIT 10
  `).all(siteId, startDate || '1970-01-01', endDate || '2100-01-01');

  const topReferrers = db.prepare(`
    SELECT referrer, COUNT(*) as count 
    FROM events 
    WHERE site_id = ? 
    AND referrer IS NOT NULL 
    AND referrer != ''
    AND timestamp BETWEEN ? AND ?
    GROUP BY referrer 
    ORDER BY count DESC 
    LIMIT 10
  `).all(siteId, startDate || '1970-01-01', endDate || '2100-01-01');

  const browserStats = db.prepare(`
    SELECT 
      CASE 
        WHEN user_agent LIKE '%Chrome%' THEN 'Chrome'
        WHEN user_agent LIKE '%Firefox%' THEN 'Firefox'
        WHEN user_agent LIKE '%Safari%' THEN 'Safari'
        WHEN user_agent LIKE '%Edge%' THEN 'Edge'
        ELSE 'Other'
      END as browser,
      COUNT(*) as count
    FROM events
    WHERE site_id = ?
    AND timestamp BETWEEN ? AND ?
    GROUP BY browser
    ORDER BY count DESC
  `).all(siteId, startDate || '1970-01-01', endDate || '2100-01-01');

  const topCountries = db.prepare(`
    SELECT country, country_code, COUNT(*) as count
    FROM events
    WHERE site_id = ?
    AND country IS NOT NULL
    AND timestamp BETWEEN ? AND ?
    GROUP BY country, country_code
    ORDER BY count DESC
    LIMIT 10
  `).all(siteId, startDate || '1970-01-01', endDate || '2100-01-01');

  const topCities = db.prepare(`
    SELECT city, country, COUNT(*) as count
    FROM events
    WHERE site_id = ?
    AND city IS NOT NULL
    AND timestamp BETWEEN ? AND ?
    GROUP BY city, country
    ORDER BY count DESC
    LIMIT 10
  `).all(siteId, startDate || '1970-01-01', endDate || '2100-01-01');

  return {
    pageViews: pageViews.count,
    uniqueVisitors: uniqueVisitors.count,
    topPages,
    topReferrers,
    browserStats,
    topCountries,
    topCities
  };
}