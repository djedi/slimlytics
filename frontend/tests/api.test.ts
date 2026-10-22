import { describe, expect, it, vi } from 'vitest';
import { ApiClient, ApiError, dates, type Site } from '../src/lib/api';

describe('ApiClient', () => {
  it('uses the configured base URL, bearer auth, and camelCase contract', async () => {
    const fetcher = vi.fn().mockResolvedValue(new Response(JSON.stringify({ id: 's1', writeKey: 'wk' }), { status: 200, headers: { 'content-type': 'application/json' } }));
    const api = new ApiClient('https://example.test/api/', fetcher, false);
    api.setToken('secret');
    await api.site('s1');
    expect(fetcher).toHaveBeenCalledWith('https://example.test/api/sites/s1', expect.objectContaining({ headers: expect.objectContaining({ authorization: 'Bearer secret' }) }));
  });

  it('throws a typed error and never silently returns demo data', async () => {
    const api = new ApiClient('/api', vi.fn().mockResolvedValue(new Response('{"message":"Nope"}', { status: 403 })), false);
    await expect(api.sites()).rejects.toMatchObject({ status: 403, message: 'Nope' } satisfies Partial<ApiError>);
  });

  it('provides realistic fallback only when demo mode is explicit', async () => {
    const api = new ApiClient('/api', vi.fn().mockRejectedValue(new Error('offline')), true);
    const sites = await api.sites();
    expect(sites).toHaveLength(3);
    expect(sites[0].overview?.trend).toHaveLength(28);
    await expect(api.journeys('northstar')).resolves.toEqual([]);
    await expect(api.searchConsoleStatus('northstar')).resolves.toMatchObject({
      configured: true,
      connected: true
    });
  });

  it('persists per-site anti-adblock server and path settings', async () => {
    const fetcher = vi.fn().mockResolvedValue(new Response(JSON.stringify({
      id: 's1',
      name: 'Example',
      domain: 'example.com',
      writeKey: 'wk',
      antiAdblockServer: 'caddy',
      antiAdblockJsPath: '/456bbb63bb86.js',
      antiAdblockBeaconPath: '/0d31360a3101'
    }), { status: 200, headers: { 'content-type': 'application/json' } }));
    const api = new ApiClient('/api', fetcher, false);
    api.setToken('secret');
    await api.updateAntiAdblock('s1', {
      serverType: 'caddy',
      jsPath: '/456bbb63bb86.js',
      beaconPath: '/0d31360a3101'
    });
    expect(fetcher).toHaveBeenCalledWith('/api/sites/s1/anti-adblock', expect.objectContaining({
      method: 'PUT',
      body: JSON.stringify({ serverType: 'caddy', jsPath: '/456bbb63bb86.js', beaconPath: '/0d31360a3101' })
    }));
  });

  it('reads collection health for operational and agent diagnostics', async () => {
    const fetcher = vi.fn().mockResolvedValue(new Response(JSON.stringify({
      acceptedTotal: 42,
      rejectedTotal: 3,
      lastAcceptedAt: '2026-07-30T22:00:00Z',
      lastRejectedAt: null,
      lastRejectionCode: null,
      lastTrackerVersion: '1.0.0'
    }), { status: 200, headers: { 'content-type': 'application/json' } }));
    const api = new ApiClient('/api', fetcher, false);
    api.setToken('secret');

    await expect(api.collectionHealth('s1')).resolves.toMatchObject({
      acceptedTotal: 42,
      lastTrackerVersion: '1.0.0'
    });
    expect(fetcher).toHaveBeenCalledWith(
      '/api/sites/s1/collection-health',
      expect.objectContaining({ headers: expect.objectContaining({ authorization: 'Bearer secret' }) })
    );
  });

  it('normalizes engagement metrics and daily trend evidence', async () => {
    const fetcher = vi.fn().mockResolvedValue(new Response(JSON.stringify({
      views: { current: 12, previous: 8, change_percent: 50 },
      visitors: { current: 7, previous: 5, change_percent: 40 },
      sessions: { current: 8, previous: 6, change_percent: 33.3 },
      events: { current: 2, previous: 1, change_percent: 100 },
      currentOnline: 1,
      bounceRate: 37.5,
      avgDurationSeconds: 83.2,
      trend: [{ date: '2026-07-30', visitors: 7, pageViews: 12 }]
    }), { status: 200, headers: { 'content-type': 'application/json' } }));
    const api = new ApiClient('/api', fetcher, false);

    await expect(api.overview('s1', { period: 7, compare: 'previous' })).resolves.toMatchObject({
      visitors: 7,
      pageViews: 12,
      bounceRate: 37.5,
      avgDuration: 83.2,
      trend: [{ date: '2026-07-30', visitors: 7, pageViews: 12 }]
    });
  });

  it('loads every site summary in one request', async () => {
    const fetcher = vi.fn().mockResolvedValue(new Response(JSON.stringify([{
      siteId: 's1',
      views: 12,
      visitors: { current: 7, previous: 5, change_percent: 40 },
      currentOnline: 1,
      bounceRate: 37.5,
      avgDurationSeconds: 83.2,
      trend: [{ date: '2026-07-30', visitors: 7, pageViews: 12 }]
    }]), { status: 200, headers: { 'content-type': 'application/json' } }));
    const api = new ApiClient('/api', fetcher, false);

    const summaries = await api.siteSummaries({ period: 7, compare: 'previous' }, [{ id: 's1', timezone: 'UTC' } as Site]);
    expect(fetcher).toHaveBeenCalledTimes(1);
    expect(String(fetcher.mock.calls[0][0])).toMatch(/^\/api\/sites\/summary\?from=\d{4}-\d{2}-\d{2}&to=/);
    expect(summaries.get('s1')).toMatchObject({
      visitors: 7,
      previousVisitors: 5,
      pageViews: 12,
      change: 40,
      currentOnline: 1,
      bounceRate: 37.5,
      avgDuration: 83.2,
      trend: [{ date: '2026-07-30', visitors: 7, pageViews: 12 }]
    });
  });

  it('normalizes goal conversion evidence returned by the API', async () => {
    const fetcher = vi.fn().mockResolvedValue(new Response(JSON.stringify([{
      id: 'g1',
      name: 'Signup',
      eventName: 'signup',
      pathPattern: null,
      conversions: 9,
      conversionRate: 12.5
    }]), { status: 200, headers: { 'content-type': 'application/json' } }));
    const api = new ApiClient('/api', fetcher, false);

    await expect(api.goals('s1')).resolves.toEqual([{
      id: 'g1',
      name: 'Signup',
      type: 'event',
      target: 'signup',
      conversions: 9,
      conversionRate: 12.5
    }]);
  });

  it('exposes dated marketing intelligence for humans and agents', async () => {
    const responses = [
      [{ steps: ['/', '/pricing'], sessions: 12, visitors: 10 }],
      [{ source: 'newsletter', medium: 'email', campaign: 'launch', visitors: 20, conversions: 4, revenue: 199.8 }],
      [{ date: '2026-07-30', metric: 'pageViews', value: 100, baseline: 50, deviationPercent: 100, direction: 'up' }],
      [{ id: 'f1', name: 'Signup', steps: [{ label: 'Visit', path: '/' }], createdAt: '2026-07-30T00:00:00Z' }],
      { id: 'f1', name: 'Signup', from: '2026-07-03', to: '2026-07-30', steps: [{ index: 1, label: 'Visit', visitors: 10, conversionRate: 100 }] }
    ];
    const fetcher = vi.fn();
    for (const body of responses) {
      fetcher.mockResolvedValueOnce(new Response(JSON.stringify(body), {
        status: 200,
        headers: { 'content-type': 'application/json' }
      }));
    }
    const api = new ApiClient('/api', fetcher, false);

    await expect(api.journeys('s1', 28)).resolves.toHaveLength(1);
    await expect(api.attribution('s1', 28)).resolves.toMatchObject([{ revenue: 199.8 }]);
    await expect(api.anomalies('s1', 28)).resolves.toMatchObject([{ direction: 'up' }]);
    await expect(api.funnels('s1')).resolves.toMatchObject([{ id: 'f1' }]);
    await expect(api.funnelReport('s1', 'f1', 28)).resolves.toMatchObject({ name: 'Signup' });

    expect(fetcher.mock.calls[0][0]).toMatch(/^\/api\/sites\/s1\/insights\/journeys\?from=/);
    expect(fetcher.mock.calls[4][0]).toMatch(/^\/api\/sites\/s1\/funnels\/f1\/report\?from=/);
  });

  it('manages Search Console connection, sync, and reports', async () => {
    const fetcher = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify({ configured: true, connected: false }), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ authorizationUrl: 'https://accounts.google.com/o/oauth2/v2/auth' }), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ status: 'ok', rows: 12 }), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify([{ value: 'analytics', clicks: 10, impressions: 100, ctr: 0.1, position: 3.2 }]), { status: 200 }));
    const api = new ApiClient('/api', fetcher, false);

    await expect(api.searchConsoleStatus('s1')).resolves.toMatchObject({ configured: true });
    await expect(api.connectSearchConsole('s1')).resolves.toHaveProperty('authorizationUrl');
    await expect(api.syncSearchConsole('s1', 28)).resolves.toMatchObject({ rows: 12 });
    await expect(api.searchConsoleReport('s1', 'query', 28)).resolves.toHaveLength(1);

    expect(fetcher.mock.calls[2][0]).toMatch(/^\/api\/sites\/s1\/integrations\/search-console\/sync\?from=/);
    expect(fetcher.mock.calls[3][0]).toContain('dimension=query');
  });

  it('adds and verifies the site as a Search Console domain property', async () => {
    const added = {
      propertyUrl: 'sc-domain:example.com',
      verificationMethod: 'DNS_TXT',
      dnsRecord: { type: 'TXT', name: 'example.com', value: 'google-site-verification=abc' }
    };
    const fetcher = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(added), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ propertyUrl: 'sc-domain:example.com', verified: true }), { status: 200 }));
    const api = new ApiClient('/api', fetcher, false);

    await expect(api.addSearchConsoleProperty('s1')).resolves.toEqual(added);
    await expect(api.verifySearchConsoleProperty('s1')).resolves.toMatchObject({ verified: true });

    expect(fetcher.mock.calls[0][0]).toBe('/api/sites/s1/integrations/search-console/property');
    expect(fetcher.mock.calls[0][1]).toMatchObject({ method: 'POST' });
    expect(fetcher.mock.calls[1][0]).toBe('/api/sites/s1/integrations/search-console/property/verify');
    expect(fetcher.mock.calls[1][1]).toMatchObject({ method: 'POST' });
  });

  it('shows sample Search Console data in demo mode', async () => {
    const api = new ApiClient('/api', vi.fn().mockRejectedValue(new Error('offline')), true);
    const queries = await api.searchConsoleReport('northstar', 'query', 28);
    expect(queries.length).toBeGreaterThan(0);
    expect(queries[0]).toEqual(expect.objectContaining({ clicks: expect.any(Number), impressions: expect.any(Number) }));
  });

  it('manages signed scheduled marketing briefs', async () => {
    const subscription = {
      id: 'r1', siteId: 's1', name: 'Weekly brief', webhookUrl: 'https://hooks.example.com/report',
      frequency: 'weekly', anomalyOnly: false, enabled: true,
      nextRunAt: '2026-08-07T12:00:00Z', createdAt: '2026-07-31T12:00:00Z'
    };
    const fetcher = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify([subscription]), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ ...subscription, signingSecret: 'once' }), { status: 201 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ status: 'success' }), { status: 200 }));
    const api = new ApiClient('/api', fetcher, false);

    await expect(api.reportSubscriptions('s1')).resolves.toHaveLength(1);
    await expect(api.createReportSubscription('s1', {
      name: 'Weekly brief', webhookUrl: 'https://hooks.example.com/report',
      frequency: 'weekly', anomalyOnly: false, enabled: true
    })).resolves.toMatchObject({ signingSecret: 'once' });
    await expect(api.deliverReportSubscription('s1', 'r1')).resolves.toEqual({ status: 'success' });
    expect(fetcher.mock.calls[1][1]).toMatchObject({ method: 'POST' });
  });
});

