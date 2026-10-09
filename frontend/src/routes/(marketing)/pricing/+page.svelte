<script lang="ts">
  import { Bot, Check, GitFork, Minus, Zap } from '@lucide/svelte';
  import PricingCards from '$lib/components/marketing/PricingCards.svelte';
  import { pricingComparison, pricingFaqs } from '$lib/marketing/pricing';
</script>

<svelte:head>
  <title>Pricing · Slimlytics</title>
  <meta
    name="description"
    content="Slimlytics pricing: open-source analytics, free to self-host. Hosted plans from $0, with every feature on every plan and an MCP server for AI agents."
  />
</svelte:head>

<main id="main" class="mkt-page">
  <header class="mkt-page-head">
    <p class="eyebrow">Pricing</p>
    <h1>Every feature. Every plan.</h1>
    <p class="mkt-lead">
      Self-host the open-source product for free, or let us run it for you. Plans differ only in
      sites, page views, and support — nothing is held back. The Free plan never requires a card.
    </p>
  </header>

  <section class="mkt-plans" aria-labelledby="plans-title">
    <h2 id="plans-title" class="sr-only">Plans</h2>
    <PricingCards />
  </section>

  <section class="mkt-section" aria-labelledby="why-title">
    <div class="mkt-section-head">
      <p class="eyebrow">Why Slimlytics</p>
      <h2 id="why-title">Open, fast, and built for AI agents</h2>
    </div>
    <div class="why-grid">
      <article class="feature-card">
        <span class="feature-icon"><GitFork size={22} aria-hidden="true" /></span>
        <h3>Open source</h3>
        <p>Read every line, run it on your own servers, and never get locked in. Move between hosted and self-hosted whenever you like.</p>
      </article>
      <article class="feature-card">
        <span class="feature-icon"><Zap size={22} aria-hidden="true" /></span>
        <h3>Built in Rust</h3>
        <p>A small, fast API on PostgreSQL that runs comfortably on modest hardware — which is how we keep prices low.</p>
      </article>
      <article class="feature-card">
        <span class="feature-icon"><Bot size={22} aria-hidden="true" /></span>
        <h3>MCP server for AI agents</h3>
        <p>Claude Code, Codex, and Hermes can install tracking, configure sites, and answer analytics questions for you.</p>
      </article>
    </div>
  </section>

  <section class="mkt-section" aria-labelledby="compare-title">
    <div class="mkt-section-head">
      <p class="eyebrow">Compare</p>
      <h2 id="compare-title">Compare plans</h2>
    </div>
    <div class="compare-wrap">
      <table class="compare-table">
        <thead>
          <tr>
            <th scope="col">Capability</th>
            <th scope="col">Self-hosted</th>
            <th scope="col">Free</th>
            <th scope="col">Pro</th>
            <th scope="col">Business</th>
          </tr>
        </thead>
        <tbody>
          {#each pricingComparison as row}
            <tr>
              <th scope="row">{row.feature}</th>
              {#each [row.selfHosted, row.free, row.pro, row.business] as cell}
                <td class:text-cell={typeof cell === 'string'}
                  >{#if typeof cell === 'string'}{cell}{:else if cell}<Check
                      size={18}
                      aria-label="Included"
                      role="img"
                    />{:else}<Minus size={18} aria-label="Not included" role="img" />{/if}</td
                >
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </section>

  <section class="mkt-section mkt-section-alt" aria-labelledby="faq-title">
    <div class="mkt-section-head">
      <p class="eyebrow">FAQ</p>
      <h2 id="faq-title">Common questions</h2>
    </div>
    <div class="faq-list">
      {#each pricingFaqs as item}
        <details class="faq-item">
          <summary>{item.question}</summary>
          <p>{item.answer}</p>
        </details>
      {/each}
    </div>
  </section>

  <section class="mkt-section mkt-cta">
    <div class="mkt-cta-inner">
      <h2>Ready when you are</h2>
      <p>Spin up a free account and install the tracker in a few minutes.</p>
      <div class="mkt-hero-actions">
        <a class="primary" href="/register">Create free account</a>
        <a class="secondary" href="/docs">Self-host docs</a>
      </div>
    </div>
  </section>
</main>
