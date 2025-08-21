import { Database } from 'bun:sqlite';

// Connect to database
const db = new Database('./data/analytics.db');

async function testSessionCounts() {
  const baseUrl = 'http://localhost:3000';
  const testSiteId = 'demo';
  
  // Clean up any existing test data
  console.log('Cleaning up old test data...');
  db.prepare('DELETE FROM events WHERE visitor_id LIKE ?').run('test-count-%');
  db.prepare('DELETE FROM sessions WHERE visitor_id LIKE ?').run('test-count-%');
  
  // Create a test visitor with session
  const visitorId = 'test-count-' + Date.now();
  const sessionId = 'test-session-' + Date.now();
  
  console.log('\n=== Test Scenario: One visitor, multiple pageviews ===');
  console.log('Visitor ID:', visitorId);
  console.log('Session ID:', sessionId);
  
  // First pageview - creates session
  console.log('\n1. First pageview (direct traffic)...');
  await fetch(`${baseUrl}/track`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      siteId: testSiteId,
      visitorId: visitorId,
      sessionId: sessionId,
      url: 'https://example.com/page1',
      referrer: null,
      userAgent: 'Test Browser',
      language: 'en-US',
      screenWidth: 1920,
      screenHeight: 1080,
      eventType: 'pageview'
    })
  });
  
  // Second pageview - same session
  console.log('2. Second pageview (same session)...');
  await fetch(`${baseUrl}/track`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      siteId: testSiteId,
      visitorId: visitorId,
      sessionId: sessionId,
      url: 'https://example.com/page2',
      referrer: 'https://example.com/page1',
      userAgent: 'Test Browser',
      language: 'en-US',
      screenWidth: 1920,
      screenHeight: 1080,
      eventType: 'pageview'
    })
  });
  
  // Third pageview - same session
  console.log('3. Third pageview (same session)...');
  await fetch(`${baseUrl}/track`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      siteId: testSiteId,
      visitorId: visitorId,
      sessionId: sessionId,
      url: 'https://example.com/page3',
      referrer: 'https://example.com/page2',
      userAgent: 'Test Browser',
      language: 'en-US',
      screenWidth: 1920,
      screenHeight: 1080,
      eventType: 'pageview'
    })
  });
  
  // Wait a moment for processing
  await new Promise(resolve => setTimeout(resolve, 500));
  
  // Check database directly
  console.log('\n=== Database Check ===');
  
  const events = db.prepare('SELECT COUNT(*) as count FROM events WHERE visitor_id = ?').get(visitorId);
  console.log('Events in database:', events.count);
  
  const sessions = db.prepare('SELECT * FROM sessions WHERE visitor_id = ?').all(visitorId);
  console.log('Sessions in database:', sessions.length);
  if (sessions.length > 0) {
    console.log('Session details:');
    sessions.forEach(s => {
      console.log('  - Traffic source:', s.traffic_source);
      console.log('  - Language:', s.language);
      console.log('  - Page views:', s.page_views);
    });
  }
  
  // Get stats from API
  console.log('\n=== API Stats Check ===');
  const statsResponse = await fetch(`${baseUrl}/api/stats/${testSiteId}`);
  const stats = await statsResponse.json();
  
  console.log('Total visitors:', stats.visitors);
  console.log('Total page views:', stats.pageViews);
  
  if (stats.topLocales && stats.topLocales.length > 0) {
    console.log('\nTop Locales:');
    stats.topLocales.forEach(locale => {
      if (locale.locale === 'en-US') {
        console.log(`  ${locale.language}: ${locale.count} sessions (should be 1)`);
      }
    });
  }
  
  if (stats.trafficSources && stats.trafficSources.length > 0) {
    console.log('\nTraffic Sources:');
    stats.trafficSources.forEach(source => {
      if (source.source === 'Direct') {
        console.log(`  ${source.source}: ${source.count} sessions (should be 1)`);
      }
    });
  }
  
  // Check recent visitors
  if (stats.recentVisitors && stats.recentVisitors.length > 0) {
    const testVisitors = stats.recentVisitors.filter(v => v.ip_hash === visitorId);
    console.log('\nRecent Visitors matching test:', testVisitors.length, '(should be 1)');
  }
  
  // Clean up
  console.log('\n=== Cleanup ===');
  db.prepare('DELETE FROM events WHERE visitor_id = ?').run(visitorId);
  db.prepare('DELETE FROM sessions WHERE visitor_id = ?').run(visitorId);
  console.log('Test data cleaned up.');
  
  console.log('\n✅ Test complete!');
  console.log('Expected: 1 visitor, 1 session, 3 page views');
  console.log('Actual:', stats.visitors, 'visitors,', sessions.length, 'sessions,', events.count, 'page views');
}

// Run test
testSessionCounts().catch(console.error).finally(() => {
  db.close();
  process.exit(0);
});