describe('session expiry', () => {
  it('notifies when an authenticated request is rejected, but not for a failed login', async () => {
    const fetcher = (async () => new Response(JSON.stringify({ message: 'unauthorized' }), { status: 401 })) as typeof fetch;
    const client = new ApiClient('/api', fetcher);
    let calls = 0;
    client.onUnauthorized = () => { calls += 1; };
    await expect(client.login('a@b.co', 'wrong-password')).rejects.toThrow();
    expect(calls).toBe(0);
    client.setToken('expired');
    await expect(client.sites()).rejects.toThrow();
    expect(calls).toBe(1);
  });
});

describe('date ranges', () => {
  // 8:55 PM on Oct 21 in Denver is already Oct 22 in UTC.
  const evening = new Date('2026-10-22T02:55:00Z');

  it('ends on today in the site timezone, not the UTC date', () => {
    expect(dates(28, 'America/Denver', evening)).toEqual({ from: '2026-09-24', to: '2026-10-21' });
    expect(dates('today', 'UTC', evening)).toEqual({ from: '2026-10-22', to: '2026-10-22' });
    expect(dates('yesterday', 'America/Denver', evening)).toEqual({ from: '2026-10-20', to: '2026-10-20' });
  });

  it('requests per-site reports in that site timezone', async () => {
    vi.useFakeTimers({ now: evening, toFake: ['Date'] });
    try {
      const fetcher = vi.fn()
        .mockResolvedValueOnce(new Response(JSON.stringify([{ id: 's1', writeKey: 'wk', timezone: 'America/Denver' }]), { status: 200 }))
        .mockResolvedValueOnce(new Response('[]', { status: 200 }));
      const api = new ApiClient('/api', fetcher, false);
      await api.sites();
      await api.report('s1', 'pages', 7);
      expect(fetcher.mock.calls[1][0]).toBe('/api/sites/s1/reports/pages?from=2026-10-15&to=2026-10-21');
    } finally {
      vi.useRealTimers();
    }
  });

  it('requests yesterday as one whole local day', async () => {
    vi.useFakeTimers({ now: evening, toFake: ['Date'] });
    try {
      const fetcher = vi.fn()
        .mockResolvedValueOnce(new Response(JSON.stringify([{ id: 's1', writeKey: 'wk', timezone: 'America/Denver' }]), { status: 200 }))
        .mockResolvedValueOnce(new Response('[]', { status: 200 }));
      const api = new ApiClient('/api', fetcher, false);
      await api.sites();
      await api.visitors('s1', 'yesterday');
      expect(fetcher.mock.calls[1][0]).toBe('/api/sites/s1/visitors?from=2026-10-20&to=2026-10-20');
    } finally {
      vi.useRealTimers();
    }
  });

  it('remembers the timezone of a newly created site', async () => {
    vi.useFakeTimers({ now: evening, toFake: ['Date'] });
    try {
      const fetcher = vi.fn()
        .mockResolvedValueOnce(new Response(JSON.stringify({ id: 's2', writeKey: 'wk', timezone: 'UTC' }), { status: 200 }))
        .mockResolvedValueOnce(new Response('[]', { status: 200 }));
      const api = new ApiClient('/api', fetcher, false);
      await api.createSite({ name: 'New', domain: 'new.test' });
      await api.report('s2', 'pages', 'today');
      expect(fetcher.mock.calls[1][0]).toBe('/api/sites/s2/reports/pages?from=2026-10-22&to=2026-10-22');
    } finally {
      vi.useRealTimers();
    }
  });

  it('gives each site a summary card for its own calendar days', async () => {
    vi.useFakeTimers({ now: evening, toFake: ['Date'] });
    try {
      const row = (siteId: string, views: number) => ({ siteId, views, visitors: { current: views, previous: 0 }, currentOnline: 0, bounceRate: 0, avgDurationSeconds: 0, trend: [] });
      const fetcher = vi.fn(async (input: RequestInfo | URL) => {
        const url = String(input);
        if (url === '/api/sites') return new Response(JSON.stringify([{ id: 'den', writeKey: 'a', timezone: 'America/Denver' }, { id: 'utc', writeKey: 'b', timezone: 'UTC' }]), { status: 200 });
        const today = url.includes('to=2026-10-22') ? 22 : 21;
        return new Response(JSON.stringify([row('den', today), row('utc', today)]), { status: 200 });
      });
      const api = new ApiClient('/api', fetcher, false);
      const summaries = await api.siteSummaries({ period: 'today', compare: 'week' });
      expect(fetcher.mock.calls.map(([url]) => url).sort()).toEqual([
        '/api/sites',
        '/api/sites/summary?from=2026-10-21&to=2026-10-21&compare=week',
        '/api/sites/summary?from=2026-10-22&to=2026-10-22&compare=week'
      ]);
      expect(summaries.get('den')?.pageViews).toBe(21);
      expect(summaries.get('utc')?.pageViews).toBe(22);
    } finally {
      vi.useRealTimers();
    }
  });
});
