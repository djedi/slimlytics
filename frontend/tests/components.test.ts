import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import ReportTable from '../src/lib/components/ReportTable.svelte';

describe('analytics components', () => {
  afterEach(cleanup);

  it('renders sortable report semantics and empty state', () => {
    const { rerender } = render(ReportTable, { title: 'Top pages', rows: [{ label: '/docs', value: 42 }] });
    expect(screen.getByRole('table', { name: 'Top pages' })).toBeInTheDocument();
    expect(screen.getByText('/docs')).toBeInTheDocument();
    rerender({ title: 'Top pages', rows: [] });
    expect(screen.getByText(/No report data/i)).toBeInTheDocument();
  });

  it('links page paths to the live site in a new tab, only on the site itself', () => {
    render(ReportTable, {
      title: 'Top pages',
      pageOrigin: 'https://shop.example.com',
      rows: [
        { label: '/about', value: 3 },
        { label: '(not set)', value: 2 },
        { label: '//evil.example/phish', value: 1 }
      ]
    });
    const link = screen.getByRole('link', { name: 'Open /about in a new tab' });
    expect(link).toHaveAttribute('href', 'https://shop.example.com/about');
    expect(link).toHaveAttribute('target', '_blank');
    expect(link).toHaveAttribute('rel', 'noopener noreferrer');
    expect(screen.queryByRole('link', { name: /not set/ })).toBeNull();
    expect(screen.queryByRole('link', { name: /evil/ })).toBeNull();
  });

  it('links bare referrer hostnames, not direct traffic or UTM source names', () => {
    render(ReportTable, {
      title: 'Top referrers',
      linkHosts: true,
      rows: [
        { label: 'news.ycombinator.com', value: 3 },
        { label: '(direct)', value: 2 },
        { label: 'newsletter', value: 1 }
      ]
    });
    const link = screen.getByRole('link', { name: 'Open news.ycombinator.com in a new tab' });
    expect(link).toHaveAttribute('href', 'https://news.ycombinator.com/');
    expect(link).toHaveAttribute('rel', 'noopener noreferrer');
    expect(screen.getAllByRole('link')).toHaveLength(1);
  });
});
