<script lang="ts">
  import { Shield } from '@lucide/svelte';
  import { privacyHighlights } from '$lib/marketing/features';
</script>

<svelte:head>
  <title>Privacy · Slimlytics</title>
  <meta
    name="description"
    content="How Slimlytics approaches privacy: cookieless defaults, no form capture, site-scoped IDs, and owner-controlled retention."
  />
</svelte:head>

<main id="main" class="mkt-page">
  <header class="mkt-page-head">
    <p class="eyebrow">Privacy</p>
    <h1>Analytics without the surveillance model</h1>
    <p class="mkt-lead">
      Slimlytics is designed to answer site-analytics questions without building cross-site identity
      profiles or capturing more than you need.
    </p>
  </header>

  <section class="mkt-section">
    <div class="privacy-grid">
      <article class="feature-card">
        <Shield size={22} aria-hidden="true" />
        <h2>Cookieless by default</h2>
        <p>
          The browser tracker does not set tracking cookies. Collection can stay disabled until your
          site grants consent when that is legally or contractually required.
        </p>
      </article>
      <article class="feature-card">
        <Shield size={22} aria-hidden="true" />
        <h2>What we deliberately skip</h2>
        <p>
          No cross-site tracking, no device fingerprinting for identity, no form-field capture, and
          no session replay. Those are non-goals — not delayed features we quietly enable later.
        </p>
      </article>
      <article class="feature-card">
        <Shield size={22} aria-hidden="true" />
        <h2>Sensitive data redaction</h2>
        <p>
          Common secret and identity query parameters (tokens, emails, session keys, and similar)
          are stripped before URLs are stored. Prefer not putting personal data in URLs at all.
        </p>
      </article>
      <article class="feature-card">
        <Shield size={22} aria-hidden="true" />
        <h2>Site-scoped visitor IDs</h2>
        <p>
          Visitor identifiers are derived from privacy-reduced inputs with a rotating server secret
          and stay scoped to each site. Raw IPs are used only transiently for abuse controls and
          coarse location when enabled.
        </p>
      </article>
    </div>
  </section>

  <section class="mkt-section mkt-section-alt">
    <div class="mkt-section-head">
      <p class="eyebrow">Defaults</p>
      <h2>Built-in privacy highlights</h2>
    </div>
    <ul class="privacy-bullets">
      {#each privacyHighlights as item}
        <li><Shield size={16} aria-hidden="true" />{item}</li>
      {/each}
      <li><Shield size={16} aria-hidden="true" />Do Not Track and Global Privacy Control are respected</li>
      <li><Shield size={16} aria-hidden="true" />Site owners control retention, export, and deletion</li>
    </ul>
  </section>

  <section class="mkt-section">
    <div class="mkt-prose">
      <h2>Your control as a site owner</h2>
      <p>
        Configure each site’s retention period (365 days by default), export site data as CSV or
        through the API, and delete a site together with all of its data. When you self-host, the
        data never leaves the infrastructure you choose.
      </p>
      <h2>Consent integration</h2>
      <p>
        Initialize the tracker with collection disabled when consent is required. Call the tracker
        consent API only after the visitor grants analytics consent. Revocation stops new events
        immediately; deleting prior data is a separate server-side operation.
      </p>
    </div>
  </section>

  <section id="data-processing" class="mkt-section mkt-section-alt" aria-labelledby="processing-title">
    <div class="mkt-prose">
      <!-- DRAFT: the "[TODO: …]" is a fact only Dustin can supply. Do not merge until resolved. -->
      <h2 id="processing-title">Data processing and hosting</h2>
      <p>
        This section covers the hosted service at slimlytics.com. Self-hosted installs keep all
        data on the operator’s own servers.
      </p>
      <h3>Roles</h3>
      <p>
        For analytics collected from your sites, you are the controller and Slimlytics acts as your
        processor: we process that data only to provide the service to you. For your own account
        details (email, sign-in, and billing), Slimlytics is the controller.
      </p>
      <h3>What the hosted service stores</h3>
      <ul>
        <li><strong>Analytics events:</strong> page URLs (with sensitive parameters removed) and titles, referrers, campaign tags, coarse location (country, continent, and, without Global Privacy Control, region and city), browser, OS, and device type, custom events and their properties you send, and site-scoped visitor and session identifiers derived with rotating secrets.</li>
        <li><strong>Not stored:</strong> raw IP addresses, cookies, fingerprints, form values, coordinates, or session recordings.</li>
        <li><strong>Account data:</strong> your email, a password hash and passkeys, active sessions with their browser user agent, API tokens, and your Stripe customer and subscription references.</li>
      </ul>
      <h3>Where it is hosted</h3>
      <p>
        The application and its PostgreSQL database run on a virtual private server from HostRush
        (ServerCheap) in the United States. Visitor location lookups use a local DB-IP database on that server, so
        visitor IP addresses are never sent to a geolocation provider.
      </p>
      <h3>Service providers</h3>
      <ul>
        <li>HostRush (ServerCheap), United States: the server that runs the application and database.</li>
        <li>Stripe: payment processing for paid plans. Card details go to Stripe, never to us.</li>
        <li>Google: only if you connect Search Console, using read-only access you can disconnect at any time.</li>
      </ul>
      <h3>Retention and deletion</h3>
      <p>
        Raw events are deleted after each site’s retention period (365 days by default). Deleting a
        site deletes its analytics data. Database backups are kept for [TODO: backup retention
        period] before they expire.
      </p>
      <h3>Data processing agreement</h3>
      <p>
        Need a signed data processing agreement (DPA), for example for GDPR? Email
        <a href="mailto:dustin@davis.im">dustin@davis.im</a> and we will send one. See also our
        <a href="/terms">Terms of Service</a>.
      </p>
    </div>
  </section>

  <section class="mkt-section mkt-cta">
    <div class="mkt-cta-inner">
      <h2>Measure without the guilt hangover</h2>
      <p>Create an account and see privacy-first analytics on your own terms.</p>
      <div class="mkt-hero-actions">
        <a class="primary" href="/register">Create free account</a>
        <a class="secondary" href="/docs">Documentation</a>
      </div>
    </div>
  </section>
</main>
