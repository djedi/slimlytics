import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import ReportTable from '../src/lib/components/ReportTable.svelte';

describe('analytics components', () => {
  it('renders sortable report semantics and empty state', () => {
    const { rerender } = render(ReportTable, { title: 'Top pages', rows: [{ label: '/docs', value: 42 }] });
    expect(screen.getByRole('table', { name: 'Top pages' })).toBeInTheDocument();
    expect(screen.getByText('/docs')).toBeInTheDocument();
    rerender({ title: 'Top pages', rows: [] });
    expect(screen.getByText(/No report data/i)).toBeInTheDocument();
  });
});
