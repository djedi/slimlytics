<script lang="ts">
  import { ArrowRight, Settings } from '@lucide/svelte';
  import type { Site } from '$lib/api';
  import { appHref } from '$lib/app-routes';
  import { avatarHue, compactNumber, duration, trendPaths } from '$lib/ui';
  import SiteIcon from '../SiteIcon.svelte';
  import ChangeBadge from './ChangeBadge.svelte';

  let { site, days, apiBase }: { site: Site; days: number; apiBase?: string } = $props();

  const overview = $derived(site.overview);
  const trend = $derived(overview?.trend ?? []);
  const values = $derived(trend.map((point) => point.visitors));
  const paths = $derived(trendPaths(values, 300, 60));
  // No overview means the request failed: show that as unavailable, not as a quiet site.
  const unavailable = $derived(!overview);
  const quiet = $derived(!unavailable && overview!.visitors === 0 && values.every((value) => value === 0));
  const muted = $derived(quiet || unavailable);
  const peak = $derived(
    trend.reduce<{ date: string; visitors: number } | null>(
      (best, point) => (!best || point.visitors > best.visitors ? point : best),
      null
    )
  );
  const online = $derived(overview?.currentOnline ?? 0);
  const href = $derived(appHref(site.id, quiet ? 'settings' : 'overview', days));
  const chartLabel = $derived(
    peak && peak.visitors > 0
      ? `${site.name}: ${overview?.visitors ?? 0} visitors over the last ${days} days, peaking at ${peak.visitors} on ${new Date(peak.date).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })}`
      : `${site.name}: no visitors in the last ${days} days`
  );
</script>

<a class="site-card-v2" class:quiet {href} style={`--hue:${avatarHue(site.domain)}`}>
  <header>
    <SiteIcon {site} size={38} {apiBase} />
    <span class="name">
      <strong title={site.name}>{site.name}</strong>
      {#if site.name.toLowerCase() !== site.domain.toLowerCase()}<small>{site.domain}</small>{/if}
    </span>
    {#if online > 0}
      <span class="online active"><i aria-hidden="true"></i>{online} online</span>
    {/if}
  </header>

  <div class="headline">
    <strong>{unavailable ? '—' : compactNumber(overview?.visitors ?? 0)}</strong>
    <span>visitors</span>
    {#if !muted}<ChangeBadge change={overview?.change} />{/if}
  </div>

  {#if unavailable}
    <div class="empty-chart">
      <span>Stats unavailable</span>
      <span class="hint-muted">Couldn’t load this site’s overview. It will retry shortly.</span>
    </div>
  {:else if quiet}
    <div class="empty-chart">
      <span>No visits in the last {days} days</span>
      <span class="hint"><Settings size={13} aria-hidden="true" /> Check installation</span>
    </div>
  {:else}
    <svg class="chart" viewBox="0 0 300 60" preserveAspectRatio="none" role="img" aria-label={chartLabel}>
      <defs>
        <linearGradient id={`trend-${site.id}`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="var(--accent)" stop-opacity=".32" />
          <stop offset="1" stop-color="var(--accent)" stop-opacity="0" />
        </linearGradient>
      </defs>
      <path class="area" d={paths.area} fill={`url(#trend-${site.id})`} />
      <path class="line" d={paths.line} />
    </svg>
  {/if}

  <dl>
    <div><dt>Page views</dt><dd>{unavailable ? '—' : compactNumber(overview?.pageViews ?? 0)}</dd></div>
    <div><dt>Bounce rate</dt><dd>{muted ? '—' : `${Math.round(overview?.bounceRate ?? 0)}%`}</dd></div>
    <div><dt>Avg. visit</dt><dd>{muted ? '—' : duration(overview?.avgDuration ?? 0)}</dd></div>
  </dl>

  <span class="open" aria-hidden="true">{quiet ? 'Set up tracking' : 'Open dashboard'} <ArrowRight size={14} /></span>
</a>

<style>
  .site-card-v2 {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 20px 20px 16px;
    border: 1px solid var(--line);
    border-radius: 16px;
    background: var(--surface);
    box-shadow: var(--shadow);
    color: inherit;
    text-decoration: none;
    overflow: hidden;
    transition: transform 0.2s ease, border-color 0.2s ease, box-shadow 0.2s ease;
  }
  .site-card-v2::before {
    content: '';
    position: absolute;
    inset: 0 0 auto;
    height: 3px;
    background: linear-gradient(90deg, hsl(var(--hue) 70% 55%), var(--accent));
    opacity: 0;
    transition: opacity 0.2s ease;
  }
  .site-card-v2:hover,
  .site-card-v2:focus-visible {
    transform: translateY(-3px);
    border-color: color-mix(in srgb, var(--accent) 45%, var(--line));
    box-shadow: 0 18px 40px -18px rgba(0, 0, 0, 0.45);
  }
  .site-card-v2:hover::before,
  .site-card-v2:focus-visible::before {
    opacity: 1;
  }
  .site-card-v2:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  header {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .name {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }
  .name strong {
    font-size: 15px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .name small {
    color: var(--muted);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .online {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 9px;
    border-radius: 999px;
    border: 1px solid var(--line);
    color: var(--muted);
    font-size: 11px;
    font-weight: 600;
    white-space: nowrap;
  }
  .online i {
    position: relative;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--muted) 60%, transparent);
  }
  .online.active {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 35%, var(--line));
    background: color-mix(in srgb, var(--accent) 9%, transparent);
  }
  .online.active i {
    background: var(--accent);
  }
  .online.active i::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: var(--accent);
    animation: ping 1.8s ease-out infinite;
  }
  @keyframes ping {
    from { transform: scale(1); opacity: 0.7; }
    to { transform: scale(2.8); opacity: 0; }
  }
  .headline {
    display: flex;
    align-items: baseline;
    gap: 8px;
    flex-wrap: wrap;
  }
  .headline strong {
    font: 800 34px/1 var(--display);
    letter-spacing: -0.03em;
    font-variant-numeric: tabular-nums;
  }
  .headline > span:not(:last-child) {
    color: var(--muted);
    font-size: 13px;
  }
  .headline :global(.change) {
    align-self: center;
    margin-left: auto;
  }
  .chart {
    display: block;
    width: 100%;
    height: 64px;
    overflow: visible;
  }
  .line {
    fill: none;
    stroke: var(--accent);
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
    vector-effect: non-scaling-stroke;
  }
  .empty-chart {
    display: grid;
    place-content: center;
    gap: 4px;
    height: 64px;
    border: 1px dashed var(--line);
    border-radius: 10px;
    color: var(--muted);
    font-size: 13px;
    text-align: center;
  }
  .empty-chart .hint-muted {
    font-size: 12px;
  }
  .empty-chart .hint {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    color: var(--accent);
    font-weight: 600;
    font-size: 12px;
  }
  dl {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
    margin: 0;
    padding-top: 14px;
    border-top: 1px solid var(--line);
  }
  dt {
    color: var(--muted);
    font-size: 11px;
    margin-bottom: 3px;
  }
  dd {
    margin: 0;
    font: 700 15px var(--display);
    font-variant-numeric: tabular-nums;
  }
  .open {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
    transition: color 0.2s ease, gap 0.2s ease;
  }
  .site-card-v2:hover .open {
    color: var(--accent);
    gap: 8px;
  }
  @media (prefers-reduced-motion: reduce) {
    .site-card-v2,
    .site-card-v2:hover {
      transform: none;
    }
    .online.active i::after {
      animation: none;
    }
  }
</style>
