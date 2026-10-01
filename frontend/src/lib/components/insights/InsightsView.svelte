<script lang="ts">
  import {
    ArrowRight,
    Bot,
    CircleAlert,
    Filter,
    Goal as GoalIcon,
    Route,
    Sparkles,
    Target,
    TrendingDown,
    TrendingUp
  } from '@lucide/svelte';
  import type { Anomaly, Attribution, FunnelReport, Journey, ReportRow } from '$lib/api';
  import { compactNumber } from '$lib/ui';
  import ReportTable from '$lib/components/ReportTable.svelte';

  type Section = 'attribution' | 'journeys' | 'anomalies' | 'funnels' | 'reports';
  let {
    attribution,
    journeys,
    anomalies,
    funnelReports,
    landingPages,
    exitPages,
    sources,
    content,
    aiReferrers,
    aiCrawlers,
    failed,
    retry
  }: {
    attribution: Attribution[];
    journeys: Journey[];
    anomalies: Anomaly[];
    funnelReports: FunnelReport[];
    landingPages: ReportRow[];
    exitPages: ReportRow[];
    sources: ReportRow[];
    content: ReportRow[];
    aiReferrers: ReportRow[];
    aiCrawlers: ReportRow[];
    failed: Set<Section>;
    retry: () => void;
  } = $props();

  const plural = (value: number, noun: string) =>
    `${value.toLocaleString()} ${noun}${value === 1 ? '' : 's'}`;
  const money = (value: number) =>
    value.toLocaleString(undefined, { style: 'currency', currency: 'USD', maximumFractionDigits: value >= 1000 ? 0 : 2 });
  const day = (date: string) =>
    new Date(`${date}T00:00:00`).toLocaleDateString(undefined, { weekday: 'short', month: 'short', day: 'numeric' });

  const totals = $derived(
    attribution.reduce(
      (sum, row) => ({
        visitors: sum.visitors + row.visitors,
        conversions: sum.conversions + row.conversions,
        revenue: sum.revenue + row.revenue
      }),
      { visitors: 0, conversions: 0, revenue: 0 }
    )
  );
  const hasRevenue = $derived(totals.revenue > 0);
  const topChannel = $derived(attribution[0]);
  const channelMax = $derived(Math.max(1, ...attribution.map((row) => row.visitors)));
  const journeyMax = $derived(Math.max(1, ...journeys.map((row) => row.sessions)));
  const latestAnomaly = $derived([...anomalies].sort((a, b) => b.date.localeCompare(a.date))[0]);
  const channelName = (row: Attribution) =>
    row.medium === '(none)' ? row.source : `${row.source} / ${row.medium}`;
</script>

<section class="page-head">
  <div>
    <p class="eyebrow">Marketing intelligence</p>
    <h2>What is driving outcomes</h2>
    <p class="muted">Where visitors first came from, what they did, and what changed.</p>
  </div>
</section>

