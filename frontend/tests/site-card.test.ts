import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import SiteCard from '../src/lib/components/rollup/SiteCard.svelte';

const site = {
  id: 's1',
  name: 'Shop',
  domain: 'shop.example.com',
  writeKey: 'k',
  serverWriteKey: 'k2',
  antiAdblockServer: 'caddy' as const,
  antiAdblockJsPath: '/a.js',
  antiAdblockBeaconPath: '/b'
};
const zeroTraffic = { visitors: 0, previousVisitors: 0, sessions: 0, pageViews: 0, bounceRate: 0, avgDuration: 0, change: 0, currentOnline: 0, trend: [{ date: '2026-09-30', visitors: 0, pageViews: 0 }] };

describe('site card', () => {
  afterEach(cleanup);

  it('shows a loaded zero-traffic site as quiet, linking to installation settings', () => {
    render(SiteCard, { site: { ...site, overview: zeroTraffic }, days: 28 });
    expect(document.body).toHaveTextContent('No visits in the last 28 days');
    expect(screen.getByRole('link')).toHaveAttribute('href', '/app/s1/settings');
  });

  it('does not mistake a failed overview for a quiet site', () => {
    render(SiteCard, { site, days: 28 });
    expect(document.body).toHaveTextContent('Stats unavailable');
    expect(document.body).not.toHaveTextContent('No visits');
    expect(document.body).not.toHaveTextContent('Check installation');
    expect(screen.getByRole('link')).toHaveAttribute('href', '/app/s1');
  });
});
