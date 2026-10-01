<script lang="ts">
  import { Eye, X, Zap } from '@lucide/svelte';
  import type { LiveEvent, Visitor } from '$lib/api';
  import { countryLabel, flagEmoji, relativeTime } from '$lib/ui';

  let {
    visitor,
    events,
    onClose
  }: { visitor: Visitor; events: LiveEvent[]; onClose: () => void } = $props();

  // Most recent first, as loaded on the page (the trail is limited to those events).
  const trail = $derived(events.filter((item) => item.visitorId === visitor.id).slice(0, 12));
  const place = $derived(
    [visitor.city, visitor.region].filter(Boolean).join(', ') || countryLabel(visitor.country)
  );

  function keydown(event: KeyboardEvent) {
    if (event.key === 'Escape') onClose();
  }
</script>

<svelte:window onkeydown={keydown} />

<button class="drawer-scrim" type="button" aria-label="Close visitor details" onclick={onClose}></button>
<aside class="visitor-drawer" aria-label="Visitor details">
  <button class="icon-button" onclick={onClose} aria-label="Close visitor details"><X /></button>
  <div class="visitor-badge" aria-hidden="true">{flagEmoji(visitor.country)}</div>
  <p class="eyebrow">Visitor</p>
  <h2>{place}</h2>
  {#if visitor.city || visitor.region}<p class="muted drawer-country">{countryLabel(visitor.country)}</p>{/if}
  <dl>
    <div><dt>Browser</dt><dd>{visitor.browser ?? 'Unknown'}</dd></div>
    <div><dt>Device</dt><dd>{visitor.device ?? 'Unknown'}</dd></div>
    <div><dt>Sessions</dt><dd>{visitor.sessions ?? 1}</dd></div>
    {#if visitor.lastSeen}
      <div>
        <dt>Last seen</dt>
        <dd title={new Date(visitor.lastSeen).toLocaleString()}>{relativeTime(visitor.lastSeen)}</dd>
      </div>
    {/if}
  </dl>
  <h3 class="drawer-heading">Recent activity</h3>
  {#if trail.length}
    <ol class="visitor-trail">
      {#each trail as item (item.id)}
        <li>
          <span class="event-icon" class:custom={item.type !== 'pageview'} aria-hidden="true">
            {#if item.type === 'pageview'}<Eye size={14} />{:else}<Zap size={14} />{/if}
          </span>
          <span><strong>{item.type === 'pageview' ? item.page : item.type}</strong><small>{relativeTime(item.timestamp)}</small></span>
        </li>
      {/each}
    </ol>
  {:else}
    <p class="muted">No recent events loaded for this visitor.</p>
  {/if}
  <p class="privacy-note">Visitor IDs are anonymous, site-scoped, and rotate — no cross-site profile exists.</p>
</aside>
