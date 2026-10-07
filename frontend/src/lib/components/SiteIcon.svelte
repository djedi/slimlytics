<script lang="ts">
  import { siteIconUrl, type Site } from '$lib/api';
  import { avatarHue } from '$lib/ui';

  let {
    site,
    size = 34,
    apiBase = '/api'
  }: {
    site: Pick<Site, 'id' | 'name' | 'domain' | 'iconMode' | 'iconBackground' | 'iconBackgroundEnd' | 'iconForeground' | 'iconUpdatedAt'>;
    size?: number;
    /** Where the API lives (PUBLIC_API_BASE_URL); the favicon is served from it. */
    apiBase?: string;
  } = $props();

  // A favicon that fails to load falls back to initials rather than a broken image.
  let failed = $state(false);
  $effect(() => {
    void site.iconUpdatedAt;
    failed = false;
  });

  const favicon = $derived(site.iconMode === 'favicon' && !!site.iconUpdatedAt && !failed);
  const background = $derived.by(() => {
    const start = site.iconBackground;
    const end = site.iconBackgroundEnd;
    if (start) return end ? `linear-gradient(140deg, ${start}, ${end})` : start;
    // Favicons are drawn for light backgrounds; initials get the automatic per-domain hue.
    if (favicon) return '#ffffff';
    const hue = avatarHue(site.domain);
    return `linear-gradient(140deg, hsl(${hue} 65% 52%), hsl(${hue + 40} 60% 40%))`;
  });
</script>

<span
  class="site-icon"
  aria-hidden="true"
  style:width={`${size}px`}
  style:height={`${size}px`}
  style:border-radius={`${Math.round(size * 0.27)}px`}
  style:font-size={`${Math.round(size * 0.32)}px`}
  style:background
  style:color={site.iconForeground || '#ffffff'}
>
  {#if favicon}
    <img src={siteIconUrl(apiBase, site)} alt="" onerror={() => (failed = true)} />
  {:else}
    {site.name.slice(0, 2).toUpperCase()}
  {/if}
</span>

<style>
  .site-icon {
    display: grid;
    place-items: center;
    flex: none;
    overflow: hidden;
    font-weight: 800;
    letter-spacing: 0.02em;
  }
  img {
    width: 66%;
    height: 66%;
    object-fit: contain;
  }
</style>
