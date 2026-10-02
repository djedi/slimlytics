<script lang="ts">
  import { ArrowUpRight, CreditCard, Sparkles } from '@lucide/svelte';
  import type { BillingStatus } from '$lib/api';

  let {
    status,
    busy = false,
    onCheckout,
    onPortal
  }: {
    status: BillingStatus;
    busy?: boolean;
    onCheckout: (plan: string, interval: 'month' | 'year') => void;
    onPortal: () => void;
  } = $props();

  let interval = $state<'month' | 'year'>('month');
  const plan = $derived(status.plan!);
  const usage = $derived(status.usage ?? { sites: 0, pageViewsToday: 0 });
  const comped = $derived(status.planSource === 'admin');
  const plans = $derived(status.plans ?? []);
  // Only plans above the current one, by daily volume, are offered as upgrades.
  const upgrades = $derived(
    comped || !status.checkoutAvailable
      ? []
      : plans.filter(
          (option) =>
            (option.intervals?.length ?? 0) > 0 &&
            option.id !== plan.id &&
            (option.dailyPageViews ?? Infinity) > (plan.dailyPageViews ?? Infinity)
        )
  );
  const ratio = (used: number, limit: number | null) => (limit ? used / limit : 0);
  const viewsRatio = $derived(ratio(usage.pageViewsToday, plan.dailyPageViews));
  const sitesRatio = $derived(ratio(usage.sites, plan.sites));
  const fmt = (value: number) => value.toLocaleString();
  const price = (cents: number, currency: string) =>
    (cents / 100).toLocaleString(undefined, {
      style: 'currency',
      currency: currency.toUpperCase(),
      minimumFractionDigits: cents % 100 ? 2 : 0
    });
</script>

<section class="plan-card" aria-label="Plan and usage">
  <header>
    <span class="plan-icon" aria-hidden="true"><Sparkles size={18} /></span>
    <div>
      <h2>{plan.name} plan</h2>
      <p>
        {#if comped}Complimentary — managed by an administrator
        {:else if status.planSource === 'stripe' && status.interval}Billed {status.interval === 'year' ? 'annually' : 'monthly'}{#if status.subscriptionStatus === 'past_due'} · <strong class="warn">payment past due</strong>{/if}
        {:else}Free — upgrade any time{/if}
      </p>
    </div>
    <!-- Always reachable with a Stripe customer, even when comped, so a running subscription can be cancelled. -->
    {#if status.hasBillingAccount}
      <button class="secondary compact" onclick={onPortal} disabled={busy}><CreditCard size={15} aria-hidden="true" />Manage billing</button>
    {/if}
  </header>

  <div class="meters">
    <div>
      <span class="meter-label">
        {#if plan.sites === null}Unlimited sites · {fmt(usage.sites)} in use{:else}{fmt(usage.sites)} of {fmt(plan.sites)} sites{/if}
      </span>
      {#if plan.sites !== null}<span class="meter" class:full={sitesRatio >= 1}><i style={`width:${Math.min(100, sitesRatio * 100)}%`}></i></span>{/if}
    </div>
    <div>
      <span class="meter-label">
        {#if plan.dailyPageViews === null}Unlimited page views · {fmt(usage.pageViewsToday)} today{:else}{fmt(usage.pageViewsToday)} of {fmt(plan.dailyPageViews)} page views today{/if}
      </span>
      {#if plan.dailyPageViews !== null}<span class="meter" class:full={viewsRatio >= 1} class:near={viewsRatio >= 0.8 && viewsRatio < 1}><i style={`width:${Math.min(100, viewsRatio * 100)}%`}></i></span>{/if}
    </div>
  </div>

  {#if viewsRatio >= 1}
    <p class="limit-note" role="status">You’re over today’s limit. Page views are still being collected — upgrade to keep within your plan.</p>
  {:else if viewsRatio >= 0.8}
    <p class="limit-note near" role="status">You’ve used {Math.round(viewsRatio * 100)}% of today’s page views.</p>
  {/if}

  {#if upgrades.length}
    <div class="upgrades">
      <div class="interval" role="group" aria-label="Billing interval">
        <button class:active={interval === 'month'} aria-pressed={interval === 'month'} onclick={() => (interval = 'month')}>Monthly</button>
        <button class:active={interval === 'year'} aria-pressed={interval === 'year'} onclick={() => (interval = 'year')}>Annual · save 33%</button>
      </div>
      {#each upgrades.filter((option) => option.intervals?.includes(interval)) as option}
        <button class="primary compact" disabled={busy} onclick={() => onCheckout(option.id, interval)}>
          Upgrade to {option.name} · {interval === 'year' ? `${price(option.annualPriceCents, option.currency)}/yr` : `${price(option.monthlyPriceCents, option.currency)}/mo`}
          <ArrowUpRight size={14} aria-hidden="true" />
        </button>
      {/each}
    </div>
  {/if}
</section>

<style>
  .plan-card {
    display: grid;
    gap: 14px;
    margin-bottom: 22px;
    padding: 18px 20px;
    border: 1px solid var(--line);
    border-radius: 16px;
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  header {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  header > div {
    flex: 1;
    min-width: 0;
  }
  header button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .plan-icon {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--accent) 13%, transparent);
    color: var(--accent);
  }
  h2 {
    margin: 0;
    font: 700 16px var(--display);
  }
  header p {
    margin: 2px 0 0;
    color: var(--muted);
    font-size: 13px;
  }
  .warn {
    color: var(--red);
  }
  .meters {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 16px;
  }
  .meter-label {
    display: block;
    margin-bottom: 6px;
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }
  .meter {
    display: block;
    height: 6px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--muted) 18%, transparent);
    overflow: hidden;
  }
  .meter i {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--accent);
  }
  .meter.near i {
    background: #d79a26;
  }
  .meter.full i {
    background: var(--red);
  }
  .limit-note {
    margin: 0;
    padding: 10px 12px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--red) 10%, transparent);
    color: var(--red);
    font-size: 13px;
    font-weight: 600;
  }
  .limit-note.near {
    background: color-mix(in srgb, #d79a26 12%, transparent);
    color: #b7791f;
  }
  .upgrades {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }
  .upgrades .primary {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .interval {
    display: inline-flex;
    padding: 3px;
    border: 1px solid var(--line);
    border-radius: 999px;
  }
  .interval button {
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: var(--muted);
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .interval button.active {
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    color: var(--accent);
  }
  @media (max-width: 640px) {
    .meters {
      grid-template-columns: 1fr;
    }
    header {
      flex-wrap: wrap;
    }
  }
</style>
