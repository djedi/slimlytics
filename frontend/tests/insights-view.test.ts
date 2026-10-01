import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import InsightsView from '../src/lib/components/insights/InsightsView.svelte';

const base = {
  attribution: [{ source: '(direct)', medium: '(none)', campaign: '(none)', visitors: 7, conversions: 10, revenue: 0 }],
  journeys: [{ steps: ['/', '/pricing'], sessions: 1, visitors: 1 }],
  anomalies: [],
  funnelReports: [],
  landingPages: [],
  exitPages: [],
  sources: [],
  content: [],
  aiReferrers: [],
  aiCrawlers: []
};

describe('insights view', () => {
  afterEach(cleanup);

  it('contains a failed section instead of blanking the page', async () => {
    const retry = vi.fn();
    render(InsightsView, { ...base, failed: new Set(['anomalies'] as const), retry });
    expect(screen.getByRole('alert')).toHaveTextContent('Couldn’t load anomalies.');
    expect(screen.getByRole('table', { name: 'First-touch attribution' })).toBeInTheDocument();
    screen.getByRole('button', { name: 'Retry' }).click();
    expect(retry).toHaveBeenCalledOnce();
  });

  it('reports conversions per visitor rather than an impossible rate', () => {
    render(InsightsView, { ...base, failed: new Set<'anomalies'>(), retry: () => {} });
    expect(screen.getAllByText('1.43')).not.toHaveLength(0);
    expect(document.body).not.toHaveTextContent('142.9%');
    expect(document.body).toHaveTextContent('1 session');
  });

  it('marks highlights unavailable instead of showing stale data from a failed section', () => {
    render(InsightsView, { ...base, failed: new Set(['attribution', 'anomalies'] as const), retry: () => {} });
    const highlights = screen.getByRole('region', { name: 'Insight highlights' });
    expect(highlights).not.toHaveTextContent('(direct)');
    expect(highlights).not.toHaveTextContent(/^.*Conversions10/);
    expect(highlights.textContent?.match(/Unavailable/g)).toHaveLength(3);
  });
});
