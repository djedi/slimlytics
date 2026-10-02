<script lang="ts">
  import '../app.css';
  import '$lib/marketing/marketing.css';
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { applyTheme, type Theme } from '$lib/ui';
  let { children } = $props();
  onMount(() => {
    const theme = (localStorage.getItem('slimlytics_theme') ?? 'system') as Theme;
    applyTheme(theme);
  });
</script>
<svelte:head>
  <title>Slimlytics · Private web analytics</title>
  <!-- Social previews need absolute URLs; the origin comes from ORIGIN when server-rendered. -->
  <meta property="og:site_name" content="Slimlytics" />
  <meta property="og:type" content="website" />
  <meta property="og:url" content={page.url.origin + page.url.pathname} />
  <meta property="og:title" content="Slimlytics · Privacy-first web analytics" />
  <meta
    property="og:description"
    content="Know what works. Skip the noise. Privacy-first, open-source web analytics with real-time insights and AI-agent setup. Self-host or use our cloud."
  />
  <meta property="og:image" content={`${page.url.origin}/og-image.jpg`} />
  <meta property="og:image:width" content="1200" />
  <meta property="og:image:height" content="630" />
  <meta
    property="og:image:alt"
    content="Slimlytics: Know what works. Skip the noise. Privacy-first, open-source web analytics."
  />
  <meta name="twitter:card" content="summary_large_image" />
  <meta name="twitter:image" content={`${page.url.origin}/og-image.jpg`} />
</svelte:head>
<a class="skip-link" href="#main">Skip to content</a>
{@render children()}
