<script lang="ts">
  // DRAFT. Every "[TODO: …]" below is a fact only Dustin can supply (business address, effective
  // date, price-change notice, deletion process and timing). Do not merge until all are resolved
  // and the text has had a legal review.
  const updated = '[TODO: effective date]';
</script>

<svelte:head>
  <title>Terms of Service · Slimlytics</title>
  <meta
    name="description"
    content="The terms for using the hosted Slimlytics analytics service at slimlytics.com: accounts, plans and billing, your data, acceptable use, and liability."
  />
</svelte:head>

<main id="main" class="mkt-page">
  <header class="mkt-page-head">
    <p class="eyebrow">Legal</p>
    <h1>Terms of Service</h1>
    <p class="mkt-lead">
      Plain-language terms for the hosted Slimlytics service. Self-hosting the open-source software
      is governed by its license, not by these terms.
    </p>
    <p class="muted">Last updated: {updated}</p>
  </header>

  <section class="mkt-section">
    <div class="mkt-prose">
      <h2>1. Who we are and what these terms cover</h2>
      <p>
        These terms are an agreement between you and Red Seam, Inc., [TODO: business address] (“we”,
        “us”) for your use of the hosted Slimlytics service at slimlytics.com, including the
        dashboard, API, CLI access, and MCP server (the “Service”). By creating an account or using
        the Service you agree to them. If you use the Service for an organization, you accept these
        terms on its behalf and confirm you are authorized to do so.
      </p>
      <p>
        The Slimlytics source code is open source under the GNU Affero General Public License v3.0
        (the browser tracker and CLI under the MIT License). Nothing in these terms limits the rights
        those licenses give you. If you run Slimlytics on your own servers, these terms do not apply.
      </p>

      <h2>2. Your account</h2>
      <ul>
        <li>Give us a working email address and keep your sign-in credentials, passkeys, and API tokens secure.</li>
        <li>You are responsible for activity under your account, including actions taken by AI agents, scripts, or tokens you authorize.</li>
        <li>Tell us promptly if you believe your account has been compromised. You can revoke sessions and API tokens from your account settings.</li>
      </ul>

      <h2>3. Plans, limits, and billing</h2>
      <ul>
        <li>
          Plans, prices, and limits are listed on the <a href="/pricing">pricing page</a>. Every
          feature is included on every plan; plans differ in sites, daily page views, and support.
        </li>
        <li>
          Site limits are enforced. Daily page views are a soft limit: collection continues if you go
          over, and the dashboard asks you to upgrade.
        </li>
        <li>
          Paid plans are billed in advance, monthly or annually, through our payment processor,
          Stripe, and renew automatically until cancelled. You can upgrade, downgrade, or cancel from
          your account at any time. Cancellation takes effect at the end of the current billing
          period, and your plan stays active until then.
        </li>
        <li>Payments are non-refundable, including for partial billing periods and unused page views.</li>
        <li>
          Prices may change. A price change applies to new subscriptions; we will give existing
          subscribers [TODO: notice period, e.g. 30 days’] notice by email before their price changes.
        </li>
        <li>Prices exclude any taxes that apply unless stated otherwise.</li>
      </ul>

      <h2>4. Your data and your visitors’ data</h2>
      <ul>
        <li>
          You own the analytics data collected from your sites (“Site Data”). We use it only to
          provide, secure, and support the Service for you, and we never sell it or use it to build
          advertising or cross-site profiles.
        </li>
        <li>
          For Site Data you are the controller and we act as your processor. You are responsible for
          your own site’s privacy notice and for obtaining any consent your visitors’ jurisdiction
          requires. The tracker can stay disabled until consent is granted; see the
          <a href="/privacy">privacy model</a>.
        </li>
        <li>
          Do not send personal data such as names, email addresses, or account numbers in URLs,
          page titles, or custom event properties. The collector strips common sensitive query
          parameters, but you control what your site sends.
        </li>
        <li>
          You can export Site Data (CSV or API) at any time, and deleting a site deletes its data.
          Raw events are kept for each site’s configured retention period (365 days by default).
        </li>
        <li>
          How we handle data, where it is hosted, and which providers we use are described in
          <a href="/privacy#data-processing">Data processing and hosting</a>.
        </li>
      </ul>

      <h2>5. Acceptable use</h2>
      <p>You agree not to:</p>
      <ul>
        <li>track websites or apps you do not own or have permission to measure;</li>
        <li>use the Service to collect sensitive personal data, or in violation of privacy, consumer protection, or other laws;</li>
        <li>send fake or automated traffic, overload the collection endpoints or API, or try to get around plan limits;</li>
        <li>access other accounts’ data, probe or attack the Service, or test its security other than through our <a href="https://github.com/djedi/slimlytics/blob/main/SECURITY.md">security policy</a>;</li>
        <li>resell the hosted Service without our written permission (self-hosting the open-source code is always allowed under its license).</li>
      </ul>

      <h2>6. Availability and changes</h2>
      <p>
        We work to keep the Service fast and available, but we do not guarantee uninterrupted
        operation or a specific uptime unless we agree to one in writing. We may add, change, or
        remove features. If we discontinue the hosted Service, we will give you reasonable notice
        so you can export your data or move to a self-hosted install.
      </p>

      <h2>7. Suspension and termination</h2>
      <p>
        You can stop using the Service at any time. To delete your account, email
        <a href="mailto:dustin@davis.im">dustin@davis.im</a> from the address on the account [TODO:
        confirm this process]. We may suspend or close accounts that break these terms, create risk for the Service
        or other customers, or have unpaid invoices. After an account is closed we delete its Site
        Data within [TODO: deletion timeline, e.g. 30 days]; copies in database backups expire as described in our <a href="/privacy#data-processing">data processing note</a>.
      </p>

      <h2>8. Disclaimers</h2>
      <p>
        The Service is provided “as is” and “as available”. To the extent the law allows, we disclaim
        all warranties, express or implied, including merchantability, fitness for a particular
        purpose, and non-infringement. Analytics are estimates: ad blockers, privacy settings, bot
        filtering, and network conditions all affect what is counted.
      </p>

      <h2>9. Limitation of liability</h2>
      <p>
        To the extent the law allows, we are not liable for indirect, incidental, special,
        consequential, or punitive damages, or for lost profits, revenue, or data. Our total
        liability for any claim relating to the Service is limited to the amount you paid us for the
        Service in the 12 months before the claim arose.
      </p>

      <h2>10. Changes to these terms</h2>
      <p>
        We may update these terms. For material changes we will give notice by email or in the
        dashboard before they take effect. Continuing to use the Service after that means you accept
        the updated terms.
      </p>

      <h2>11. Governing law</h2>
      <p>
        These terms are governed by the laws of the State of Utah, USA, without regard to its
        conflict-of-law rules. Any dispute will be resolved in the state or federal courts located
        in Utah, and both parties consent to their jurisdiction.
      </p>

      <h2>12. Contact</h2>
      <p>
        Questions about these terms: <a href="mailto:dustin@davis.im">dustin@davis.im</a>. More ways to reach us are on the
        <a href="/about#contact">About page</a>.
      </p>
    </div>
  </section>
</main>
