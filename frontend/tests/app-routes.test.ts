import { describe, expect, it } from 'vitest';
import { DEFAULT_DAYS, appHref, parseDays, parseView } from '../src/lib/app-routes';

describe('dashboard routes', () => {
  it('builds workspace, site, and view paths', () => {
    expect(appHref()).toBe('/app');
    expect(appHref('site-1')).toBe('/app/site-1');
    expect(appHref('site-1', 'overview')).toBe('/app/site-1');
    expect(appHref('site-1', 'spy')).toBe('/app/site-1/spy');
    expect(appHref('a b', 'operating-systems')).toBe('/app/a%20b/operating-systems');
  });

  it('keeps a non-default date range in the query string', () => {
    expect(appHref('site-1', 'pages', 7)).toBe('/app/site-1/pages?days=7');
    expect(appHref('site-1', 'pages', DEFAULT_DAYS)).toBe('/app/site-1/pages');
    expect(appHref(undefined, undefined, 90)).toBe('/app?days=90');
  });

  it('parses views, treating a missing view as overview', () => {
    expect(parseView(undefined)).toBe('overview');
    expect(parseView('referrers')).toBe('referrers');
    expect(parseView('rollup')).toBeNull();
    expect(parseView('nope')).toBeNull();
  });

  it('accepts only the offered date ranges', () => {
    expect(parseDays(null)).toBe(DEFAULT_DAYS);
    expect(parseDays('7')).toBe(7);
    expect(parseDays('90')).toBe(90);
    expect(parseDays('13')).toBe(DEFAULT_DAYS);
    expect(parseDays('abc')).toBe(DEFAULT_DAYS);
  });
});
