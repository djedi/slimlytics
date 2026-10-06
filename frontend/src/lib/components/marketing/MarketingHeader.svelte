<script lang="ts">
  import { Menu, Monitor, Moon, Sun, X } from '@lucide/svelte';

  const themes = [
    { value: 'light', label: 'Light', icon: Sun },
    { value: 'dark', label: 'Dark', icon: Moon },
    { value: 'system', label: 'System', icon: Monitor }
  ];

  // Marketing pages render without client-side Svelte. The inline script in app.html opens the
  // mobile menu, swaps the data-auth links for signed-in visitors, and applies the theme radios.
</script>

<header class="mkt-header">
  <div class="mkt-header-inner">
    <a class="brand" href="/" aria-label="Slimlytics home">
      <img class="mkt-mascot-logo" src="/images/mascot/meerkat-logo.webp" width="38" height="38" alt="" />
      <strong>Slimlytics</strong>
    </a>

    <nav id="mkt-nav" class="mkt-nav" data-mkt-nav aria-label="Marketing">
      <a href="/#features">Features</a>
      <a href="/docs/mcp">Agents</a>
      <a href="/pricing">Pricing</a>
      <a href="/privacy">Privacy</a>
      <a href="/docs">Docs</a>
      <div class="mkt-nav-actions">
        <a class="primary" href="/app" data-auth="in" hidden>Open dashboard</a>
        <a class="mkt-link-quiet" href="/login" data-auth="out">Sign in</a>
        <a class="primary" href="/register" data-auth="out">Get started</a>
      </div>
    </nav>

    <a class="mkt-github" href="https://github.com/djedi/slimlytics" aria-label="Slimlytics on GitHub" title="View source on GitHub">
      <svg width="21" height="21" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
        <path d="M12 .297C5.37.297 0 5.67 0 12.297c0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.043-1.61-4.043-1.61-.546-1.387-1.333-1.756-1.333-1.756-1.09-.745.083-.729.083-.729 1.205.084 1.838 1.237 1.838 1.237 1.07 1.835 2.809 1.305 3.495.998.108-.776.418-1.305.762-1.605-2.665-.3-5.466-1.333-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23a11.5 11.5 0 0 1 3-.405c1.02.005 2.045.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12" />
      </svg>
    </a>

    <fieldset class="mkt-theme">
      <legend class="sr-only">Color theme</legend>
      {#each themes as theme}
        {@const Icon = theme.icon}
        <label title={theme.label}>
          <input type="radio" name="mkt-theme" value={theme.value} checked={theme.value === 'system'} />
          <Icon size={16} aria-hidden="true" />
          <span class="sr-only">{theme.label}</span>
        </label>
      {/each}
    </fieldset>

    <button
      class="icon-button mkt-menu"
      type="button"
      data-mkt-menu
      aria-label="Open menu"
      aria-expanded="false"
      aria-controls="mkt-nav"
    >
      <Menu class="mkt-menu-open" size={20} aria-hidden="true" />
      <X class="mkt-menu-close" size={20} aria-hidden="true" />
    </button>
  </div>
</header>
<button class="mkt-scrim" type="button" data-mkt-scrim aria-label="Close menu" hidden></button>
