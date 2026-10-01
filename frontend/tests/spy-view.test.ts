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
});
