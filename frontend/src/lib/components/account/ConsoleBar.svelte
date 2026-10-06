<script lang="ts">
  import '$lib/console.css';

  let { current, isAdmin = false }: { current: 'account' | 'admin'; isAdmin?: boolean } = $props();
</script>

<header class="console-bar">
  <a class="brand" href="/app" aria-label="Slimlytics dashboard">
    <img class="mascot-logo" src="/images/mascot/meerkat-logo.webp" width="38" height="38" alt="" />Slimlytics
  </a>
  <nav aria-label="Account">
    <a href="/app">Dashboard</a>
    <a href="/account" aria-current={current === 'account' ? 'page' : undefined}>Account security</a>
    {#if isAdmin}
      <a href="/admin" aria-current={current === 'admin' ? 'page' : undefined}>Admin</a>
    {/if}
  </nav>
</header>
