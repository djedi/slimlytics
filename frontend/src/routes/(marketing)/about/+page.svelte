<script lang="ts">
  import { Bot, GitFork, Heart, MessageCircle, ShieldCheck } from '@lucide/svelte';
</script>

<svelte:head>
  <title>About · Slimlytics</title>
  <meta
    name="description"
    content="Who builds Slimlytics, why it is open source under the AGPL, and how to reach us about bugs, security, billing, or anything else."
  />
</svelte:head>

<main id="main" class="mkt-page">
  <header class="mkt-page-head">
    <p class="eyebrow">About</p>
    <h1>Small analytics, built in the open</h1>
    <p class="mkt-lead">
      Slimlytics is independent, open-source web analytics built by Dustin Davis for his own sites
      first: real-time like Clicky, private by default, cheap to run, and simple enough that a
      coding agent can install it for you.
    </p>
  </header>

  <section class="mkt-section">
    <div class="privacy-grid">
      <article class="feature-card">
        <Heart size={22} aria-hidden="true" />
        <h2>Who builds it</h2>
        <p>
          Slimlytics is built by Dustin Davis and operated by Red Seam, Inc. We use Slimlytics on
          our own sites, including this one, so we feel every rough edge first.
        </p>
      </article>
      <article class="feature-card">
        <GitFork size={22} aria-hidden="true" />
        <h2>Open source, AGPL-3.0</h2>
        <p>
          The whole product is on
          <a href="https://github.com/djedi/slimlytics">GitHub</a> under the GNU Affero General
          Public License v3.0. Read every line, self-host it for free, or move between hosted and
          self-hosted whenever you like. The browser tracker and CLI are MIT-licensed so you can
          embed them anywhere.
        </p>
      </article>
      <article class="feature-card">
        <ShieldCheck size={22} aria-hidden="true" />
        <h2>Why the AGPL</h2>
        <p>
          The AGPL keeps Slimlytics open for everyone. Anyone may run or modify it, and anyone who
          offers a modified version as a hosted service has to share their changes too. The hosted
          plans pay for development; the code stays yours either way.
        </p>
      </article>
      <article class="feature-card">
        <Bot size={22} aria-hidden="true" />
        <h2>Built for agents</h2>
        <p>
          A remote MCP server with browser OAuth lets Claude Code, Codex, Hermes, or any MCP client
          create a site, wire up first-party tracking, verify it, and answer questions about your
          traffic.
        </p>
      </article>
    </div>
  </section>

  <section id="contact" class="mkt-section mkt-section-alt" aria-labelledby="contact-title">
    <div class="mkt-prose">
      <h2 id="contact-title"><MessageCircle size={22} aria-hidden="true" /> How to reach us</h2>
      <ul>
        <li>
          <strong>Email:</strong> <a href="mailto:dustin@davis.im">dustin@davis.im</a> for support,
          billing, privacy, and anything else.
        </li>
        <li>
          <strong>Bugs and feature requests:</strong>
          <a href="https://github.com/djedi/slimlytics/issues">open an issue on GitHub</a>.
        </li>
        <li>
          <strong>Security reports:</strong> please don’t open a public issue. Follow the
          <a href="https://github.com/djedi/slimlytics/blob/main/SECURITY.md">security policy</a> to
          report privately.
        </li>
        <li>
          <strong>Billing:</strong> paid plans are managed by Stripe. Upgrade, downgrade, or cancel
          anytime from your account’s plan card.
        </li>
        <li>
          <strong>Updates:</strong> follow
          <a href="https://x.com/DustinDavis">@DustinDavis on X</a> for what’s shipping.
        </li>
      </ul>
    </div>
  </section>

  <section class="mkt-section mkt-cta">
    <div class="mkt-cta-inner">
      <h2>Try it on your own site</h2>
      <p>Start on the free hosted plan, or self-host the same open-source product.</p>
      <div class="mkt-hero-actions">
        <a class="primary" href="/register">Create free account</a>
        <a class="secondary" href="https://github.com/djedi/slimlytics">View on GitHub</a>
      </div>
    </div>
  </section>
</main>
