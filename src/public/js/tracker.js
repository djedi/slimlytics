(function() {
  const ANALYTICS_URL = 'http://localhost:3000/track';
  
  function getScreenResolution() {
    return window.screen ? `${window.screen.width}x${window.screen.height}` : 'unknown';
  }
  
  function trackPageView() {
    const data = {
      site_id: window.SLIMLYTICS_SITE_ID || 'demo',
      page_url: window.location.href,
      referrer: document.referrer,
      user_agent: navigator.userAgent,
      screen_resolution: getScreenResolution(),
      language: navigator.language || navigator.userLanguage
    };
    
    fetch(ANALYTICS_URL, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(data)
    }).catch(function(error) {
      console.error('Analytics tracking error:', error);
    });
  }
  
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', trackPageView);
  } else {
    trackPageView();
  }
  
  let lastUrl = window.location.href;
  const observer = new MutationObserver(function() {
    if (window.location.href !== lastUrl) {
      lastUrl = window.location.href;
      trackPageView();
    }
  });
  
  observer.observe(document.body, {
    childList: true,
    subtree: true
  });
})();