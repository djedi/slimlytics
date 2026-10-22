<script lang="ts">
  import type { HourPoint, TrendPoint } from '$lib/api';
  import { compactNumber, niceTicks, trendPaths } from '$lib/ui';

  let {
    trend,
    hourly,
    compareLabel = 'previous period'
  }: { trend: TrendPoint[]; hourly?: HourPoint[]; compareLabel?: string } = $props();

  // One chart point: a day, or for single-day ranges an hour beside the same hour of the
  // comparison day. Hours that have not happened yet have no visitors or page views.
  type Point = { label: string; long: string; visitors: number | null; pageViews: number | null; previous?: number };
  const hourText = (hour: number) => new Date(2000, 0, 1, hour).toLocaleTimeString(undefined, { hour: 'numeric' });
  const shortDate = (date: string, long = false) =>
    new Date(`${date}T00:00:00`).toLocaleDateString(undefined, long
      ? { weekday: 'short', month: 'short', day: 'numeric' }
      : { month: 'short', day: 'numeric' });
  const points = $derived<Point[]>(
    hourly
      ? hourly.map((h) => ({ label: hourText(h.hour), long: hourText(h.hour), visitors: h.visitors, pageViews: h.pageViews, previous: h.previousVisitors }))
      : trend.map((p) => ({ label: shortDate(p.date), long: shortDate(p.date, true), visitors: p.visitors, pageViews: p.pageViews }))
  );
  const unit = $derived(hourly ? 'hour' : 'day');

  const height = 240;
  const gutter = 40; // room for y-axis labels
  let width = $state(0);
  let active = $state<number | null>(null);

  const plotWidth = $derived(Math.max(0, width - gutter));
  const ticks = $derived(niceTicks(Math.max(0, ...points.flatMap((p) => [p.visitors ?? 0, p.pageViews ?? 0, p.previous ?? 0]))));
  const scaleMax = $derived(ticks[ticks.length - 1]);
  const xFor = (index: number) => (points.length < 2 ? plotWidth / 2 : (index / (points.length - 1)) * plotWidth);
  const yFor = (value: number) => height - (value / scaleMax) * height;
  // The lines stop at the last hour that has happened; the comparison line spans the whole day.
  const present = $derived(points.filter((p) => p.visitors !== null));
  const presentWidth = $derived(points.length < 2 ? plotWidth : xFor(Math.max(0, present.length - 1)));
  const visitors = $derived(trendPaths(present.map((p) => p.visitors ?? 0), presentWidth, height, scaleMax));
  const pageViews = $derived(trendPaths(present.map((p) => p.pageViews ?? 0), presentWidth, height, scaleMax));
  const previous = $derived(hourly ? trendPaths(points.map((p) => p.previous ?? 0), plotWidth, height, scaleMax) : null);

  // Evenly spaced labels, roughly one per 90px, always including the last point.
  const labelIndexes = $derived.by(() => {
    if (!points.length) return [];
    const slots = Math.max(2, Math.floor(plotWidth / 90));
    const step = Math.max(1, Math.ceil(points.length / slots));
    const indexes = points.map((_, index) => index).filter((index) => index % step === 0);
    const last = points.length - 1;
    if (last - indexes[indexes.length - 1] < step / 2) indexes.pop();
    return [...indexes, last];
  });

  const count = (value: number, noun: string) => `${value.toLocaleString()} ${noun}${value === 1 ? '' : 's'}`;
  const totals = $derived(points.reduce((sum, p) => ({ v: sum.v + (p.visitors ?? 0), pv: sum.pv + (p.pageViews ?? 0) }), { v: 0, pv: 0 }));
  const peak = $derived(points.reduce<Point | null>((best, p) => (!best || (p.visitors ?? 0) > (best.visitors ?? 0) ? p : best), null));
  const summary = $derived(
    points.length
      ? `${hourly ? 'Hourly' : 'Daily'} traffic from ${points[0].long} to ${points[points.length - 1].long}: ${count(totals.v, 'visitor')} and ${count(totals.pv, 'page view')}${peak && peak.visitors ? `, peaking at ${count(peak.visitors, 'visitor')} ${hourly ? 'at' : 'on'} ${peak.long}` : ''}. Use the arrow keys to step through ${unit}s.`
      : 'No traffic data for this period.'
  );

  function pick(event: PointerEvent) {
    if (!points.length || !plotWidth) return;
    const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const x = event.clientX - bounds.left - gutter;
    active = Math.min(points.length - 1, Math.max(0, Math.round((x / plotWidth) * (points.length - 1))));
  }
  function step(event: KeyboardEvent) {
    if (!points.length) return;
    const moves: Record<string, number> = { ArrowRight: 1, ArrowLeft: -1, Home: -Infinity, End: Infinity };
    if (!(event.key in moves)) return;
    event.preventDefault();
    const start = active ?? points.length - 1;
    active = Math.min(points.length - 1, Math.max(0, start + moves[event.key]));
  }
  const point = $derived(active === null ? null : points[active]);
  const pointText = (p: Point) =>
    [
      p.visitors === null ? null : `${count(p.visitors, 'visitor')}, ${count(p.pageViews ?? 0, 'page view')}`,
      p.previous === undefined ? null : `${count(p.previous, 'visitor')} ${compareLabel}`
    ].filter(Boolean).join('; ');
  // Keyboard users step through points; screen readers announce each one as the slider value.
  const pointLabel = $derived.by(() => {
    const p = points[active ?? points.length - 1];
    return p ? `${p.long}: ${pointText(p)}` : 'No data';
  });
