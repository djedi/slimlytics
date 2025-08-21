export function setupSimplifiedTrackingRoutes(app) {
  // Simplified tracking script endpoint (like Clicky's /js)
  app.get('/sa.js', (c) => {
    const script = `
(function() {
  'use strict';
  
  // Get site ID from script tag - use document.currentScript or find by data-id
  var currentScript = document.currentScript;
  if (!currentScript) {
    // Fallback: find any script with data-id attribute
    var scripts = document.getElementsByTagName('script');
    for (var i = 0; i < scripts.length; i++) {
      if (scripts[i].getAttribute('data-id')) {
        currentScript = scripts[i];
        break;
      }
    }
  }
  var SITE_ID = currentScript ? currentScript.getAttribute('data-id') : null;
  
  if (!SITE_ID) {
    console.error('Slimlytics: No data-id attribute found on script tag');
    return;
  }
  
  // Helper to get cookie value
  function getCookie(name) {
    var value = '; ' + document.cookie;
    var parts = value.split('; ' + name + '=');
    if (parts.length === 2) return parts.pop().split(';').shift();
  }
  
  // Helper to set cookie
  function setCookie(name, value, days) {
    var expires = '';
    if (days) {
      var date = new Date();
      date.setTime(date.getTime() + (days * 24 * 60 * 60 * 1000));
      expires = '; expires=' + date.toUTCString();
    }
    document.cookie = name + '=' + value + expires + '; path=/; SameSite=Lax';
  }
  
  // Generate or get visitor ID
  function getVisitorId() {
    var visitorId = getCookie('_sa_vid');
    if (!visitorId) {
      visitorId = Math.random().toString(36).substring(2) + Date.now().toString(36);
      setCookie('_sa_vid', visitorId, 365);
    }
    return visitorId;
  }
  
  // Generate session ID
  function getSessionId() {
    var sessionId = sessionStorage.getItem('_sa_sid');
    if (!sessionId) {
      sessionId = Math.random().toString(36).substring(2) + Date.now().toString(36);
      sessionStorage.setItem('_sa_sid', sessionId);
    }
    return sessionId;
  }
  
  // Send tracking event
  function track(eventType, eventData) {
    var data = {
      siteId: SITE_ID,
      visitorId: getVisitorId(),
      sessionId: getSessionId(),
      eventType: eventType || 'pageview',
      eventData: eventData || {},
      url: window.location.href,
      referrer: document.referrer,
      userAgent: navigator.userAgent,
      language: navigator.language,
      screenWidth: window.screen.width,
      screenHeight: window.screen.height,
      timestamp: new Date().toISOString()
    };
    
    // Use sendBeacon if available, fallback to fetch
    var payload = JSON.stringify(data);
    // For anti-adblock, send to the site ID path (e.g., /3PXT05lP0j)
    // which the proxy will rewrite to /track
    var trackUrl = currentScript.src.replace('.js', '');
    
    if (navigator.sendBeacon) {
      navigator.sendBeacon(trackUrl, payload);
    } else {
      fetch(trackUrl, {
        method: 'POST',
        body: payload,
        headers: {
          'Content-Type': 'application/json'
        },
        keepalive: true
      }).catch(function(error) {
        console.error('Slimlytics tracking error:', error);
      });
    }
  }
  
  // Track initial pageview
  track('pageview');
  
  // Track page visibility changes
  document.addEventListener('visibilitychange', function() {
    if (document.hidden) {
      track('page_hidden', { hiddenAt: new Date().toISOString() });
    }
  });
  
  // Track clicks on external links
  document.addEventListener('click', function(e) {
    var target = e.target;
    while (target && target.tagName !== 'A') {
      target = target.parentElement;
    }
    
    if (target && target.href && target.hostname !== window.location.hostname) {
      track('outbound_click', { 
        url: target.href,
        text: target.textContent.substring(0, 100)
      });
    }
  });
  
  // Expose tracking function globally
  window.slimAnalytics = {
    track: track,
    getVisitorId: getVisitorId,
    getSessionId: getSessionId
  };
})();
`;

    // Return JavaScript with appropriate headers
    return c.text(script, 200, {
      'Content-Type': 'application/javascript',
      'Cache-Control': 'public, max-age=3600' // Cache for 1 hour
    });
  });
  
  // Noscript GIF beacon endpoint - use specific path to avoid conflicts
  app.get('/t/:siteId([a-zA-Z0-9-]+)ns.gif', async (c) => {
    const { siteId } = c.req.param();
    
    // Log the pageview (simplified version for noscript)
    const ip = c.req.header('x-forwarded-for') || c.req.header('x-real-ip') || 'unknown';
    const referrer = c.req.header('referer') || null;
    const userAgent = c.req.header('user-agent') || 'unknown';
    
    // Track the noscript pageview
    try {
      // Import necessary functions
      const { Database } = await import('bun:sqlite');
      const { trackEvent } = await import('../../db/queries.js');
      const geoip = await import('../../../api/services/geoip.js');
      
      const db = new Database('./data/analytics.db');
      const geoData = await geoip.default.lookup(ip);
      
      // Simple hash function for IP
      const crypto = require('crypto');
      const ipHash = crypto.createHash('sha256').update(ip + (process.env.SALT || 'default-salt')).digest('hex').substring(0, 16);
      
      trackEvent(db, {
        site_id: siteId,
        page_url: referrer || 'unknown',
        referrer: null,
        user_agent: userAgent,
        ip_hash: ipHash,
        screen_resolution: null,
        language: null,
        timestamp: new Date().toISOString(),
        visitor_id: 'noscript',
        session_id: 'noscript',
        event_type: 'noscript_pageview',
        event_data: null,
        country: geoData.country,
        country_code: geoData.countryCode,
        region: geoData.region,
        city: geoData.city,
        latitude: geoData.latitude,
        longitude: geoData.longitude,
        timezone: geoData.timezone,
        asn: geoData.asn,
        asn_org: geoData.asnOrg
      });
    } catch (error) {
      console.error('Error tracking noscript pageview:', error);
    }
    
    // Return 1x1 transparent GIF
    const gif = Buffer.from('R0lGODlhAQABAIAAAP///wAAACH5BAEAAAAALAAAAAABAAEAAAICRAEAOw==', 'base64');
    
    return new Response(gif, {
      headers: {
        'Content-Type': 'image/gif',
        'Cache-Control': 'no-cache, no-store, must-revalidate',
        'Pragma': 'no-cache',
        'Expires': '0'
      }
    });
  });
}