<section class="portfolio-summary insights-summary" aria-label="Insight highlights">
  <div>
    <span class="label"><Target size={15} aria-hidden="true" /> Conversions</span>
    <strong>{compactNumber(totals.conversions)}</strong>
    <small>
      {#if hasRevenue}{money(totals.revenue)} revenue{:else if totals.conversions}{(totals.conversions / Math.max(1, totals.visitors)).toFixed(2)} per visitor{:else}No goals completed yet{/if}
    </small>
  </div>
  <div>
    <span class="label"><Sparkles size={15} aria-hidden="true" /> Top channel</span>
    <strong class="text-value" title={topChannel ? channelName(topChannel) : undefined}
      >{topChannel ? channelName(topChannel) : '—'}</strong
    >
    <small
      >{topChannel
        ? `${Math.round((topChannel.visitors / Math.max(1, totals.visitors)) * 100)}% of visitors`
        : 'No traffic in this period'}</small
    >
  </div>
  <div>
    <span class="label"><CircleAlert size={15} aria-hidden="true" /> Anomalies</span>
    <strong>{anomalies.length}</strong>
    <small
      >{latestAnomaly
        ? `Latest ${day(latestAnomaly.date)}`
        : 'Traffic within its usual range'}</small
    >
  </div>
  <div>
    <span class="label"><Filter size={15} aria-hidden="true" /> Funnels</span>
    <strong>{funnelReports.length}</strong>
    <small
      >{funnelReports.length
        ? `${plural(funnelReports.reduce((sum, f) => sum + (f.steps.at(-1)?.visitors ?? 0), 0), 'completion')}`
        : 'None defined yet'}</small
    >
  </div>
</section>

{#snippet failure(label: string)}
  <div class="insight-error" role="alert">
    <CircleAlert size={16} aria-hidden="true" />
    <span>Couldn’t load {label}.</span>
    <button type="button" onclick={retry}>Retry</button>
  </div>
{/snippet}

<section class="panel insight-section attribution-panel">
  <div class="panel-head">
    <div>
      <h2>First-touch attribution</h2>
      <p class="panel-sub">The channel that first brought each visitor, and what they went on to do.</p>
    </div>
    <span>{plural(attribution.length, 'channel')}</span>
  </div>
  {#if failed.has('attribution')}
    {@render failure('attribution')}
  {:else if attribution.length}
    <div class="table-wrap">
      <table aria-label="First-touch attribution">
        <thead>
          <tr>
            <th scope="col">Channel</th>
            <th scope="col">Campaign</th>
            <th scope="col" class="numeric">Visitors</th>
            <th scope="col" class="numeric">Conversions</th>
            <th scope="col" class="numeric" title="Goal completions per first-touch visitor">Per visitor</th>
            {#if hasRevenue}<th scope="col" class="numeric">Revenue</th>{/if}
          </tr>
        </thead>
        <tbody>
          {#each attribution as row}
            <tr>
              <th scope="row">
                <span class="channel" style={`--share:${row.visitors / channelMax}`}>{channelName(row)}</span>
              </th>
              <td class="muted">{row.campaign === '(none)' ? '—' : row.campaign}</td>
              <td class="numeric">{row.visitors.toLocaleString()}</td>
              <td class="numeric"><strong>{row.conversions.toLocaleString()}</strong></td>
              <td class="numeric muted"
                >{row.visitors ? (row.conversions / row.visitors).toFixed(2) : '—'}</td
              >
              {#if hasRevenue}<td class="numeric">{money(row.revenue)}</td>{/if}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <div class="empty"><Sparkles size={22} aria-hidden="true" /><p>No attributed traffic in this period.</p></div>
  {/if}
</section>

<div class="two-col insight-section">
  <section class="panel">
    <div class="panel-head">
      <div>
        <h2>Common journeys</h2>
        <p class="panel-sub">Most frequent page sequences within a session.</p>
      </div>
    </div>
    {#if failed.has('journeys')}
      {@render failure('journeys')}
    {:else if journeys.length}
      <ol class="journey-list">
        {#each journeys.slice(0, 8) as journey}
          <li style={`--share:${journey.sessions / journeyMax}`}>
            <span class="journey-steps">
              {#each journey.steps as step, index}
                {#if index}<ArrowRight size={12} aria-hidden="true" />{/if}<code title={step}>{step}</code>
              {/each}
            </span>
            <span class="journey-count">{plural(journey.sessions, 'session')}</span>
          </li>
        {/each}
      </ol>
    {:else}
      <div class="empty"><Route size={22} aria-hidden="true" /><p>No journeys yet.</p><span>Journeys appear once visitors view more than one page.</span></div>
    {/if}
  </section>

  <section class="panel">
    <div class="panel-head">
      <div>
        <h2>Anomalies</h2>
        <p class="panel-sub">Days when page views moved 30%+ from the trailing baseline.</p>
      </div>
    </div>
    {#if failed.has('anomalies')}
      {@render failure('anomalies')}
    {:else if anomalies.length}
      <ul class="anomaly-list">
        {#each [...anomalies].sort((a, b) => b.date.localeCompare(a.date)).slice(0, 8) as anomaly}
          <li class={anomaly.direction}>
            <span class="anomaly-icon" aria-hidden="true">
              {#if anomaly.direction === 'up'}<TrendingUp size={16} />{:else}<TrendingDown size={16} />{/if}
            </span>
            <span>
              <strong>{day(anomaly.date)}</strong>
              <small>{plural(anomaly.value, 'page view')} vs ~{Math.round(anomaly.baseline * 10) / 10} typical</small>
            </span>
            <b>{anomaly.deviationPercent > 0 ? '+' : '−'}{Math.abs(Math.round(anomaly.deviationPercent))}%</b>
          </li>
        {/each}
      </ul>
    {:else}
      <div class="empty"><TrendingUp size={22} aria-hidden="true" /><p>No material anomalies.</p><span>Traffic stayed within its usual range.</span></div>
    {/if}
  </section>
</div>

<section class="panel insight-section">
  <div class="panel-head">
    <div>
      <h2>Funnels</h2>
      <p class="panel-sub">Unique visitors completing each step in order.</p>
    </div>
  </div>
  {#if failed.has('funnels')}
    {@render failure('funnels')}
  {:else if funnelReports.length}
    <div class="funnel-stack">
      {#each funnelReports as funnel}
        {@const first = Math.max(1, funnel.steps[0]?.visitors ?? 0)}
        <article>
          <h3>{funnel.name}</h3>
          <ol>
            {#each funnel.steps as step, index}
              <li style={`--share:${step.visitors / first}`}>
                <span class="funnel-label"><small>Step {index + 1}</small>{step.label}</span>
                <span class="funnel-bar" aria-hidden="true"><i></i></span>
                <span class="funnel-numbers"
                  ><b>{step.visitors.toLocaleString()}</b><small>{step.conversionRate.toFixed(1)}%</small></span
                >
              </li>
            {/each}
          </ol>
        </article>
      {/each}
    </div>
  {:else}
    <div class="empty">
      <GoalIcon size={22} aria-hidden="true" />
      <p>No funnels defined.</p>
      <span>Create one with <code>POST /api/sites/&#123;id&#125;/funnels</code> — see the <a href="/docs/api">API reference</a>.</span>
    </div>
  {/if}
</section>

{#if failed.has('reports')}
  <section class="panel insight-section">{@render failure('page and source reports')}</section>
{:else}
  <h3 class="insight-group">Entry &amp; exit</h3>
  <div class="two-col">
    <ReportTable title="Landing pages" rows={landingPages.slice(0, 10)} />
    <ReportTable title="Exit pages" rows={exitPages.slice(0, 10)} />
  </div>
  <h3 class="insight-group">Acquisition &amp; content</h3>
  <div class="two-col">
    <ReportTable title="Traffic sources" rows={sources.slice(0, 10)} />
    <ReportTable title="Content" rows={content.slice(0, 10)} />
  </div>
  <h3 class="insight-group"><Bot size={15} aria-hidden="true" /> AI traffic</h3>
  <div class="two-col">
    <ReportTable
      title="AI referrals"
      rows={aiReferrers.slice(0, 10)}
      emptyText="No visits from AI assistants yet."
      emptyHint="Referrals from ChatGPT, Perplexity, Claude, and others appear here."
    />
    <ReportTable
      title="AI crawlers"
      rows={aiCrawlers.slice(0, 10)}
      emptyText="No AI crawler requests recorded."
      emptyHint="Crawlers don’t run JavaScript; they’re captured through server log ingestion."
    />
  </div>
{/if}
