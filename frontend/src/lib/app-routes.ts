// URL scheme for the dashboard:
//   /app                    all sites
//   /app/{siteId}           a site's overview
//   /app/{siteId}/{view}    one panel, e.g. /app/{siteId}/spy
// A non-default date range is kept in ?days= so a copied link reproduces the view.

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
  'visitors',
  'goals',
  'settings'
] as const;

export type SiteView = (typeof siteViews)[number];

export const DEFAULT_DAYS = 28;
export const dayOptions = [7, 28, 90] as const;

export function appHref(siteId?: string | null, view?: SiteView | null, days = DEFAULT_DAYS): string {
  let path = '/app';
  if (siteId) {
    path += `/${encodeURIComponent(siteId)}`;
    if (view && view !== 'overview') path += `/${view}`;
  }
  return days === DEFAULT_DAYS ? path : `${path}?days=${days}`;
}

/** The view for a path segment; a missing segment is the overview, an unknown one is null. */
export function parseView(segment: string | undefined): SiteView | null {
  if (!segment) return 'overview';
  return (siteViews as readonly string[]).includes(segment) ? (segment as SiteView) : null;
}

export function parseDays(value: string | null): number {
  const days = Number(value);
  return (dayOptions as readonly number[]).includes(days) ? days : DEFAULT_DAYS;
}
