import { Database } from 'bun:sqlite';

async function testRecentVisitors() {
  const baseUrl = 'http://localhost:3000';
  const testSiteId = 'demo';
  
  console.log('Testing Recent Visitors display...\n');
  
  // Create test visitors with different IPs
  const visitors = [
    { ip: '216.41.234.123', country: 'US', path: '/blog/envelope-budgeting-basics/' },
    { ip: '68.234.46.45', country: 'US', path: '/blog/markdown-reports-ai-coaching/' },
    { ip: '149.57.191.78', country: 'NL', path: '/' },
    { ip: '185.50.234.90', country: 'AT', path: '/' },
  ];
  
  for (let i = 0; i < visitors.length; i++) {
    const visitor = visitors[i];
    const visitorId = 'test-visitor-' + Date.now() + '-' + i;
    const sessionId = 'test-session-' + Date.now() + '-' + i;
    
    // Simulate tracking with custom IP header
    await fetch(`${baseUrl}/track`, {
      method: 'POST',
      headers: { 
        'Content-Type': 'application/json',
        'x-forwarded-for': visitor.ip
      },
      body: JSON.stringify({
        siteId: testSiteId,
        visitorId: visitorId,
        sessionId: sessionId,
        url: `https://example.com${visitor.path}`,
        referrer: null,
        userAgent: 'Test Browser',
        language: 'en-US',
        screenWidth: 1920,
        screenHeight: 1080,
        eventType: 'pageview'
      })
    });
    
    console.log(`Created visitor from ${visitor.country} (${visitor.ip}) visiting ${visitor.path}`);
    
    // Add some delay between visitors
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  
  // Wait for processing
  await new Promise(resolve => setTimeout(resolve, 500));
  
  // Get stats to check recent visitors
  const statsResponse = await fetch(`${baseUrl}/api/stats/${testSiteId}`);
  const stats = await statsResponse.json();
  
  console.log('\n=== Recent Visitors from API ===');
  if (stats.recentVisitors && stats.recentVisitors.length > 0) {
    stats.recentVisitors.slice(0, 5).forEach(visitor => {
      const time = new Date(visitor.timestamp);
      const hours = time.getHours();
      const minutes = time.getMinutes().toString().padStart(2, '0');
      const ampm = hours >= 12 ? 'p' : 'a';
      const displayHours = hours % 12 || 12;
      const timeStr = `${displayHours}:${minutes}${ampm}`;
      
      // Extract path from URL
      let path = '/';
      try {
        const url = new URL(visitor.page_url);
        path = url.pathname;
      } catch (e) {
        path = visitor.page_url;
      }
      
      console.log(`${timeStr} ${visitor.country_code || '??'} ${visitor.ip_address || visitor.ip_hash} ${path}`);
    });
  }
  
  console.log('\n✅ Test complete!');
  console.log('Recent Visitors should show:');
  console.log('- Time in format like "7:26a" or "1:20p"');
  console.log('- Country flag emoji');
  console.log('- Actual IP address (not hashed)');
  console.log('- Path as clickable link');
  
  // Test Clear Data functionality
  console.log('\n=== Testing Clear Data ===');
  const clearResponse = await fetch(`${baseUrl}/api/stats/${testSiteId}/data`, {
    method: 'DELETE',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ range: 'today' })
  });
  const clearResult = await clearResponse.json();
  console.log('Clear Data result:', clearResult);
  
  // Clean up remaining test data
  const db = new Database('./data/analytics.db');
  db.prepare('DELETE FROM events WHERE visitor_id LIKE ?').run('test-visitor-%');
  db.prepare('DELETE FROM sessions WHERE visitor_id LIKE ?').run('test-visitor-%');
  db.close();
  console.log('Test data cleaned up.');
}

testRecentVisitors().catch(console.error);