import { describe, expect, it } from 'vitest';
import { DEFAULT_RANGE, appHref, parseRange, parseView } from '../src/lib/app-routes';

describe('dashboard routes', () => {
  it('builds workspace, site, and view paths', () => {
    expect(appHref()).toBe('/app');
    expect(appHref('site-1')).toBe('/app/site-1');
    expect(appHref('site-1', 'overview')).toBe('/app/site-1');
    expect(appHref('site-1', 'spy')).toBe('/app/site-1/spy');
    expect(appHref('a b', 'operating-systems')).toBe('/app/a%20b/operating-systems');
  });

  it('keeps a non-default date range in the query string', () => {
    expect(appHref('site-1', 'pages', { period: 7, compare: 'previous' })).toBe('/app/site-1/pages?days=7');
    expect(appHref('site-1', 'pages', DEFAULT_RANGE)).toBe('/app/site-1/pages');
    expect(appHref(undefined, undefined, { period: 90, compare: 'previous' })).toBe('/app?days=90');
    expect(appHref('site-1', null, { period: 'today', compare: 'week' })).toBe('/app/site-1?days=today&compare=week');
  });

  it('parses views, treating a missing view as overview', () => {
    expect(parseView(undefined)).toBe('overview');
    expect(parseView('referrers')).toBe('referrers');
    expect(parseView('search')).toBe('search');
    expect(parseView('rollup')).toBeNull();
    expect(parseView('nope')).toBeNull();
  });

  it('accepts only the offered date ranges', () => {
    const parse = (query: string) => parseRange(new URLSearchParams(query));
    expect(parse('')).toEqual(DEFAULT_RANGE);
    expect(parse('days=7')).toEqual({ period: 7, compare: 'previous' });
    expect(parse('days=today')).toEqual({ period: 'today', compare: 'previous' });
    expect(parse('days=yesterday&compare=week')).toEqual({ period: 'yesterday', compare: 'week' });
    expect(parse('days=13')).toEqual(DEFAULT_RANGE);
    expect(parse('days=abc&compare=nope')).toEqual(DEFAULT_RANGE);
    // A week-earlier comparison is offered only for a single day.
    expect(parse('days=28&compare=week')).toEqual(DEFAULT_RANGE);
  });
});
