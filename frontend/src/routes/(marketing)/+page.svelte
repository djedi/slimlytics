<script lang="ts">
  import { env } from '$env/dynamic/public';
  import {
    Activity,
    ArrowRight,
    Bot,
    Check,
    CircleDot,
    Eye,
    FileText,
    Globe2,
    Goal,
    Radio,
    Server,
    Shield,
    ShieldCheck,
    TrendingUp,
    Users,
    Zap
  } from '@lucide/svelte';
  import PricingCards from '$lib/components/marketing/PricingCards.svelte';
  import { howItWorks, privacyHighlights, productFeatures } from '$lib/marketing/features';
  import { sparklinePoints } from '$lib/ui';

  const demo = env.PUBLIC_DEMO_MODE === 'true';
  const trend = [42, 48, 45, 61, 58, 72, 68, 80, 76, 88, 84, 95];
  const previous = [38, 40, 44, 47, 46, 52, 55, 57, 60, 62, 61, 66];
  const featureIcons = [ShieldCheck, Radio, TrendingUp, Goal, Globe2, Server];
  const topPages = [
    { path: '/', views: '2,184', width: 1 },
    { path: '/pricing', views: '1,206', width: 0.56 },
    { path: '/docs/mcp', views: '864', width: 0.4 },
    { path: '/blog/launch', views: '512', width: 0.24 }
  ];
</script>

<svelte:head>
  <title>Slimlytics · Private web analytics</title>
  <meta
    name="description"
    content="Privacy-first, cookieless web analytics with real-time Spy, multi-site reports, MCP agent setup, and self-hosting. Know what works — skip the noise."
  />
</svelte:head>

