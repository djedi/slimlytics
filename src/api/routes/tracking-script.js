export function setupTrackingScriptRoute(app) {
  // Serve dynamic tracking script
  app.get('/js/:filename', (req, res) => {
    const { siteId } = req.query;
    const beaconPath = req.headers['x-beacon-path'] || '/track';
    
    if (!siteId) {
      return res.status(400).send('Missing siteId parameter');
    }

    // Generate the tracking script dynamically
    const script = `
(function() {
  'use strict';
  
  // Configuration
  var SITE_ID = '${siteId}';
  var BEACON_URL = document.currentScript.getAttribute('data-beacon-url') || '${beaconPath}';
  
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
    
    if (navigator.sendBeacon) {
      navigator.sendBeacon(BEACON_URL, payload);
    } else {
      fetch(BEACON_URL, {
        method: 'POST',
        body: payload,
        headers: {
          'Content-Type': 'application/json'
        },
        keepalive: true
      }).catch(function(error) {
        console.error('Slim Analytics tracking error:', error);
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

    // Set appropriate headers
    res.setHeader('Content-Type', 'application/javascript');
    res.setHeader('Cache-Control', 'public, max-age=3600'); // Cache for 1 hour
    res.send(script);
  });
}