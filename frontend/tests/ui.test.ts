import { describe, expect, it } from 'vitest';
import { applyTheme, ignoreVisitsLinks, sparklinePoints } from '../src/lib/ui';

describe('UI helpers', () => {
  it('creates bounded SVG points including flat datasets', () => {
    expect(sparklinePoints([5, 5, 5], 100, 20)).toBe('0,10 50,10 100,10');
    expect(sparklinePoints([0, 10], 100, 20)).toBe('0,20 100,0');
  });

  it('applies accessible light, dark, and system theme modes', () => {
    applyTheme('dark');
    expect(document.documentElement.dataset.theme).toBe('dark');
    applyTheme('light');
    expect(document.documentElement.dataset.theme).toBe('light');
    applyTheme('system');
    expect(document.documentElement.dataset.theme).toBeUndefined();
  });
});

describe('ignoreVisitsLinks', () => {
  it('builds opt-out and opt-in links for a bare domain', () => {
    expect(ignoreVisitsLinks('example.com')).toEqual({
      ignore: 'https://example.com/#slimlytics-ignore',
      resume: 'https://example.com/#slimlytics-ignore=off'
    });
  });

  it('keeps an explicit scheme and drops paths, queries, and fragments', () => {
    expect(ignoreVisitsLinks('http://localhost:5173/app?x=1#y')?.ignore).toBe('http://localhost:5173/#slimlytics-ignore');
  });

  it('returns nothing for unusable domains', () => {
    expect(ignoreVisitsLinks('')).toBeUndefined();
    expect(ignoreVisitsLinks('javascript:alert(1)')).toBeUndefined();
  });
});
