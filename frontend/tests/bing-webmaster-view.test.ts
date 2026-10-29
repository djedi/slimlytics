import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import BingWebmasterView from '../src/lib/components/BingWebmasterView.svelte';
import type { BingStatus, SearchConsoleRow } from '../src/lib/api';

const connected: BingStatus = {
  configured: true,
  connected: true,
  siteUrl: 'https://example.com/',
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
  query: [row('privacy analytics', 30, 600, 4), row('slimlytics', 10, 400, 0), row('cookieless', 5, 200, 7)],
  page: [row('https://example.com/pricing', 25, 500, 5)],
  date: [row('2026-10-12', 15, 400, 0), row('2026-10-13', 25, 600, 0)]
};

describe('Bing Webmaster view', () => {
  afterEach(cleanup);

  it('totals daily clicks and weights position by ranked query impressions', () => {
    render(BingWebmasterView, { status: connected, reports, settingsHref: '/settings', onSync: vi.fn() });
    const summary = screen.getByRole('region', { name: 'Bing search performance summary' });
    expect(summary).toHaveTextContent('40');
    expect(summary).toHaveTextContent('1,000');
    expect(summary).toHaveTextContent('4.0%');
    // Unranked rows are skipped: (4*600 + 7*200) / 800.
    expect(summary).toHaveTextContent('4.8');
  });

  it('lists queries with Bing links and pages by path', () => {
    render(BingWebmasterView, { status: connected, reports, settingsHref: '/settings', onSync: vi.fn() });
    expect(screen.getByRole('table', { name: 'Top queries' })).toHaveTextContent('privacy analytics');
    expect(screen.getByRole('table', { name: 'Top pages' })).toHaveTextContent('/pricing');
    expect(screen.getByRole('link', { name: 'Search Bing for “privacy analytics”' })).toHaveAttribute(
      'href',
      'https://www.bing.com/search?q=privacy%20analytics'
    );
  });

  it('sends unconnected sites to settings', () => {
    render(BingWebmasterView, {
      status: { configured: true, connected: false },
      reports: { query: [], page: [], date: [] },
      settingsHref: '/settings',
      onSync: vi.fn()
    });
    expect(screen.getByRole('heading', { name: 'Connect Bing Webmaster Tools' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Set up in settings' })).toHaveAttribute('href', '/settings');
  });

  it('syncs on demand', async () => {
    const onSync = vi.fn();
    render(BingWebmasterView, { status: connected, reports, settingsHref: '/settings', onSync });
    await fireEvent.click(screen.getByRole('button', { name: 'Sync now' }));
    expect(onSync).toHaveBeenCalled();
  });
});
