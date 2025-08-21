export function trackEvent(db, eventData) {
  const stmt = db.prepare(`
    INSERT INTO events (
      site_id, page_url, referrer, user_agent, ip_hash, 
      screen_resolution, language, timestamp,
      country, country_code, region, city, 
      latitude, longitude, timezone, asn, asn_org,
      visitor_id, session_id
    )
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
    eventData.asn_org || null,
    eventData.visitor_id || null,
    eventData.session_id || null
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
    SELECT COUNT(DISTINCT visitor_id) as count 
    FROM sessions 
    WHERE site_id = ? 
    AND started_at BETWEEN ? AND ?
  `).get(siteId, startDate || '1970-01-01', endDate || '2100-01-01');

  const totalSessions = db.prepare(`
    SELECT COUNT(*) as count 
    FROM sessions 
    WHERE site_id = ? 
    AND started_at BETWEEN ? AND ?
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
    SELECT s.referrer, COUNT(DISTINCT s.session_id) as count 
    FROM sessions s
    WHERE s.site_id = ? 
    AND s.referrer IS NOT NULL 
    AND s.referrer != ''
    AND s.started_at BETWEEN ? AND ?
    GROUP BY s.referrer 
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
    SELECT s.country, s.country_code, COUNT(DISTINCT s.session_id) as count
    FROM sessions s
    WHERE s.site_id = ?
    AND s.country IS NOT NULL
    AND s.started_at BETWEEN ? AND ?
    GROUP BY s.country, s.country_code
    ORDER BY count DESC
    LIMIT 10
  `).all(siteId, startDate || '1970-01-01', endDate || '2100-01-01');

  const topCities = db.prepare(`
    SELECT s.city, s.country, COUNT(DISTINCT s.session_id) as count
    FROM sessions s
    WHERE s.site_id = ?
    AND s.city IS NOT NULL
    AND s.started_at BETWEEN ? AND ?
    GROUP BY s.city, s.country
    ORDER BY count DESC
    LIMIT 10
  `).all(siteId, startDate || '1970-01-01', endDate || '2100-01-01');

  const trafficSources = db.prepare(`
    SELECT traffic_source, COUNT(DISTINCT session_id) as count
    FROM sessions
    WHERE site_id = ?
    AND started_at BETWEEN ? AND ?
    GROUP BY traffic_source
    ORDER BY count DESC
  `).all(siteId, startDate || '1970-01-01', endDate || '2100-01-01');

  const topLanguages = db.prepare(`
    SELECT language, COUNT(DISTINCT session_id) as count
    FROM sessions
    WHERE site_id = ?
    AND language IS NOT NULL
    AND started_at BETWEEN ? AND ?
    GROUP BY language
    ORDER BY count DESC
    LIMIT 10
  `).all(siteId, startDate || '1970-01-01', endDate || '2100-01-01');

  return {
    pageViews: pageViews.count,
    uniqueVisitors: uniqueVisitors.count,
    totalSessions: totalSessions.count,
    topPages,
    topReferrers,
    browserStats,
    topCountries,
    topCities,
    trafficSources,
    topLanguages
  };
}