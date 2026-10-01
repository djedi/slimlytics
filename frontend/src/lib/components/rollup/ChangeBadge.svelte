<script lang="ts">
  import { ArrowDownRight, ArrowUpRight, Minus } from '@lucide/svelte';
  import { formatChange } from '$lib/ui';

  let { change, label = 'vs previous period' }: { change: number | undefined; label?: string } = $props();
  const formatted = $derived(formatChange(change));
  const spoken = $derived(
    formatted.tone === 'flat'
      ? `No change ${label}`
      : `${formatted.tone === 'up' ? 'Up' : 'Down'} ${formatted.text} ${label}`
  );
</script>

<span class="change {formatted.tone}" title={spoken}>
  {#if formatted.tone === 'up'}<ArrowUpRight size={13} aria-hidden="true" />
  {:else if formatted.tone === 'down'}<ArrowDownRight size={13} aria-hidden="true" />
  {:else}<Minus size={13} aria-hidden="true" />{/if}
  <span aria-hidden="true">{formatted.text}</span>
  <span class="sr-only">{spoken}</span>
</span>

<style>
  .change {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 3px 8px 3px 6px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .up {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 13%, transparent);
  }
  .down {
    color: var(--red);
    background: color-mix(in srgb, var(--red) 13%, transparent);
  }
  .flat {
    color: var(--muted);
    background: color-mix(in srgb, var(--muted) 13%, transparent);
  }
</style>
