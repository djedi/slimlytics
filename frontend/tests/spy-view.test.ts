import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import SpyView from '../src/lib/components/spy/SpyView.svelte';

const now = Date.now();
const events = [
  { id: 'e2', type: 'signup', page: '/pricing', visitorId: 'v1', country: 'DE', timestamp: new Date(now - 20_000).toISOString() },
  { id: 'e1', type: 'pageview', page: '/pricing', visitorId: 'v1', country: 'DE', timestamp: new Date(now - 90_000).toISOString(), referrer: 'news.ycombinator.com' }
];

describe('spy view', () => {
  afterEach(cleanup);

  it('streams real locations, event types, and relative times', () => {
    render(SpyView, { props: { events, visitors: [], streamState: 'live', onToggle: vi.fn(), onSelect: vi.fn() } });
    expect(screen.getByRole('status')).toHaveTextContent('Live');
    expect(document.body).toHaveTextContent('Germany');
    expect(document.body).toHaveTextContent('from news.ycombinator.com');
    expect(document.body).toHaveTextContent('2m ago');
  });

  it('counts only page views as top pages and selects visitors from the stream', async () => {
    const onSelect = vi.fn();
    render(SpyView, { props: { events, visitors: [], streamState: 'live', onToggle: vi.fn(), onSelect } });
    const topPages = screen.getByRole('heading', { name: 'Top pages' }).closest('section')!;
    expect(topPages).toHaveTextContent(/\/pricing\s*1$/);
    await fireEvent.click(screen.getAllByRole('button', { name: /signup/ })[0]);
    expect(onSelect).toHaveBeenCalledWith('v1');
  });

  it('computes live totals from the full time window, not the capped feed', () => {
    const windowEvents = Array.from({ length: 150 }, (_, index) => ({
      id: `w${index}`,
      type: 'pageview',
      page: '/',
      visitorId: `v${index % 40}`,
      country: 'US',
      timestamp: new Date(now - index * 5_000).toISOString()
    }));
    render(SpyView, {
      props: { events: windowEvents.slice(0, 100), windowEvents, visitors: [], streamState: 'live', onToggle: vi.fn(), onSelect: vi.fn() }
    });
    const totals = screen.getByRole('region', { name: 'Live totals' });
    expect(totals).toHaveTextContent(/Last 30 minutes\s*150/);
    // 40 distinct visitors appear within the last five minutes of the window.
    expect(totals).toHaveTextContent(/Active now\s*40/);
    expect(screen.getByText('100 events')).toBeInTheDocument();
  });

  it('links page paths to the live site in a new tab from top pages and the stream', () => {
    render(SpyView, {
      props: { events, visitors: [], streamState: 'live', pageOrigin: 'https://shop.example.com', onToggle: vi.fn(), onSelect: vi.fn() }
    });
    const links = screen.getAllByRole('link', { name: 'Open /pricing in a new tab' });
    expect(links).toHaveLength(3); // top pages + two stream rows
    for (const link of links) {
      expect(link).toHaveAttribute('href', 'https://shop.example.com/pricing');
      expect(link).toHaveAttribute('target', '_blank');
      expect(link).toHaveAttribute('rel', 'noopener noreferrer');
    }
  });

  it('omits page links without a site origin', () => {
    render(SpyView, { props: { events, visitors: [], streamState: 'live', onToggle: vi.fn(), onSelect: vi.fn() } });
    expect(screen.queryByRole('link')).toBeNull();
  });
});