<main id="main">
  <div class="mkt-hero-band">
    <section class="mkt-hero">
      <div class="mkt-hero-copy">
        <a class="mkt-chip" href="/docs/mcp"><b>New</b> Set up analytics from Claude Code, Codex, or Hermes</a>
        <h1>Know what works.<br /><em>Skip the noise.</em></h1>
        <p class="mkt-lead">
          Focused traffic intelligence, live visitor activity, and goals — without invasive profiles
          or an overgrown interface. Self-host free, or start a hosted plan when you are ready.
        </p>
        <div class="mkt-hero-actions">
          <a class="primary" href="/register">Create free account <ArrowRight class="mkt-arrow" size={17} aria-hidden="true" /></a>
          <a class="secondary" href="/pricing">View pricing</a>
          {#if demo}
            <a class="secondary" href="/login">Explore demo</a>
          {/if}
        </div>
        <ul class="mkt-hero-pills" aria-label="Key benefits">
          <li><CircleDot size={16} aria-hidden="true" /> Cookieless by default</li>
          <li><Zap size={16} aria-hidden="true" /> Real-time Spy</li>
          <li><Shield size={16} aria-hidden="true" /> Self-hostable</li>
        </ul>
      </div>

      <aside class="mkt-hero-preview" aria-label="Product preview">
        <div class="mkt-preview-card">
          <div class="mkt-preview-top">
            <span class="mkt-live"><i aria-hidden="true"></i>14 online now</span>
            <span>Last 28 days</span>
          </div>
          <div class="mkt-preview-metrics">
            <div><small>Visitors</small><strong>3,421</strong><span>+12.8%</span></div>
            <div><small>Page views</small><strong>8,754</strong><span>+9.4%</span></div>
            <div><small>Bounce</small><strong>38%</strong><span>−3.1 pts</span></div>
          </div>
          <svg class="mkt-preview-chart" viewBox="0 0 300 70" preserveAspectRatio="none" role="img" aria-label="Sample traffic trend rising over 28 days">
            <defs>
              <linearGradient id="mkt-fade" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0" stop-color="#34d399" stop-opacity=".35" />
                <stop offset="1" stop-color="#34d399" stop-opacity="0" />
              </linearGradient>
            </defs>
            <polygon points={`0,70 ${sparklinePoints(trend, 300, 64)} 300,70`} fill="url(#mkt-fade)" />
            <polyline class="line-2" points={sparklinePoints(previous, 300, 64)} />
            <polyline class="line" points={sparklinePoints(trend, 300, 64)} />
          </svg>
          <div class="mkt-preview-split">
            <div>
              <p class="mkt-preview-label">Top pages</p>
              <ul class="mkt-bars">
                {#each topPages as page}
                  <li style={`--w:${page.width}`}><span>{page.path}</span><b>{page.views}</b></li>
                {/each}
              </ul>
            </div>
            <div>
              <p class="mkt-preview-label">Live</p>
              <ul class="mkt-preview-feed">
                <li><Eye size={16} aria-hidden="true" /><span><strong>/pricing</strong><small>Berlin · pageview</small></span></li>
                <li><FileText size={16} aria-hidden="true" /><span><strong>/docs</strong><small>Austin · pageview</small></span></li>
                <li><Users size={16} aria-hidden="true" /><span><strong>signup</strong><small>Tokyo · goal</small></span></li>
              </ul>
            </div>
          </div>
        </div>
        <div class="mkt-float" aria-hidden="true">
          <ShieldCheck size={20} />
          <span><strong>0 cookies set</strong><small>GPC &amp; DNT respected</small></span>
        </div>
      </aside>
    </section>
  </div>

  <section class="mkt-proof" aria-label="At a glance">
    <ul>
      <li><strong>0</strong><span>tracking cookies</span></li>
      <li><strong>1 tag</strong><span>first-party script install</span></li>
      <li><strong>MCP</strong><span>agent-native setup</span></li>
      <li><strong>$0</strong><span>to run it yourself</span></li>
    </ul>
  </section>

  <section id="features" class="mkt-section">
    <div class="mkt-section-head">
      <p class="eyebrow">Product</p>
      <h2>Everything you need. Nothing you do not.</h2>
      <p>Built for teams who want clear traffic intelligence without the surveillance stack.</p>
    </div>
    <div class="feature-grid">
      {#each productFeatures as feature, index}
        {@const Icon = featureIcons[index] ?? Activity}
        <article class="feature-card">
          <span class="feature-icon"><Icon size={22} aria-hidden="true" /></span>
          <h3>{feature.title}</h3>
          <p>{feature.body}</p>
        </article>
      {/each}
    </div>
  </section>

  <section class="mkt-dark" aria-labelledby="agents-title">
    <div class="mkt-split">
      <div>
        <p class="eyebrow">Agent-native</p>
        <h2 id="agents-title">Ask your coding agent to install analytics</h2>
        <p>
          Slimlytics ships a remote MCP server with browser OAuth. Connect Claude Code, Codex, or
          Hermes once, then let your agent create the site, wire first-party routes, and verify
          collection.
        </p>
        <ul class="mkt-checks">
          <li><Check size={18} aria-hidden="true" />No API keys pasted into config — log in through your browser</li>
          <li><Check size={18} aria-hidden="true" />Returns Caddy, Nginx, or Apache proxy routes plus the script tag</li>
          <li><Check size={18} aria-hidden="true" />Query reports and marketing briefs straight from the terminal</li>
        </ul>
        <a class="primary" href="/docs/mcp">Read the agent guide <ArrowRight class="mkt-arrow" size={17} aria-hidden="true" /></a>
      </div>
      <div class="mkt-terminal" role="img" aria-label="Terminal session connecting Claude Code to Slimlytics and installing analytics">
        <div class="mkt-terminal-bar" aria-hidden="true"><i></i><i></i><i></i><span>~/shop — claude</span></div>
        <pre aria-hidden="true"><span class="t-p">$</span> claude mcp add --transport http slimlytics https://slimlytics.com/api/mcp
<span class="t-m">Added HTTP MCP server slimlytics</span>

<span class="t-p">&gt;</span> Set up Slimlytics for https://shop.example.com behind Nginx and verify it.

<span class="t-a">● slimlytics · setup_site</span> <span class="t-m">(domain: shop.example.com, serverType: nginx)</span>
<span class="t-a">● Edit</span> <span class="t-m">deploy/nginx/shop.conf  +14 lines</span>
<span class="t-a">● Edit</span> <span class="t-m">src/app.html  +1 line</span>
<span class="t-a">● Bash</span> <span class="t-m">curl -fsS $scriptTestUrl  →  200</span>
<span class="t-ok">✓ First-party tracker live. First page view recorded.</span></pre>
      </div>
    </div>
  </section>

  <section class="mkt-section mkt-section-alt">
    <div class="mkt-section-head center">
      <p class="eyebrow">How it works</p>
      <h2>Live in three steps</h2>
    </div>
    <ol class="how-grid">
      {#each howItWorks as step}
        <li>
          <span class="how-step" aria-hidden="true">{step.step}</span>
          <h3>{step.title}</h3>
          <p>{step.body}</p>
        </li>
      {/each}
    </ol>
  </section>

  <section class="mkt-section">
    <div class="privacy-strip">
      <div>
        <p class="eyebrow">Privacy</p>
        <h2>Analytics that respect your visitors</h2>
        <p>
          Slimlytics answers site-analytics questions without building cross-site identity profiles.
        </p>
        <a class="secondary" href="/privacy">Read the privacy model <ArrowRight class="mkt-arrow" size={17} aria-hidden="true" /></a>
      </div>
      <ul>
        {#each privacyHighlights as item}
          <li><Shield size={18} aria-hidden="true" />{item}</li>
        {/each}
      </ul>
    </div>
  </section>

  <section class="mkt-section mkt-section-alt" aria-labelledby="pricing-teaser-title">
    <div class="mkt-section-head center">
      <p class="eyebrow">Pricing</p>
      <h2 id="pricing-teaser-title">Simple plans. Full product when you self-host.</h2>
      <p>Start free on your own infrastructure, or pick a hosted tier when you want us to run it.</p>
    </div>
    <PricingCards compact />
    <p class="mkt-center">
      <a class="secondary" href="/pricing">Compare plans in detail</a>
    </p>
  </section>

  <section class="mkt-section mkt-cta">
    <div class="mkt-cta-inner">
      <Bot size={32} aria-hidden="true" />
      <h2>Start measuring what actually matters</h2>
      <p>Create an account, add your site, and see private, useful analytics in minutes.</p>
      <div class="mkt-hero-actions">
        <a class="primary" href="/register">Create free account</a>
        <a class="secondary" href="/docs">Read the docs</a>
      </div>
      <ul class="mkt-cta-stats" aria-label="Product highlights">
        <li><Activity size={16} aria-hidden="true" /> Live Spy stream</li>
        <li><Globe2 size={16} aria-hidden="true" /> Multi-site workspace</li>
        <li><CircleDot size={16} aria-hidden="true" /> Cookieless defaults</li>
      </ul>
    </div>
  </section>
</main>
