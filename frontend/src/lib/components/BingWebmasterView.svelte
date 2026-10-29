<script lang="ts">
  import { ArrowUpRight, ExternalLink, Eye, MousePointerClick, Percent, RefreshCw, Search, Settings } from '@lucide/svelte';
  import type { BingDimension, BingStatus, SearchConsoleRow } from '$lib/api';
  import { compactNumber } from '$lib/ui';

  let {
    status,
    reports,
    settingsHref,
    onSync,
    syncing = false
  }: {
    status: BingStatus | null;
    reports: Record<BingDimension, SearchConsoleRow[]>;
    settingsHref: string;
    onSync: () => void;
    syncing?: boolean;
  } = $props();

  const ready = $derived(Boolean(status?.configured && status.connected && status.siteUrl));
  // Daily totals carry no position, so the average comes from the weekly query buckets.
  const totals = $derived.by(() => {
    const clicks = reports.date.reduce((sum, row) => sum + row.clicks, 0);
    const impressions = reports.date.reduce((sum, row) => sum + row.impressions, 0);
    const ranked = reports.query.filter((row) => row.position > 0);
    const rankedImpressions = ranked.reduce((sum, row) => sum + row.impressions, 0);
    return {
      clicks,
      impressions,
      ctr: impressions ? clicks / impressions : 0,
      position: rankedImpressions
        ? ranked.reduce((sum, row) => sum + row.position * row.impressions, 0) / rankedImpressions
        : 0
    };
  });
  const hasData = $derived(reports.date.length > 0 || reports.query.length > 0 || reports.page.length > 0);
  const percent = (value: number) => `${(value * 100).toFixed(1)}%`;
  const whole = (value: number) => Math.round(value).toLocaleString();
  // Page rows are full URLs; show the path and keep the URL for the link.
  const pagePath = (url: string) => {
    try {
      const parsed = new URL(url);
      return `${parsed.pathname}${parsed.search}`;
    } catch {
      return url;
    }
  };

  const tables: Array<{ dimension: Exclude<BingDimension, 'date'>; title: string; label: string }> = [
    { dimension: 'query', title: 'Top queries', label: 'Query' },
    { dimension: 'page', title: 'Top pages', label: 'Page' }
  ];
</script>

