<script lang="ts">
  import { ArrowUpRight, ExternalLink, Eye, MousePointerClick, Percent, RefreshCw, Search, Settings } from '@lucide/svelte';
  import type { SearchConsoleDimension, SearchConsoleRow, SearchConsoleStatus } from '$lib/api';
  import { compactNumber } from '$lib/ui';

  let {
    status,
    reports,
    settingsHref,
    onSync,
    syncing = false
  }: {
    status: SearchConsoleStatus | null;
    reports: Record<SearchConsoleDimension, SearchConsoleRow[]>;
    settingsHref: string;
    onSync: () => void;
    syncing?: boolean;
  } = $props();

  const ready = $derived(Boolean(status?.configured && status.connected && status.propertyUrl));
  const totals = $derived.by(() => {
    const sum = reports.date.reduce(
      (acc, row) => ({
        clicks: acc.clicks + row.clicks,
        impressions: acc.impressions + row.impressions,
        weightedPosition: acc.weightedPosition + row.position * row.impressions
      }),
      { clicks: 0, impressions: 0, weightedPosition: 0 }
    );
    return {
      clicks: sum.clicks,
      impressions: sum.impressions,
      ctr: sum.impressions ? sum.clicks / sum.impressions : 0,
      position: sum.impressions ? sum.weightedPosition / sum.impressions : 0
    };
  });
  const hasData = $derived(reports.date.length > 0 || reports.query.length > 0);
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
  const deviceName = (value: string) => value.charAt(0) + value.slice(1).toLowerCase();

  const tables: Array<{ dimension: Exclude<SearchConsoleDimension, 'date'>; title: string; label: string; limit: number }> = [
    { dimension: 'query', title: 'Top queries', label: 'Query', limit: 25 },
    { dimension: 'page', title: 'Top pages', label: 'Page', limit: 25 },
    { dimension: 'country', title: 'Countries', label: 'Country', limit: 10 },
    { dimension: 'device', title: 'Devices', label: 'Device', limit: 10 }
  ];
</script>

<section class="page-head">
  <div>
    <p class="eyebrow">Organic search</p>
    <h2>Google Search Console</h2>
    <p class="muted">
      {#if ready}
        {status?.propertyUrl} · last synced {status?.lastSyncedAt
          ? new Date(status.lastSyncedAt).toLocaleString()
          : 'never'}
      {:else}
        Queries, pages, and positions from Google Search for this site.
      {/if}
    </p>
  </div>
  {#if ready}
    <button class="secondary button" onclick={onSync} disabled={syncing}
      ><RefreshCw size={15} aria-hidden="true" />{syncing ? 'Syncing…' : 'Sync now'}</button
    >
  {/if}
</section>

{#if !ready}
  <section class="panel empty large">
    <Search size={26} aria-hidden="true" />
    <h2>
      {#if !status?.configured}
        Search Console isn’t configured on this deployment
      {:else if !status.connected}
        Connect Google Search Console
      {:else}
        Add this site to Search Console
      {/if}
    </h2>
    <p class="muted">
      {#if !status?.configured}
        An administrator needs to add Google OAuth credentials before sites can connect.
      {:else if !status.connected}
        Connect your Google account to see the searches that bring visitors here.
      {:else}
        {status.lastError ?? 'No verified property matches this site yet.'} Slimlytics can add and verify it for you.
      {/if}
    </p>
    {#if status?.configured}
      <a class="primary button" href={settingsHref}><Settings size={15} aria-hidden="true" />Set up in settings</a>
    {/if}
  </section>
{:else if !hasData}
  <section class="panel empty large">
    <Search size={26} aria-hidden="true" />
    <h2>No Search Console data for this period</h2>
    <p class="muted">
      {status?.lastError ?? 'Sync to fetch the latest data from Google. New properties can take a few days to report.'}
    </p>
  </section>
{:else}
  <section class="portfolio-summary" aria-label="Search performance summary">
    <div>
      <span class="label"><MousePointerClick size={15} aria-hidden="true" /> Clicks</span>
      <strong title={whole(totals.clicks)}>{whole(totals.clicks)}</strong>
      <small>From Google Search</small>
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
      <small>Weighted by impressions</small>
    </div>
  </section>

  <div class="two-col search-console-tables">
    {#each tables as table (table.dimension)}
      {@const rows = reports[table.dimension].slice(0, table.limit)}
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
                      {:else if table.dimension === 'device'}
                        {deviceName(row.value)}
                      {:else if table.dimension === 'country'}
                        {row.value.toUpperCase()}
                      {:else}
                        <span title={row.value}>{row.value}</span>
                      {/if}
                    </th>
                    <td class="numeric"><strong>{compactNumber(row.clicks)}</strong></td>
                    <td class="numeric">{compactNumber(row.impressions)}</td>
                    <td class="numeric muted">{percent(row.ctr)}</td>
                    <td class="numeric muted">{row.position.toFixed(1)}</td>
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
  .empty.large :global(svg) {
    justify-self: center;
    color: var(--accent);
  }
  .empty.large .button {
    justify-self: center;
    margin-top: 14px;
  }
</style>
