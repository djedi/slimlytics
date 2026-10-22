import { cleanup, render } from '@testing-library/svelte';
import { afterEach, beforeAll, describe, expect, it } from 'vitest';
import TrafficChart from '../src/lib/components/TrafficChart.svelte';
import { niceTicks, trendPaths } from '../src/lib/ui';

describe('traffic chart helpers', () => {
  it('rounds the y-axis to readable ticks that cover the data', () => {
    expect(niceTicks(0)).toEqual([0, 1]);
    expect(niceTicks(7)).toEqual([0, 2, 4, 6, 8]);
    expect(niceTicks(65)).toEqual([0, 20, 40, 60, 80]);
    expect(niceTicks(1234)).toEqual([0, 500, 1000, 1500]);
    expect(niceTicks(3)).toEqual([0, 1, 2, 3]);
  });

  it('can draw several series against one shared maximum', () => {
    // With a shared max of 20, a series peaking at 10 reaches only half the height.
    const { line } = trendPaths([0, 10], 100, 40, 20);
    expect(line.endsWith(' 100,20')).toBe(true);
  });
});

const hour = (h: number, visitors: number | null, previousVisitors: number) => ({
  hour: h,
  visitors,
  pageViews: visitors === null ? null : visitors * 2,
  previousVisitors,
  previousPageViews: previousVisitors * 2
});

// bind:clientWidth needs ResizeObserver, which jsdom lacks.
beforeAll(() => {
  globalThis.ResizeObserver ??= class { observe() {} unobserve() {} disconnect() {} } as unknown as typeof ResizeObserver;
});
afterEach(cleanup);

describe('TrafficChart', () => {
  it('charts a single day by hour beside the comparison day', () => {
    const hourly = [hour(0, 3, 1), hour(1, 5, 4), hour(2, null, 9)];
    const { getByRole } = render(TrafficChart, { trend: [], hourly, compareLabel: 'vs. yesterday' });
    const chart = getByRole('slider');

    expect(chart.getAttribute('aria-label')).toMatch(/^Hourly traffic .*8 visitors and 16 page views/);
    expect(chart.getAttribute('aria-label')).toContain('step through hours');
    // The latest hour has not happened yet: only the comparison day is announced for it.
    expect(chart.getAttribute('aria-valuetext')).toMatch(/: 9 visitors vs\. yesterday$/);
  });

  it('charts longer ranges by day', () => {
    const trend = [{ date: '2026-10-20', visitors: 2, pageViews: 3 }, { date: '2026-10-21', visitors: 4, pageViews: 6 }];
    const { getByRole } = render(TrafficChart, { trend });

    expect(getByRole('slider').getAttribute('aria-label')).toMatch(/^Daily traffic .*6 visitors and 9 page views/);
    expect(getByRole('slider').getAttribute('aria-valuetext')).toMatch(/4 visitors, 6 page views$/);
  });
});
