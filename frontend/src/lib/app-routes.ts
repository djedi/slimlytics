// URL scheme for the dashboard:
//   /app                    all sites
//   /app/{siteId}           a site's overview
//   /app/{siteId}/{view}    one panel, e.g. /app/{siteId}/spy
// A non-default date range is kept in ?days= (and ?compare=) so a copied link reproduces the view.

export const siteViews = [
  'overview',
  'insights',
  'spy',
  'pages',
  'referrers',
  'countries',
  'regions',
  'cities',
  'devices',
  'browsers',
  'operating-systems',
  'campaigns',
  'search',
  'visitors',
  'goals',
  'settings'
] as const;

export type SiteView = (typeof siteViews)[number];

/** A preset date range: a single local day, or the last N days ending today. */
export type Period = 'today' | 'yesterday' | 7 | 28 | 90;
/** What a range's metrics are compared against: the period just before it, or a week earlier. */
export type Compare = 'previous' | 'week';
export interface DateRange { period: Period; compare: Compare }

export const DEFAULT_RANGE: DateRange = { period: 28, compare: 'previous' };
export const periodOptions: readonly { value: Period; label: string }[] = [
  { value: 'today', label: 'Today' },
  { value: 'yesterday', label: 'Yesterday' },
  { value: 7, label: 'Last 7 days' },
  { value: 28, label: 'Last 28 days' },
  { value: 90, label: 'Last 90 days' }
];

export function isSingleDay(period: Period) { return period === 'today' || period === 'yesterday'; }
/** Days a period spans. */
export function periodDays(period: Period) { return isSingleDay(period) ? 1 : (period as number); }
/** The period as a phrase: "today", "yesterday", or "in the last 7 days". */
export function periodText(period: Period) { return isSingleDay(period) ? String(period) : `in the last ${period} days`; }
/** Choices for comparing a period; only a single day has a meaningful "same day last week". */
export function compareOptions(period: Period): readonly { value: Compare; label: string }[] {
  if (period === 'today') return [{ value: 'previous', label: 'vs. yesterday' }, { value: 'week', label: 'vs. same day last week' }];
  if (period === 'yesterday') return [{ value: 'previous', label: 'vs. the day before' }, { value: 'week', label: 'vs. same day last week' }];
  return [{ value: 'previous', label: 'vs. previous period' }];
}
/** What a range's change badges compare against, e.g. "vs. yesterday at this time". */
export function compareText({ period, compare }: DateRange) {
  const earlier = compare === 'week' ? 'same day last week' : period === 'today' ? 'yesterday' : period === 'yesterday' ? 'the day before' : 'previous period';
  return `vs. ${earlier}${period === 'today' ? ' at this time' : ''}`;
}
/** A stable key for caching data loaded for a range. */
export function rangeKey(range: DateRange) { return `${range.period}:${range.compare}`; }

export function appHref(siteId?: string | null, view?: SiteView | null, range: DateRange = DEFAULT_RANGE): string {
  let path = '/app';
  if (siteId) {
    path += `/${encodeURIComponent(siteId)}`;
    if (view && view !== 'overview') path += `/${view}`;
  }
  const query = new URLSearchParams();
  if (range.period !== DEFAULT_RANGE.period) query.set('days', String(range.period));
  if (range.compare !== DEFAULT_RANGE.compare) query.set('compare', range.compare);
  return query.size ? `${path}?${query}` : path;
}

/** The view for a path segment; a missing segment is the overview, an unknown one is null. */
export function parseView(segment: string | undefined): SiteView | null {
  if (!segment) return 'overview';
  return (siteViews as readonly string[]).includes(segment) ? (segment as SiteView) : null;
}

export function parseRange(params: URLSearchParams): DateRange {
  const value = params.get('days');
  const period = periodOptions.find((option) => String(option.value) === value)?.value ?? DEFAULT_RANGE.period;
  const compare = compareOptions(period).find((option) => option.value === params.get('compare'))?.value ?? 'previous';
  return { period, compare };
}
