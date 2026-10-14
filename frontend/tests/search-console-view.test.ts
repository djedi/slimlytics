import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import SearchConsoleView from '../src/lib/components/SearchConsoleView.svelte';
import type { SearchConsoleRow, SearchConsoleStatus } from '../src/lib/api';

const connected: SearchConsoleStatus = {
  configured: true,
  connected: true,
  propertyUrl: 'sc-domain:example.com',
  lastSyncedAt: '2026-10-13T12:00:00Z'
};
const row = (value: string, clicks: number, impressions: number, position: number): SearchConsoleRow => ({
  value,
  clicks,
  impressions,
  ctr: impressions ? clicks / impressions : 0,
  position
});
const reports = {
  query: [row('privacy analytics', 30, 600, 4.2), row('cookieless analytics', 10, 400, 8.5)],
  page: [row('https://example.com/pricing', 25, 500, 5)],
  country: [row('usa', 20, 700, 6)],
  device: [row('MOBILE', 28, 650, 5.5)],
  date: [row('2026-10-12', 15, 400, 6), row('2026-10-13', 25, 600, 5.3)]
};

describe('Search Console view', () => {
  afterEach(cleanup);

  it('summarizes clicks, impressions, CTR, and weighted position', () => {
    render(SearchConsoleView, { status: connected, reports, settingsHref: '/settings', onSync: vi.fn() });
    const summary = screen.getByRole('region', { name: 'Search performance summary' });
    expect(summary).toHaveTextContent('40');
    expect(summary).toHaveTextContent('1,000');
    expect(summary).toHaveTextContent('4.0%');
    // Position is weighted by impressions: (6*400 + 5.3*600) / 1000.
    expect(summary).toHaveTextContent('5.6');
  });

  it('lists top queries, pages, countries, and devices', () => {
    render(SearchConsoleView, { status: connected, reports, settingsHref: '/settings', onSync: vi.fn() });
    expect(screen.getByRole('table', { name: 'Top queries' })).toHaveTextContent('privacy analytics');
    expect(screen.getByRole('table', { name: 'Top pages' })).toHaveTextContent('/pricing');
    expect(screen.getByRole('table', { name: 'Countries' })).toHaveTextContent('USA');
    expect(screen.getByRole('table', { name: 'Devices' })).toHaveTextContent('Mobile');
  });

  it('offers a sync when connected without data', async () => {
    const onSync = vi.fn();
    render(SearchConsoleView, {
      status: connected,
      reports: { query: [], page: [], country: [], device: [], date: [] },
      settingsHref: '/settings',
      onSync
    });
    expect(screen.getByText(/No Search Console data/)).toBeInTheDocument();
    screen.getByRole('button', { name: 'Sync now' }).click();
    expect(onSync).toHaveBeenCalledOnce();
  });

  it('points to settings when not connected or no property is set up', () => {
    render(SearchConsoleView, {
      status: { configured: true, connected: true, propertyUrl: null },
      reports: { query: [], page: [], country: [], device: [], date: [] },
      settingsHref: '/app/s1/settings',
      onSync: vi.fn()
    });
    expect(screen.getByRole('link', { name: /Set up in settings/ })).toHaveAttribute('href', '/app/s1/settings');
    expect(screen.queryByRole('button', { name: 'Sync now' })).not.toBeInTheDocument();
  });
});
