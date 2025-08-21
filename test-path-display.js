// Test script to verify path display in Content section

async function testPathDisplay() {
  const baseUrl = 'http://localhost:3000';
  const testSiteId = 'demo';
  
  console.log('Creating test pageviews with different URL formats...\n');
  
  const testUrls = [
    'https://example.com/',
    'https://example.com/about/',
    'https://example.com/blog/post-1',
    'https://example.com/products/category/item-123',
    'https://example.com/privacy-policy/',
  ];
  
  // Create a unique visitor/session for this test
  const visitorId = 'test-path-' + Date.now();
  const sessionId = 'test-session-path-' + Date.now();
  
  // Send pageviews
  for (const url of testUrls) {
    await fetch(`${baseUrl}/track`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        siteId: testSiteId,
        visitorId: visitorId,
        sessionId: sessionId,
        url: url,
        referrer: null,
        userAgent: 'Test Browser',
        language: 'en-US',
        screenWidth: 1920,
        screenHeight: 1080,
        eventType: 'pageview'
      })
    });
    console.log(`Tracked: ${url}`);
  }
  
  // Wait for processing
  await new Promise(resolve => setTimeout(resolve, 500));
  
  // Get stats
  const statsResponse = await fetch(`${baseUrl}/api/stats/${testSiteId}`);
  const stats = await statsResponse.json();
  
  console.log('\n=== Top Pages from API ===');
  if (stats.topPages && stats.topPages.length > 0) {
    stats.topPages.forEach(page => {
      // Extract path from URL for display
      let path = page.url;
      try {
        const urlObj = new URL(page.url);
        path = urlObj.pathname || '/';
      } catch (e) {
        // Already a path
      }
      console.log(`Path: ${path} (${page.views} views)`);
      console.log(`  Full URL: ${page.url}`);
    });
  }
  
  console.log('\n✅ Test complete!');
  console.log('The Content section should now show only paths like:');
  console.log('  /');
  console.log('  /about/');
  console.log('  /blog/post-1');
  console.log('  /products/category/item-123');
  console.log('  /privacy-policy/');
  console.log('\nInstead of full URLs with domains.');
  
  // Clean up test data
  const { Database } = await import('bun:sqlite');
  const db = new Database('./data/analytics.db');
  db.prepare('DELETE FROM events WHERE visitor_id = ?').run(visitorId);
  db.prepare('DELETE FROM sessions WHERE visitor_id = ?').run(visitorId);
  db.close();
  console.log('\nTest data cleaned up.');
}

testPathDisplay().catch(console.error);