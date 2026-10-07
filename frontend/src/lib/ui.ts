export type Theme = 'light' | 'dark' | 'system';

export function sparklinePoints(values: number[], width: number, height: number): string {
  if (!values.length) return '';
  const min = Math.min(...values);
  const max = Math.max(...values);
  const range = max - min;
  return values.map((value, index) => {
    const x = values.length === 1 ? width / 2 : (index / (values.length - 1)) * width;
    const y = range === 0 ? height / 2 : height - ((value - min) / range) * height;
    return `${Number(x.toFixed(2))},${Number(y.toFixed(2))}`;
  }).join(' ');
}

export function applyTheme(theme: Theme): void {
  if (theme === 'system') delete document.documentElement.dataset.theme;
  else document.documentElement.dataset.theme = theme;
  document.documentElement.style.colorScheme = theme === 'system' ? 'light dark' : theme;
}

export function duration(seconds: number): string {
  const total = Math.max(0, Math.round(seconds));
  if (total < 60) return `${total}s`;
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  if (hours) return minutes ? `${hours}h ${minutes}m` : `${hours}h`;
  const rest = total % 60;
  return rest ? `${minutes}m ${rest}s` : `${minutes}m`;
}

export type ChangeTone = 'up' | 'down' | 'flat';

/** Period-over-period change for display: whole percents from 10%, one decimal below. */
export function formatChange(change: number | undefined): { text: string; tone: ChangeTone } {
  const value = change ?? 0;
  const magnitude = Math.abs(value);
  if (magnitude < 0.05) return { text: '0%', tone: 'flat' };
  const text = magnitude >= 10 ? `${Math.round(magnitude)}%` : `${Number(magnitude.toFixed(1))}%`;
  return { text, tone: value > 0 ? 'up' : 'down' };
}

const compact = new Intl.NumberFormat('en', { notation: 'compact', maximumFractionDigits: 1 });
export function compactNumber(value: number): string {
  return compact.format(value);
}

const round = (value: number) => Number(value.toFixed(2));

/**
 * Smooth line and filled-area paths for a trend, measured from a zero baseline. Uses monotone
 * cubic interpolation (Fritsch–Carlson) so curves never dip below zero or overshoot a peak.
 */
export function trendPaths(
  values: number[],
  width: number,
  height: number,
  scaleMax?: number
): { line: string; area: string } {
  if (!values.length) return { line: '', area: '' };
  // Pass scaleMax to draw several series against one shared axis.
  const max = scaleMax ?? Math.max(...values, 0);
  const ys = values.map((value) => (max > 0 ? height - (value / max) * height : height));
  if (values.length === 1) {
    const y = round(ys[0]);
    const line = `M0,${y} L${width},${y}`;
    return { line, area: `${line} L${width},${height} L0,${height} Z` };
  }
  const xs = values.map((_, index) => (index / (values.length - 1)) * width);
  const deltas = ys.slice(1).map((y, index) => (y - ys[index]) / (xs[index + 1] - xs[index]));
  const slopes = ys.map((_, index) => {
    if (index === 0) return deltas[0];
    if (index === ys.length - 1) return deltas[deltas.length - 1];
    const [before, after] = [deltas[index - 1], deltas[index]];
    return before * after <= 0 ? 0 : (before + after) / 2;
  });
  deltas.forEach((delta, index) => {
    if (delta === 0) {
      slopes[index] = 0;
      slopes[index + 1] = 0;
      return;
    }
    const a = slopes[index] / delta;
    const b = slopes[index + 1] / delta;
    const sum = a * a + b * b;
    if (sum > 9) {
      const scale = 3 / Math.sqrt(sum);
      slopes[index] = scale * a * delta;
      slopes[index + 1] = scale * b * delta;
    }
  });
  let line = `M${round(xs[0])},${round(ys[0])}`;
  for (let index = 0; index < xs.length - 1; index++) {
    const step = (xs[index + 1] - xs[index]) / 3;
    line +=
      ` C${round(xs[index] + step)},${round(ys[index] + slopes[index] * step)}` +
      ` ${round(xs[index + 1] - step)},${round(ys[index + 1] - slopes[index + 1] * step)}` +
      ` ${round(xs[index + 1])},${round(ys[index + 1])}`;
  }
  return { line, area: `${line} L${width},${height} L0,${height} Z` };
}

type OverviewTotalsInput =
  | { visitors: number; pageViews: number; currentOnline: number; previousVisitors?: number }
  | undefined;

