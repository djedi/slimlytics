<script module lang="ts">
  export type StreamState = 'connecting' | 'live' | 'reconnecting' | 'paused' | 'offline';
</script>

<script lang="ts">
  import { onMount } from 'svelte';
  import { Activity, ExternalLink, Eye, FileText, Pause, Play, Search, Users, Zap } from '@lucide/svelte';
  import type { LiveEvent, Visitor } from '$lib/api';
  import { activeVisitorCount, countryLabel, flagEmoji, minuteBuckets, pageHref, relativeTime } from '$lib/ui';

  let {
    events,
    windowEvents = events,
    visitors,
    streamState,
    filter = $bindable(''),
    pageOrigin,
    onToggle,
    onSelect
  }: {
    /** Display feed (capped). */
    events: LiveEvent[];
    /** Every event in the last 30 minutes, uncapped; live totals are computed from this. */
    windowEvents?: LiveEvent[];
    visitors: Visitor[];
    streamState: StreamState;
    filter?: string;
    /** Site origin (e.g. https://example.com); page paths get an open-in-new-tab link. */
    pageOrigin?: string;
    onToggle: () => void;
    onSelect: (visitorId: string) => void;
  } = $props();

  // Re-render relative times and the rolling windows without waiting for new events.
  let now = $state(Date.now());
  onMount(() => {
    const timer = window.setInterval(() => (now = Date.now()), 15_000);
    return () => window.clearInterval(timer);
  });

  const filtered = $derived.by(() => {
    const query = filter.trim().toLowerCase();
    if (!query) return events;
    return events.filter((item) =>
      `${item.page} ${countryLabel(item.country)} ${item.city ?? ''} ${item.type} ${item.referrer ?? ''}`
        .toLowerCase()
        .includes(query)
    );
  });
  const activeNow = $derived(activeVisitorCount(windowEvents, now, 5));
  const recent = $derived(windowEvents.filter((item) => now - new Date(item.timestamp).getTime() < 30 * 60000));
  const buckets = $derived(minuteBuckets(windowEvents, now, 30));
  const bucketMax = $derived(Math.max(1, ...buckets));

  const rank = <T,>(items: T[], key: (item: T) => string) => {
    const counts = new Map<string, number>();
    for (const item of items) counts.set(key(item), (counts.get(key(item)) ?? 0) + 1);
    return [...counts.entries()].sort((a, b) => b[1] - a[1]);
  };
  // Countries come from visitors in the selected period; pages from the last 30 minutes,
  // falling back to the loaded events when it has been quiet.
  const countries = $derived(rank(visitors, (visitor) => visitor.country || '').slice(0, 6));
  const countryMax = $derived(Math.max(1, ...countries.map(([, count]) => count)));
  const recentViews = $derived(recent.filter((item) => item.type === 'pageview'));
  const pagesWindow = $derived(recentViews.length ? recentViews : events.filter((item) => item.type === 'pageview'));
  const pages = $derived(rank(pagesWindow, (item) => item.page).slice(0, 5));
  const pageMax = $derived(Math.max(1, ...pages.map(([, count]) => count)));

  const statusLabel: Record<StreamState, string> = {
    connecting: 'Connecting…',
    live: 'Live',
    reconnecting: 'Reconnecting…',
    paused: 'Paused',
    offline: 'Not streaming'
  };
  const isPageview = (item: LiveEvent) => item.type === 'pageview';
</script>

