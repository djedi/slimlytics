import { describe, expect, it } from 'vitest';
import { avatarHue, compactNumber, duration, formatChange, portfolioTotals, trendPaths } from '../src/lib/ui';

describe('all-sites rollup helpers', () => {
  it('formats period-over-period change with a tone and sensible precision', () => {
    expect(formatChange(-44.4444444444444)).toEqual({ text: '44%', tone: 'down' });
    expect(formatChange(-28.94736842105263)).toEqual({ text: '29%', tone: 'down' });
    expect(formatChange(3.14159)).toEqual({ text: '3.1%', tone: 'up' });
    expect(formatChange(0)).toEqual({ text: '0%', tone: 'flat' });
    expect(formatChange(0.01)).toEqual({ text: '0%', tone: 'flat' });
    expect(formatChange(undefined)).toEqual({ text: '0%', tone: 'flat' });
  });

  it('abbreviates large counts', () => {
    expect(compactNumber(48)).toBe('48');
    expect(compactNumber(1234)).toBe('1.2K');
    expect(compactNumber(2_500_000)).toBe('2.5M');
  });

  it('draws trend paths from a zero baseline without overshooting', () => {
    const { line, area } = trendPaths([0, 10, 0], 100, 20);
    expect(line.startsWith('M0,20')).toBe(true);
    expect(area.endsWith('L100,20 L0,20 Z')).toBe(true);
    // Monotone smoothing keeps every control point inside the chart.
    const ys = [...line.matchAll(/,(-?[\d.]+)/g)].map((m) => Number(m[1]));
    expect(Math.min(...ys)).toBeGreaterThanOrEqual(0);
    expect(Math.max(...ys)).toBeLessThanOrEqual(20);
    expect(trendPaths([], 100, 20)).toEqual({ line: '', area: '' });
    expect(trendPaths([0, 0, 0], 100, 20).line).toBe('M0,20 C16.67,20 33.33,20 50,20 C66.67,20 83.33,20 100,20');
  });

  it('totals the portfolio from actual previous-period visitors', () => {
    const totals = portfolioTotals([
      { visitors: 10, pageViews: 77, currentOnline: 1, previousVisitors: 20 },
      { visitors: 30, pageViews: 48, currentOnline: 0, previousVisitors: 20 },
      undefined
    ]);
    expect(totals).toEqual({ visitors: 40, pageViews: 125, online: 1, change: 0 });
    // A site that dropped to zero keeps its previous visitors: (100 - 200) / 200 = -50%.
    expect(
      portfolioTotals([
        { visitors: 0, pageViews: 0, currentOnline: 0, previousVisitors: 100 },
        { visitors: 100, pageViews: 100, currentOnline: 0, previousVisitors: 100 }
      ]).change
    ).toBe(-50);
    expect(portfolioTotals([{ visitors: 5, pageViews: 5, currentOnline: 0, previousVisitors: 0 }]).change).toBe(0);
  });

  it('gives each domain a stable avatar hue', () => {
    expect(avatarHue('toodue.com')).toBe(avatarHue('toodue.com'));
    expect(avatarHue('toodue.com')).not.toBe(avatarHue('slimlytics.com'));
    expect(avatarHue('x')).toBeGreaterThanOrEqual(0);
    expect(avatarHue('x')).toBeLessThan(360);
  });

  it('formats visit duration without empty minute units', () => {
    expect(duration(0)).toBe('0s');
    expect(duration(45.4)).toBe('45s');
    expect(duration(60)).toBe('1m');
    expect(duration(125)).toBe('2m 5s');
    expect(duration(3725)).toBe('1h 2m');
  });
});