/** Workspace totals; combined change compares summed current and previous-period visitors. */
export function portfolioTotals(overviews: OverviewTotalsInput[]) {
  let visitors = 0;
  let pageViews = 0;
  let online = 0;
  let previous = 0;
  for (const overview of overviews) {
    if (!overview) continue;
    visitors += overview.visitors;
    pageViews += overview.pageViews;
    online += overview.currentOnline;
    previous += overview.previousVisitors ?? 0;
  }
  const change = previous > 0 ? Number((((visitors - previous) / previous) * 100).toFixed(1)) : 0;
  return { visitors, pageViews, online, change: Object.is(change, -0) ? 0 : change };
}

/** Stable per-domain hue so site avatars are distinguishable at a glance. */
export function avatarHue(domain: string): number {
  let hash = 0;
  for (const character of domain) hash = (hash * 31 + character.charCodeAt(0)) >>> 0;
  return hash % 360;
}

/** Whole-number y-axis ticks from zero, stepping by 1, 2, or 5 × 10ⁿ, covering `max`. */
export function niceTicks(max: number, target = 4): number[] {
  if (max <= 0) return [0, 1];
  const raw = max / target;
  const power = 10 ** Math.floor(Math.log10(raw));
  const fraction = raw / power;
  const step = Math.max(1, (fraction <= 1 ? 1 : fraction <= 2 ? 2 : fraction <= 5 ? 5 : 10) * power);
  const ticks: number[] = [];
  for (let tick = 0; tick < max + step; tick += step) ticks.push(tick);
  return ticks;
}

const validRegion = (code: string | null | undefined): code is string => !!code && /^[A-Za-z]{2}$/.test(code);

/** Flag emoji for an ISO 3166 country code, or a globe when the location is unknown. */
export function flagEmoji(code: string | null | undefined): string {
  if (!validRegion(code)) return '🌐';
  return String.fromCodePoint(...[...code.toUpperCase()].map((letter) => 0x1f1a5 + letter.charCodeAt(0)));
}

let regionNames: Intl.DisplayNames | undefined;
export function countryLabel(code: string | null | undefined): string {
  if (!validRegion(code)) return 'Unknown location';
  regionNames ??= new Intl.DisplayNames(['en'], { type: 'region' });
  return regionNames.of(code.toUpperCase()) ?? code.toUpperCase();
}

/**
 * Absolute URL for a page path on the site's own origin, for "open in a new tab" links.
 * Labels that aren't paths ("(not set)") or that resolve to another host ("//other.host") get null.
 */
export function pageHref(label: string, origin: string | undefined): string | null {
  if (!origin || !label.startsWith('/')) return null;
  try {
    const url = new URL(label, origin);
    return url.origin === new URL(origin).origin ? url.href : null;
  } catch {
    return null;
  }
}

/**
 * https URL for a referrer label that is a bare hostname ("news.ycombinator.com"), or null for
 * "(direct)", UTM source names like "newsletter", and anything carrying a path, port or scheme.
 */
export function referrerHref(label: string): string | null {
  const host = label.trim().toLowerCase();
  if (!/^(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}$/.test(host)) return null;
  return `https://${host}/`;
}

export function relativeTime(timestamp: string, now = Date.now()): string {
  const seconds = Math.max(0, (now - new Date(timestamp).getTime()) / 1000);
  if (seconds < 45) return 'just now';
  if (seconds < 3600) return `${Math.round(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return `${Math.floor(seconds / 86400)}d ago`;
}

/** Event counts for each of the last `minutes` minutes, oldest first. */
export function minuteBuckets(events: { timestamp: string }[], now: number, minutes: number): number[] {
  const buckets = new Array<number>(minutes).fill(0);
  for (const event of events) {
    const age = Math.floor((now - new Date(event.timestamp).getTime()) / 60000);
    if (age >= 0 && age < minutes) buckets[minutes - 1 - age] += 1;
  }
  return buckets;
}

/** Distinct visitors with an event in the last `minutes` minutes. */
export function activeVisitorCount(
  events: { visitorId?: string; timestamp: string }[],
  now: number,
  minutes: number
): number {
  const since = now - minutes * 60000;
  return new Set(
    events.filter((event) => event.visitorId && new Date(event.timestamp).getTime() >= since).map((event) => event.visitorId)
  ).size;
}

/** Links that toggle the tracker's "ignore my visits" flag in the browser that opens them. */
export function ignoreVisitsLinks(domain: string): { ignore: string; resume: string } | undefined {
  const raw = domain.trim();
  if (!raw) return undefined;
  try {
    const url = new URL(/^[a-z][a-z0-9+.-]*:\/\//i.test(raw) ? raw : `https://${raw}`);
    if (!/^https?:$/.test(url.protocol) || !url.hostname) return undefined;
    return { ignore: `${url.origin}/#slimlytics-ignore`, resume: `${url.origin}/#slimlytics-ignore=off` };
  } catch {
    return undefined;
  }
}
