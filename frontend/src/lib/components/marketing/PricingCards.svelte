<script lang="ts">
  import { Check } from '@lucide/svelte';
  import { pricingPlans } from '$lib/marketing/pricing';

  let { compact = false }: { compact?: boolean } = $props();
</script>

<div class="pricing-grid" class:compact>
  {#each pricingPlans as plan}
    <article class="pricing-card" class:highlighted={plan.highlighted}>
      {#if plan.highlighted}<span class="pricing-badge">Most popular</span>{/if}
      <p class="eyebrow">{plan.name}</p>
      <h3>{plan.tagline}</h3>
      <p class="pricing-amount">
        <strong>{plan.price}</strong>
        <span>{plan.priceNote}</span>
      </p>
      {#if plan.annualNote}<p class="pricing-annual">{plan.annualNote}</p>{/if}
      <ul>
        {#each plan.features as feature}
          <li><Check size={15} aria-hidden="true" />{feature}</li>
        {/each}
      </ul>
      <a class={plan.highlighted ? 'primary wide' : 'secondary wide'} href={plan.ctaHref}
        >{plan.ctaLabel}</a
      >
    </article>
  {/each}
</div>
{#if !compact}
  <p class="pricing-disclaimer muted">
    Daily page views count human traffic across all of an account’s sites and reset at midnight
    UTC. Going over never stops collection; the dashboard just asks you to upgrade. The Free plan
    needs no card, paid plans check out securely through Stripe, and self-hosted always includes
    the full product.
  </p>
{/if}