{#snippet openLink(href: string, page: string)}
  <a class="page-open" {href} target="_blank" rel="noopener noreferrer" aria-label={`Open ${page} in a new tab`} title="Open in a new tab"
    ><ExternalLink size={13} aria-hidden="true" /></a
  >
{/snippet}

<section class="spy-toolbar">
  <div>
    <span class="stream-status {streamState}" role="status">
      <i aria-hidden="true"></i>{statusLabel[streamState]}
    </span>
    <span class="muted spy-caption">Events appear here the moment they’re recorded.</span>
  </div>
  <div>
    <label class="search"
      ><Search size={15} aria-hidden="true" /><span class="sr-only">Filter activity</span
      ><input bind:value={filter} placeholder="Filter pages, places, events…" /></label
    ><button class="secondary" onclick={onToggle}
      >{#if streamState === 'paused'}<Play size={15} aria-hidden="true" />Resume{:else}<Pause
          size={15}
          aria-hidden="true"
        />Pause{/if}</button
    >
  </div>
</section>

<div class="spy-layout">
  <div class="spy-main">
    <section class="portfolio-summary spy-stats" aria-label="Live totals">
      <div>
        <span class="label"><Users size={15} aria-hidden="true" /> Active now</span>
        <strong>{activeNow}</strong>
        <small>{activeNow === 1 ? 'visitor' : 'visitors'} in the last 5 minutes</small>
      </div>
      <div>
        <span class="label"><Activity size={15} aria-hidden="true" /> Last 30 minutes</span>
        <strong>{recent.length}</strong>
        <small>{recent.filter(isPageview).length} page views · {recent.length - recent.filter(isPageview).length} events</small>
      </div>
    </section>

    <section class="panel spy-activity">
      <div class="panel-head">
        <h2>Activity per minute</h2>
        <span>Last 30 minutes</span>
      </div>
      <div
        class="minute-bars"
        role="img"
        aria-label={`${recent.length} events in the last 30 minutes; busiest minute had ${Math.max(...buckets)}.`}
      >
        {#each buckets as count, index}
          <span
            class:idle={count === 0}
            style={`--h:${count / bucketMax}`}
            title={`${30 - index} min ago: ${count} ${count === 1 ? 'event' : 'events'}`}
          ></span>
        {/each}
      </div>
      <div class="minute-axis" aria-hidden="true"><span>30 min ago</span><span>now</span></div>
    </section>

    <div class="spy-split">
      <section class="panel">
        <div class="panel-head"><h2>Where they’re from</h2><span>Selected period</span></div>
        {#if countries.length}
          <ul class="rank-list">
            {#each countries as [code, count]}
              <li style={`--share:${count / countryMax}`}>
                <span class="flag" aria-hidden="true">{flagEmoji(code)}</span>
                <span class="rank-label">{countryLabel(code)}</span>
                <b>{count}</b>
              </li>
            {/each}
          </ul>
        {:else}
          <div class="empty"><p>No visitors yet.</p></div>
        {/if}
      </section>
      <section class="panel">
        <div class="panel-head">
          <h2>Top pages</h2><span>{recentViews.length ? 'Last 30 minutes' : 'Recent page views'}</span>
        </div>
        {#if pages.length}
          <ul class="rank-list">
            {#each pages as [page, count]}
              {@const href = pageHref(page, pageOrigin)}
              <li style={`--share:${count / pageMax}`}>
                <FileText size={14} aria-hidden="true" />
                <span class="rank-label" title={page}>{page}</span>
                {#if href}{@render openLink(href, page)}{/if}
                <b>{count}</b>
              </li>
            {/each}
          </ul>
        {:else}
          <div class="empty"><p>No page views yet.</p></div>
        {/if}
      </section>
    </div>
  </div>

  <section class="panel activity-feed spy-stream">
    <div class="panel-head">
      <h2>Visitor stream</h2>
      <span>{filtered.length} {filtered.length === 1 ? 'event' : 'events'}</span>
    </div>
    {#if filtered.length}
      <ol aria-live="polite" aria-relevant="additions">
        {#each filtered as item (item.id)}
          {@const href = pageHref(item.page, pageOrigin)}
          <li class:fresh={now - new Date(item.timestamp).getTime() < 60000}>
            <span class="event-icon" class:custom={!isPageview(item)} aria-hidden="true">
              {#if isPageview(item)}<Eye size={15} />{:else}<Zap size={15} />{/if}
            </span>
            <button onclick={() => item.visitorId && onSelect(item.visitorId)} disabled={!item.visitorId}>
              <strong>{isPageview(item) ? item.page : item.type}</strong>
              <small>
                <span aria-hidden="true">{flagEmoji(item.country)}</span>
                {item.city ? `${item.city}, ` : ''}{countryLabel(item.country)}
                {#if !isPageview(item)}· on {item.page}{/if}
                {#if item.referrer}· from {item.referrer}{/if}
              </small>
            </button>
            {#if href}{@render openLink(href, item.page)}{:else}<span></span>{/if}
            <time datetime={item.timestamp} title={new Date(item.timestamp).toLocaleString()}
              >{relativeTime(item.timestamp, now)}</time
            >
          </li>
        {/each}
      </ol>
    {:else}
      <div class="empty">
        <Activity size={22} aria-hidden="true" />
        <p>{filter ? 'No events match your filter.' : 'Waiting for visitors…'}</p>
        {#if !filter}<span>New page views and events will stream in here.</span>{/if}
      </div>
    {/if}
  </section>
</div>
