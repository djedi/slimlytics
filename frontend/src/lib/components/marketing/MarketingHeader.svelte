<script lang="ts">
  import { BarChart3, Menu, Monitor, Moon, Sun, X } from '@lucide/svelte';

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
      <span class="brand-mark"><BarChart3 size={20} aria-hidden="true" /></span>
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
