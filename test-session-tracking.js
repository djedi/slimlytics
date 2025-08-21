import { Database } from 'bun:sqlite';

// Connect to database
const db = new Database('./data/analytics.db');

// Test tracking with sessions
async function testSessionTracking() {
  const baseUrl = 'http://localhost:3000';
  
  // Simulate a visitor session
  const visitorId = 'test-visitor-' + Date.now();
  const sessionId = 'test-session-' + Date.now();
  const siteId = 'demo';
  
  console.log('Testing session tracking...');
  console.log('Visitor ID:', visitorId);
  console.log('Session ID:', sessionId);
  
  // First pageview - should create a new session
  console.log('\n1. First pageview (with referrer from Google)...');
  const response1 = await fetch(`${baseUrl}/track`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      siteId: siteId,
      visitorId: visitorId,
      sessionId: sessionId,
      url: 'https://example.com/home',
      referrer: 'https://www.google.com/search?q=test',
      userAgent: 'Mozilla/5.0 Test Browser',
      language: 'en-US',
      screenWidth: 1920,
      screenHeight: 1080,
      eventType: 'pageview'
    })
  });
  console.log('Response:', response1.status);
  
  // Second pageview - same session
  console.log('\n2. Second pageview (same session)...');
  const response2 = await fetch(`${baseUrl}/track`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      siteId: siteId,
      visitorId: visitorId,
      sessionId: sessionId,
      url: 'https://example.com/about',
      referrer: 'https://example.com/home',
      userAgent: 'Mozilla/5.0 Test Browser',
      language: 'en-US',
      screenWidth: 1920,
      screenHeight: 1080,
      eventType: 'pageview'
    })
  });
  console.log('Response:', response2.status);
  
  // New session from same visitor
  const sessionId2 = 'test-session-2-' + Date.now();
  console.log('\n3. New session from same visitor (direct traffic)...');
  console.log('New Session ID:', sessionId2);
  
  const response3 = await fetch(`${baseUrl}/track`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      siteId: siteId,
      visitorId: visitorId,
      sessionId: sessionId2,
      url: 'https://example.com/contact',
      referrer: null,
      userAgent: 'Mozilla/5.0 Test Browser',
      language: 'fr-FR',  // Different language for this session
      screenWidth: 1920,
      screenHeight: 1080,
      eventType: 'pageview'
    })
  });
  console.log('Response:', response3.status);
  
  // Check database
  console.log('\n4. Checking database...');
  
  // Check sessions
  const sessions = db.prepare(`
    SELECT * FROM sessions 
    WHERE visitor_id = ? 
    ORDER BY started_at
  `).all(visitorId);
  
  console.log('\nSessions created:', sessions.length);
  sessions.forEach((session, i) => {
    console.log(`\nSession ${i + 1}:`);
    console.log('  Session ID:', session.session_id);
    console.log('  Traffic Source:', session.traffic_source);
    console.log('  Referrer:', session.referrer);
    console.log('  Language:', session.language);
    console.log('  Page Views:', session.page_views);
    console.log('  Country:', session.country);
    console.log('  City:', session.city);
  });
  
  // Check events
  const events = db.prepare(`
    SELECT * FROM events 
    WHERE visitor_id = ? 
    ORDER BY timestamp
  `).all(visitorId);
  
  console.log('\nEvents tracked:', events.length);
  events.forEach((event, i) => {
    console.log(`  Event ${i + 1}: ${event.page_url} (session: ${event.session_id})`);
  });
  
  // Get stats
  console.log('\n5. Getting stats...');
  const statsResponse = await fetch(`${baseUrl}/api/stats/${siteId}`);
  const stats = await statsResponse.json();
  
  console.log('\nStats Overview:');
  console.log('  Page Views:', stats.pageViews);
  console.log('  Unique Visitors:', stats.uniqueVisitors);
  console.log('  Total Sessions:', stats.totalSessions);
  
  if (stats.trafficSources) {
    console.log('\nTraffic Sources:');
    stats.trafficSources.forEach(source => {
      console.log(`  ${source.traffic_source}: ${source.count} sessions`);
    });
  }
  
  if (stats.topLanguages) {
    console.log('\nTop Languages:');
    stats.topLanguages.forEach(lang => {
      console.log(`  ${lang.language}: ${lang.count} sessions`);
    });
  }
  
  // Clean up test data
  console.log('\n6. Cleaning up test data...');
  db.prepare('DELETE FROM events WHERE visitor_id = ?').run(visitorId);
  db.prepare('DELETE FROM sessions WHERE visitor_id = ?').run(visitorId);
  console.log('Test data cleaned up.');
}

// Run test
testSessionTracking().catch(console.error).finally(() => {
  db.close();
  process.exit(0);
});