<script lang="ts">
  import { ArrowDown, ArrowRight, ArrowUp, ExternalLink, Minus } from '@lucide/svelte';
  import type { ReportRow } from '../api';
  import { formatChange, pageHref } from '../ui';

  let {
    title,
    rows,
    moreHref,
    pageOrigin,
    emptyText = 'No report data for this period.',
    emptyHint = 'Try a wider date range.'
  }: {
    title: string;
    rows: ReportRow[];
    moreHref?: string;
    /** Site origin (e.g. https://example.com); rows whose label is a path get an open-in-new-tab link. */
    pageOrigin?: string;
    emptyText?: string;
    emptyHint?: string;
  } =
    $props();

  // Only show columns the data actually has.
  const hasVisitors = $derived(rows.some((row) => row.visitors !== undefined));
  const hasShare = $derived(!hasVisitors && rows.some((row) => row.secondary));
  const hasChange = $derived(rows.some((row) => row.change !== undefined && row.change !== 0));
  const max = $derived(Math.max(1, ...rows.map((row) => row.value)));

</script>

<section class="panel report">
  <div class="panel-head">
    <h2>{title}</h2>
    {#if moreHref}
      <a class="report-more" href={moreHref}>View all <ArrowRight size={13} aria-hidden="true" /></a>
    {:else}
      <span>{rows.length} {rows.length === 1 ? 'result' : 'results'}</span>
    {/if}
  </div>
  {#if rows.length}
    <div class="table-wrap">
      <table aria-label={title}>
        <thead>
          <tr>
            <th scope="col">Name</th>
            {#if hasVisitors}<th scope="col" class="numeric">Visitors</th>{/if}
            {#if hasShare}<th scope="col" class="numeric">Share</th>{/if}
            <th scope="col" class="numeric">Views</th>
            {#if hasChange}<th scope="col"><span class="sr-only">Change</span></th>{/if}
          </tr>
        </thead>
        <tbody>
          {#each rows as row}
            {@const href = pageHref(row.label, pageOrigin)}
            <tr style={`--share:${row.value / max}`}>
              <th scope="row">
                <span class="report-cell">
                  <span class="report-label" title={row.label}>{row.label}</span>
                  {#if href}
                    <a class="report-open" {href} target="_blank" rel="noopener noreferrer" aria-label={`Open ${row.label} in a new tab`} title="Open in a new tab"
                      ><ExternalLink size={13} aria-hidden="true" /></a
                    >
                  {/if}
                </span>
              </th>
              {#if hasVisitors}<td class="numeric muted">{(row.visitors ?? 0).toLocaleString()}</td>{/if}
              {#if hasShare}<td class="numeric muted">{row.secondary ?? '—'}</td>{/if}
              <td class="numeric"><strong>{row.value.toLocaleString()}</strong></td>
              {#if hasChange}
                {@const change = formatChange(row.change)}
                <td class="report-change {change.tone}">
                  {#if change.tone === 'up'}<ArrowUp size={13} aria-hidden="true" />{:else if change.tone === 'down'}<ArrowDown size={13} aria-hidden="true" />{:else}<Minus size={13} aria-hidden="true" />{/if}<span class="sr-only">{change.tone === 'flat' ? 'No change' : `${change.tone === 'up' ? 'Up' : 'Down'} ${change.text}`}</span>
                </td>
              {/if}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <div class="empty"><p>{emptyText}</p><span>{emptyHint}</span></div>
  {/if}
</section>

<style>
  .report-more {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--accent);
    font-size: 12px;
    font-weight: 600;
    text-decoration: none;
  }
  .report-more:hover {
    gap: 7px;
  }
  table {
    border-collapse: separate;
    border-spacing: 0;
  }
  thead th {
    font-size: 10px;
  }
  tbody tr {
    position: relative;
  }
  /* Share bar: drawn behind the row, proportional to the row's views. */
  tbody th {
    position: relative;
    z-index: 0;
    max-width: 0;
    width: 60%;
  }
  tbody th::before {
    content: '';
    position: absolute;
    z-index: -1;
    left: 0;
    top: 5px;
    bottom: 5px;
    width: calc(var(--share) * 100%);
    min-width: 3px;
    border-radius: 6px;
    background: color-mix(in srgb, var(--accent) 13%, transparent);
  }
  .report-cell {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .report-open {
    flex: none;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 6px;
    color: var(--muted);
    opacity: 0.55;
    transition: opacity 0.15s ease, color 0.15s ease, background-color 0.15s ease;
  }
  tr:hover .report-open,
  .report-open:focus-visible {
    opacity: 1;
  }
  .report-open:hover,
  .report-open:focus-visible {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    outline: none;
  }
  .report-label {
    display: block;
    min-width: 0;
    padding-left: 8px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
  }
  td,
  th {
    font-size: 13px;
  }
  /* app.css sizes the last column for a change arrow; Views is often last here. */
  td:last-child {
    width: auto;
  }
  .numeric {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  thead th.numeric {
    text-align: right;
  }
  .report-change {
    width: 28px;
    text-align: right;
  }
  .report-change.up {
    color: var(--accent);
  }
  .report-change.down {
    color: var(--red);
  }
  .report-change.flat {
    color: var(--muted);
  }
</style>