<section class="page-head">
  <div>
    <p class="eyebrow">Organic search</p>
    <h2>Bing Webmaster Tools</h2>
    <p class="muted">
      {#if ready}
        {status?.siteUrl} · last synced {status?.lastSyncedAt
          ? new Date(status.lastSyncedAt).toLocaleString()
          : 'never'}
      {:else}
        Queries and pages from Bing, which also powers DuckDuckGo, Yahoo, and Ecosia results.
      {/if}
    </p>
  </div>
  {#if ready}
    <button class="secondary button" onclick={onSync} disabled={syncing}
      ><RefreshCw size={15} aria-hidden="true" />{syncing ? 'Syncing…' : 'Sync now'}</button
    >
  {/if}
</section>

{#if !status}
  <section class="panel empty large" aria-busy="true">
    <Search size={26} aria-hidden="true" />
    <p class="muted">Loading Bing Webmaster Tools…</p>
  </section>
{:else if !ready}
  <section class="panel empty large">
    <Search size={26} aria-hidden="true" />
    <h2>
      {#if !status?.configured}
        Bing Webmaster Tools isn’t configured on this deployment
      {:else if !status.connected}
        Connect Bing Webmaster Tools
      {:else}
        Verify this site in Bing Webmaster Tools
      {/if}
    </h2>
    <p class="muted">
      {#if !status?.configured}
        An administrator needs to set an integration encryption key before sites can connect.
      {:else if !status.connected}
        Paste an API key from Bing Webmaster Tools to see the searches that bring visitors here.
      {:else}
        {status.lastError ?? 'No verified Bing site matches this site yet.'}
      {/if}
    </p>
    {#if status?.configured}
      <a class="primary button" href={settingsHref}><Settings size={15} aria-hidden="true" />Set up in settings</a>
    {/if}
  </section>
{:else if !hasData}
  <section class="panel empty large">
    <Search size={26} aria-hidden="true" />
    <h2>No Bing data for this period</h2>
    <p class="muted">
      {status?.lastError ?? 'Sync to fetch the latest data from Bing. New sites can take a few days to report.'}
    </p>
  </section>
{:else}
  <section class="portfolio-summary" aria-label="Bing search performance summary">
    <div>
      <span class="label"><MousePointerClick size={15} aria-hidden="true" /> Clicks</span>
      <strong title={whole(totals.clicks)}>{whole(totals.clicks)}</strong>
      <small>From Bing and its partners</small>
    </div>
    <div>
      <span class="label"><Eye size={15} aria-hidden="true" /> Impressions</span>
      <strong title={whole(totals.impressions)}>{whole(totals.impressions)}</strong>
      <small>Times shown in results</small>
    </div>
    <div>
      <span class="label"><Percent size={15} aria-hidden="true" /> CTR</span>
      <strong>{percent(totals.ctr)}</strong>
      <small>Clicks per impression</small>
    </div>
    <div>
      <span class="label"><ArrowUpRight size={15} aria-hidden="true" /> Avg. position</span>
      <strong>{totals.position ? totals.position.toFixed(1) : '—'}</strong>
      <small>Top queries, by impressions</small>
    </div>
  </section>

  <p class="muted bing-note">Bing groups query and page data into weekly buckets, so short ranges can look uneven.</p>

  <div class="two-col search-console-tables">
    {#each tables as table (table.dimension)}
      {@const rows = reports[table.dimension].slice(0, 25)}
      <section class="panel">
        <div class="panel-head">
          <h2>{table.title}</h2>
          <span>{rows.length} {rows.length === 1 ? 'result' : 'results'}</span>
        </div>
        {#if rows.length}
          <div class="table-wrap">
            <table aria-label={table.title}>
              <thead>
                <tr>
                  <th scope="col">{table.label}</th>
                  <th scope="col" class="numeric">Clicks</th>
                  <th scope="col" class="numeric">Impr.</th>
                  <th scope="col" class="numeric">CTR</th>
                  <th scope="col" class="numeric">Pos.</th>
                </tr>
              </thead>
              <tbody>
                {#each rows as row (row.value)}
                  <tr>
                    <th scope="row" class="search-label">
                      {#if table.dimension === 'page'}
                        <a href={row.value} target="_blank" rel="noopener noreferrer" title={row.value}
                          >{pagePath(row.value)} <ExternalLink size={11} aria-hidden="true" /></a
                        >
                      {:else}
                        <span class="query-cell">
                          <span title={row.value}>{row.value}</span>
                          <a
                            class="engine-link"
                            href={`https://www.bing.com/search?q=${encodeURIComponent(row.value)}`}
                            target="_blank"
                            rel="noopener noreferrer"
                            aria-label={`Search Bing for “${row.value}”`}
                            title="Search Bing"><ExternalLink size={12} aria-hidden="true" /></a
                          >
                        </span>
                      {/if}
                    </th>
                    <td class="numeric"><strong>{compactNumber(row.clicks)}</strong></td>
                    <td class="numeric">{compactNumber(row.impressions)}</td>
                    <td class="numeric muted">{percent(row.ctr)}</td>
                    <td class="numeric muted">{row.position ? row.position.toFixed(1) : '—'}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {:else}
          <div class="empty"><p>No rows for this period.</p></div>
        {/if}
      </section>
    {/each}
  </div>
{/if}

<style>
  .bing-note {
    margin: 0 0 12px;
    font-size: 0.85rem;
  }
  .search-console-tables {
    row-gap: 16px;
  }
  .search-label {
    max-width: 280px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .search-label a {
    color: inherit;
    text-decoration: none;
  }
  .search-label a:hover {
    color: var(--accent);
  }
  /* Truncate the query, never the link beside it. */
  .query-cell {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .query-cell > span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .search-label .engine-link {
    flex: none;
    display: inline-flex;
    color: var(--muted);
  }
  .empty.large :global(svg) {
    justify-self: center;
    color: var(--accent);
  }
  .empty.large .button {
    justify-self: center;
    margin-top: 14px;
  }
</style>