</script>

<div
  class="traffic-chart"
  bind:clientWidth={width}
  role="slider"
  aria-label={summary}
  aria-valuemin={0}
  aria-valuemax={Math.max(0, points.length - 1)}
  aria-valuenow={active ?? Math.max(0, points.length - 1)}
  aria-valuetext={pointLabel}
  tabindex="0"
  onpointermove={pick}
  onpointerleave={() => (active = null)}
  onkeydown={step}
  onblur={() => (active = null)}
>
  <div class="y-axis" aria-hidden="true">
    {#each ticks as tick}
      <span style={`top:${yFor(tick)}px`}>{compactNumber(tick)}</span>
    {/each}
  </div>
  {#if plotWidth > 0}
    <svg width={plotWidth} {height} style={`left:${gutter}px`} aria-hidden="true">
      <defs>
        <linearGradient id="traffic-visitors-fill" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="var(--accent)" stop-opacity=".28" />
          <stop offset="1" stop-color="var(--accent)" stop-opacity="0" />
        </linearGradient>
      </defs>
      {#each ticks as tick}
        <line class="grid" x1="0" x2={plotWidth} y1={yFor(tick)} y2={yFor(tick)} />
      {/each}
      <path d={visitors.area} fill="url(#traffic-visitors-fill)" />
      {#if previous}<path class="previous" d={previous.line} />{/if}
      <path class="views" d={pageViews.line} />
      <path class="visitors" d={visitors.line} />
      {#if point && active !== null}
        <line class="cursor" x1={xFor(active)} x2={xFor(active)} y1="0" y2={height} />
        {#if point.visitors !== null}
          <circle class="dot views" cx={xFor(active)} cy={yFor(point.pageViews ?? 0)} r="4" />
          <circle class="dot visitors" cx={xFor(active)} cy={yFor(point.visitors)} r="5" />
        {/if}
      {/if}
    </svg>
  {/if}
  <div class="x-axis" aria-hidden="true">
    {#each labelIndexes as index}
      <span style={`left:${gutter + xFor(index)}px`}>{points[index].label}</span>
    {/each}
  </div>
  {#if point && active !== null}
    <div
      class="tooltip"
      class:flip={xFor(active) > plotWidth * 0.66}
      style={`left:${gutter + xFor(active)}px;top:${Math.max(8, yFor(Math.max(point.visitors ?? 0, point.pageViews ?? 0, point.previous ?? 0)) - 12)}px`}
      aria-hidden="true"
    >
      <strong>{point.long}</strong>
      {#if point.visitors !== null}
        <span><i class="visitors"></i>Visitors <b>{point.visitors.toLocaleString()}</b></span>
        <span><i class="views"></i>Page views <b>{(point.pageViews ?? 0).toLocaleString()}</b></span>
      {/if}
      {#if point.previous !== undefined}
        <span><i class="previous"></i>Visitors {compareLabel} <b>{point.previous.toLocaleString()}</b></span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .traffic-chart {
    position: relative;
    height: 272px;
    margin-top: 6px;
    outline: none;
    touch-action: pan-y;
  }
  .traffic-chart:focus-visible {
    border-radius: 10px;
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 60%, transparent);
  }
  svg {
    position: absolute;
    top: 0;
    overflow: visible;
  }
  .grid {
    stroke: var(--line);
    stroke-dasharray: 3 4;
  }
  path.visitors,
  path.views,
  path.previous {
    fill: none;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  path.visitors {
    stroke: var(--accent);
    stroke-width: 2.5;
  }
  path.views {
    stroke: var(--blue);
    stroke-width: 2;
    opacity: 0.85;
  }
  path.previous {
    stroke: var(--muted);
    stroke-width: 1.5;
    stroke-dasharray: 4 4;
  }
  .cursor {
    stroke: var(--muted);
    stroke-dasharray: 2 3;
  }
  .dot {
    stroke: var(--surface);
    stroke-width: 2;
  }
  .dot.visitors {
    fill: var(--accent);
  }
  .dot.views {
    fill: var(--blue);
  }
  .y-axis span {
    position: absolute;
    left: 0;
    width: 30px;
    transform: translateY(-50%);
    text-align: right;
    color: var(--muted);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }
  .x-axis {
    position: absolute;
    left: 0;
    right: 0;
    top: 252px;
  }
  .x-axis span {
    position: absolute;
    transform: translateX(-50%);
    color: var(--muted);
    font-size: 11px;
    white-space: nowrap;
  }
  .x-axis span:last-child {
    transform: translateX(-100%);
  }
  .tooltip {
    position: absolute;
    z-index: 2;
    display: grid;
    gap: 4px;
    min-width: 150px;
    padding: 10px 12px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: color-mix(in srgb, var(--surface) 94%, transparent);
    backdrop-filter: blur(8px);
    box-shadow: 0 12px 30px -12px rgba(0, 0, 0, 0.45);
    transform: translate(12px, -100%);
    pointer-events: none;
    font-size: 12px;
  }
  .tooltip.flip {
    transform: translate(calc(-100% - 12px), -100%);
  }
  .tooltip strong {
    font-size: 12px;
    margin-bottom: 2px;
  }
  .tooltip span {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
  }
  .tooltip b {
    margin-left: auto;
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .tooltip i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  .tooltip i.visitors {
    background: var(--accent);
  }
  .tooltip i.views {
    background: var(--blue);
  }
  .tooltip i.previous {
    background: var(--muted);
  }
</style>
